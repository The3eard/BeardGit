//! Task manager — spawns, tracks, and cancels background CLI tasks.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::sink::TaskEventSink;
use crate::types::{OutputLine, Stream, TaskError, TaskHandle, TaskId, TaskInfo, TaskStatus};

/// Manages background tasks, their lifecycle, output, and cancellation.
pub struct TaskManager {
    pub(crate) tasks: Mutex<Vec<TaskHandle>>,
    pub(crate) next_id: AtomicU64,
    pub(crate) cancellation_tokens: Mutex<HashMap<TaskId, CancellationToken>>,
    pub(crate) sink: Arc<dyn TaskEventSink>,
}

impl TaskManager {
    /// Create a new task manager that emits events through the given sink.
    pub fn new(sink: Arc<dyn TaskEventSink>) -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
            next_id: AtomicU64::new(1),
            cancellation_tokens: Mutex::new(HashMap::new()),
            sink,
        }
    }

    /// Snapshot of all tasks for initial frontend load.
    pub async fn list_tasks(&self) -> Vec<TaskInfo> {
        let tasks = self.tasks.lock().await;
        tasks.iter().map(|t| t.to_info()).collect()
    }

    /// Full output buffer for a specific task.
    pub async fn get_output(&self, task_id: TaskId) -> Option<Vec<OutputLine>> {
        let tasks = self.tasks.lock().await;
        tasks
            .iter()
            .find(|t| t.id == task_id)
            .map(|t| t.output.clone())
    }

    /// Spawn a CLI command as a background task. Returns `TaskId` immediately.
    ///
    /// The spawned tokio task holds an `Arc<Self>` so the manager stays alive
    /// for the duration of the subprocess.
    pub async fn spawn(
        self: &Arc<Self>,
        label: String,
        command: &str,
        args: &[&str],
        cwd: &Path,
        cancellable: bool,
    ) -> TaskId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Build displayable command string from program + args.
        let command_str = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        let started_at_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as u64);

        let handle = TaskHandle {
            id,
            label: label.clone(),
            status: TaskStatus::Running,
            cancellable,
            started_at: Some(Instant::now()),
            finished_at: None,
            output: Vec::new(),
            command: command_str,
            started_at_ms,
            exit_code: None,
        };

        {
            let mut tasks = self.tasks.lock().await;
            tasks.push(handle);
        }

        let token = if cancellable {
            let t = CancellationToken::new();
            let mut tokens = self.cancellation_tokens.lock().await;
            tokens.insert(id, t.clone());
            Some(t)
        } else {
            None
        };

        // Notify sink that the task has started.
        let info = {
            let tasks = self.tasks.lock().await;
            tasks.iter().find(|t| t.id == id).unwrap().to_info()
        };
        self.sink.on_task_started(info).await;

        // Build the child process with piped stdout and stderr.
        let mut cmd = Command::new(command);
        cmd.args(args)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Suppress the console window on Windows.
        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.finish_task(
                    id,
                    TaskStatus::Failed {
                        error: e.to_string(),
                    },
                )
                .await;
                return id;
            }
        };

        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        let manager = Arc::clone(self);

        tokio::spawn(async move {
            // Read stdout and stderr concurrently.
            let mut stdout_lines = BufReader::new(stdout).lines();
            let mut stderr_lines = BufReader::new(stderr).lines();

            // Collect all stderr text for use in the Failed status message.
            let mut stderr_text = String::new();

            // We drive both streams concurrently with `tokio::select!` so
            // neither blocks the other.  When a stream is exhausted we set a
            // flag and stop selecting on it.
            let mut stdout_done = false;
            let mut stderr_done = false;

            loop {
                // Check cancellation if applicable.
                if let Some(ref t) = token
                    && t.is_cancelled()
                {
                    let _ = child.kill().await;
                    {
                        let mut tasks = manager.tasks.lock().await;
                        if let Some(handle) = tasks.iter_mut().find(|t| t.id == id) {
                            handle.exit_code = Some(-1);
                        }
                    }
                    manager.finish_task(id, TaskStatus::Cancelled).await;
                    return;
                }

                if stdout_done && stderr_done {
                    break;
                }

                tokio::select! {
                    // biased ensures deterministic drain order during tests.
                    biased;

                    // Cancellation branch — only active when cancellable.
                    _ = async {
                        if let Some(ref t) = token {
                            t.cancelled().await
                        } else {
                            // Never resolves when not cancellable.
                            std::future::pending::<()>().await
                        }
                    } => {
                        let _ = child.kill().await;
                        {
                            let mut tasks = manager.tasks.lock().await;
                            if let Some(handle) = tasks.iter_mut().find(|t| t.id == id) {
                                handle.exit_code = Some(-1);
                            }
                        }
                        manager.finish_task(id, TaskStatus::Cancelled).await;
                        return;
                    }

                    line = stdout_lines.next_line(), if !stdout_done => {
                        match line {
                            Ok(Some(text)) => {
                                let output_line = OutputLine {
                                    stream: Stream::Stdout,
                                    text: text.clone(),
                                    timestamp: Instant::now(),
                                };
                                manager.append_output(id, output_line.clone()).await;
                                manager.sink.on_task_output(id, output_line).await;
                            }
                            _ => stdout_done = true,
                        }
                    }

                    line = stderr_lines.next_line(), if !stderr_done => {
                        match line {
                            Ok(Some(text)) => {
                                let output_line = OutputLine {
                                    stream: Stream::Stderr,
                                    text: text.clone(),
                                    timestamp: Instant::now(),
                                };
                                if !stderr_text.is_empty() {
                                    stderr_text.push('\n');
                                }
                                stderr_text.push_str(&text);
                                manager.append_output(id, output_line.clone()).await;
                                manager.sink.on_task_output(id, output_line).await;
                            }
                            _ => stderr_done = true,
                        }
                    }
                }
            }

            // Wait for the child to exit and decide final status.
            let exit_status = match child.wait().await {
                Ok(s) => s,
                Err(e) => {
                    manager
                        .finish_task(
                            id,
                            TaskStatus::Failed {
                                error: e.to_string(),
                            },
                        )
                        .await;
                    return;
                }
            };

            let code = exit_status.code();

            let final_status = if exit_status.success() {
                TaskStatus::Completed
            } else {
                TaskStatus::Failed { error: stderr_text }
            };

            // Store exit code on the handle before finishing.
            {
                let mut tasks = manager.tasks.lock().await;
                if let Some(handle) = tasks.iter_mut().find(|t| t.id == id) {
                    handle.exit_code = code;
                }
            }

            manager.finish_task(id, final_status).await;
        });

        id
    }

    /// Cancel a running task. Kills the child process.
    pub async fn cancel(&self, task_id: TaskId) -> Result<(), TaskError> {
        // Check task exists and is in a cancellable, running state.
        {
            let tasks = self.tasks.lock().await;
            let handle = tasks
                .iter()
                .find(|t| t.id == task_id)
                .ok_or(TaskError::NotFound(task_id))?;

            if !handle.cancellable {
                return Err(TaskError::NotCancellable(task_id));
            }

            if !matches!(handle.status, TaskStatus::Running) {
                return Err(TaskError::NotRunning(task_id));
            }
        }

        // Trigger the cancellation token.  The spawned task will detect this
        // in the `tokio::select!` loop, kill the process, and call
        // `finish_task`.
        let mut tokens = self.cancellation_tokens.lock().await;
        if let Some(token) = tokens.remove(&task_id) {
            token.cancel();
        }

        Ok(())
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Append a single output line to the task's buffer.
    async fn append_output(&self, task_id: TaskId, line: OutputLine) {
        let mut tasks = self.tasks.lock().await;
        if let Some(handle) = tasks.iter_mut().find(|t| t.id == task_id) {
            handle.output.push(line);
        }
    }

    /// Transition a task to its terminal state and fire the appropriate sink event.
    async fn finish_task(&self, task_id: TaskId, status: TaskStatus) {
        // Remove the cancellation token if still present.
        {
            let mut tokens = self.cancellation_tokens.lock().await;
            tokens.remove(&task_id);
        }

        let info = {
            let mut tasks = self.tasks.lock().await;
            if let Some(handle) = tasks.iter_mut().find(|t| t.id == task_id) {
                handle.status = status.clone();
                handle.finished_at = Some(Instant::now());
                handle.to_info()
            } else {
                return;
            }
        };

        match status {
            TaskStatus::Completed => self.sink.on_task_completed(info).await,
            TaskStatus::Failed { .. } => self.sink.on_task_failed(info).await,
            TaskStatus::Cancelled => self.sink.on_task_cancelled(info).await,
            _ => {}
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::TaskEventSink;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;
    use tokio::time::{Duration, sleep};

    /// All events recorded by the mock sink.
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    enum TaskEvent {
        Started(TaskInfo),
        Output { task_id: TaskId, line: OutputLine },
        Completed(TaskInfo),
        Failed(TaskInfo),
        Cancelled(TaskInfo),
    }

    struct MockEventSink {
        events: Arc<TokioMutex<Vec<TaskEvent>>>,
    }

    impl MockEventSink {
        fn new() -> (Self, Arc<TokioMutex<Vec<TaskEvent>>>) {
            let events = Arc::new(TokioMutex::new(Vec::new()));
            (
                Self {
                    events: Arc::clone(&events),
                },
                events,
            )
        }
    }

    #[async_trait]
    impl TaskEventSink for MockEventSink {
        async fn on_task_started(&self, info: TaskInfo) {
            self.events.lock().await.push(TaskEvent::Started(info));
        }

        async fn on_task_output(&self, task_id: TaskId, line: OutputLine) {
            self.events
                .lock()
                .await
                .push(TaskEvent::Output { task_id, line });
        }

        async fn on_task_completed(&self, info: TaskInfo) {
            self.events.lock().await.push(TaskEvent::Completed(info));
        }

        async fn on_task_failed(&self, info: TaskInfo) {
            self.events.lock().await.push(TaskEvent::Failed(info));
        }

        async fn on_task_cancelled(&self, info: TaskInfo) {
            self.events.lock().await.push(TaskEvent::Cancelled(info));
        }
    }

    /// Convenience constructor.
    fn new_manager() -> (Arc<TaskManager>, Arc<TokioMutex<Vec<TaskEvent>>>) {
        let (sink, events) = MockEventSink::new();
        let manager = Arc::new(TaskManager::new(Arc::new(sink)));
        (manager, events)
    }

    /// Poll until the events list satisfies `pred`, or timeout after ~2 s.
    async fn wait_for<F>(events: &Arc<TokioMutex<Vec<TaskEvent>>>, pred: F)
    where
        F: Fn(&[TaskEvent]) -> bool,
    {
        for _ in 0..200 {
            {
                let ev = events.lock().await;
                if pred(&ev) {
                    return;
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!("wait_for: condition not met within timeout");
    }

    // ── 1 ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_spawn_and_complete() {
        let (manager, events) = new_manager();
        let cwd = std::env::temp_dir();

        let _id = manager
            .spawn(
                "echo hello".into(),
                "sh",
                &["-c", "echo hello"],
                &cwd,
                false,
            )
            .await;

        // Wait for Completed event.
        wait_for(&events, |ev| {
            ev.iter().any(|e| matches!(e, TaskEvent::Completed(_)))
        })
        .await;

        let ev = events.lock().await;

        // Must have a Started event.
        assert!(ev.iter().any(|e| matches!(e, TaskEvent::Started(_))));

        // Must have an Output event with "hello".
        assert!(ev.iter().any(|e| matches!(
            e,
            TaskEvent::Output { line, .. } if line.text == "hello"
        )));

        // Must have a Completed event (not Failed).
        assert!(ev.iter().any(|e| matches!(e, TaskEvent::Completed(_))));
        assert!(!ev.iter().any(|e| matches!(e, TaskEvent::Failed(_))));
    }

    // ── 2 ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_spawn_failure() {
        let (manager, events) = new_manager();
        let cwd = std::env::temp_dir();

        manager
            .spawn(
                "fail".into(),
                "sh",
                &["-c", "echo err >&2; exit 1"],
                &cwd,
                false,
            )
            .await;

        wait_for(&events, |ev| {
            ev.iter().any(|e| matches!(e, TaskEvent::Failed(_)))
        })
        .await;

        let ev = events.lock().await;
        assert!(ev.iter().any(|e| matches!(e, TaskEvent::Started(_))));
        assert!(ev.iter().any(|e| matches!(e, TaskEvent::Failed(_))));
        assert!(!ev.iter().any(|e| matches!(e, TaskEvent::Completed(_))));
    }

    // ── 3 ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_cancel() {
        let (manager, events) = new_manager();
        let cwd = std::env::temp_dir();

        let id = manager
            .spawn("sleep".into(), "sleep", &["60"], &cwd, true)
            .await;

        // Give the process time to start before cancelling.
        sleep(Duration::from_millis(200)).await;

        manager.cancel(id).await.expect("cancel should succeed");

        wait_for(&events, |ev| {
            ev.iter().any(|e| matches!(e, TaskEvent::Cancelled(_)))
        })
        .await;

        let ev = events.lock().await;
        assert!(ev.iter().any(|e| matches!(e, TaskEvent::Cancelled(_))));
        assert!(!ev.iter().any(|e| matches!(e, TaskEvent::Completed(_))));
    }

    // ── 4 ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_cancel_non_cancellable() {
        let (manager, _events) = new_manager();
        let cwd = std::env::temp_dir();

        let id = manager
            .spawn("sleep".into(), "sleep", &["60"], &cwd, false)
            .await;

        let result = manager.cancel(id).await;
        assert!(
            matches!(result, Err(TaskError::NotCancellable(_))),
            "expected NotCancellable, got {result:?}"
        );

        // Cleanup: task is still running; we just leave it for the test runtime
        // to reap — the process is killed when the manager drops.
    }

    // ── 5 ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_tasks() {
        let (manager, events) = new_manager();
        let cwd = std::env::temp_dir();

        manager
            .spawn("task-a".into(), "sh", &["-c", "echo a"], &cwd, false)
            .await;
        manager
            .spawn("task-b".into(), "sh", &["-c", "echo b"], &cwd, false)
            .await;

        // Wait for both to complete.
        wait_for(&events, |ev| {
            ev.iter()
                .filter(|e| matches!(e, TaskEvent::Completed(_)))
                .count()
                >= 2
        })
        .await;

        let list = manager.list_tasks().await;
        assert_eq!(list.len(), 2);
        let labels: Vec<&str> = list.iter().map(|t| t.label.as_str()).collect();
        assert!(labels.contains(&"task-a"));
        assert!(labels.contains(&"task-b"));
    }

    // ── 6 ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_output() {
        let (manager, events) = new_manager();
        let cwd = std::env::temp_dir();

        let id = manager
            .spawn(
                "three lines".into(),
                "sh",
                &["-c", "echo line1; echo line2; echo line3"],
                &cwd,
                false,
            )
            .await;

        wait_for(&events, |ev| {
            ev.iter().any(|e| matches!(e, TaskEvent::Completed(_)))
        })
        .await;

        let output = manager.get_output(id).await.expect("output present");
        let texts: Vec<&str> = output.iter().map(|l| l.text.as_str()).collect();
        assert!(texts.contains(&"line1"), "missing line1: {texts:?}");
        assert!(texts.contains(&"line2"), "missing line2: {texts:?}");
        assert!(texts.contains(&"line3"), "missing line3: {texts:?}");
    }

    // ── 7 ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_stderr_captured() {
        let (manager, events) = new_manager();
        let cwd = std::env::temp_dir();

        let id = manager
            .spawn(
                "stderr test".into(),
                "sh",
                &["-c", "echo error_msg >&2"],
                &cwd,
                false,
            )
            .await;

        // sh exits 0, so this completes (not fails).
        wait_for(&events, |ev| {
            ev.iter()
                .any(|e| matches!(e, TaskEvent::Completed(_) | TaskEvent::Failed(_)))
        })
        .await;

        let output = manager.get_output(id).await.expect("output present");
        let stderr_texts: Vec<&str> = output
            .iter()
            .filter(|l| matches!(l.stream, Stream::Stderr))
            .map(|l| l.text.as_str())
            .collect();

        assert!(
            stderr_texts.contains(&"error_msg"),
            "stderr not captured: {stderr_texts:?}"
        );

        // Also verify the sink received an Output event for it.
        let ev = events.lock().await;
        assert!(ev.iter().any(|e| matches!(
            e,
            TaskEvent::Output { line, .. } if line.text == "error_msg"
        )));
    }
}
