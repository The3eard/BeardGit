//! Task manager — spawns, tracks, and cancels background CLI tasks.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::sink::{NoopTaskEmitter, TaskEmitter, TaskEventSink};
use crate::types::{
    OutputLine, Stream, TaskError, TaskHandle, TaskId, TaskInfo, TaskKind, TaskStatus,
};

/// Minimum gap between two progress emissions for the same task.
///
/// 200 ms ⇒ max 5 emissions / second / task, matching the spec's event-storm
/// mitigation budget. Lifecycle transitions (`start`, `complete`, `fail`,
/// `cancel`) bypass the throttle so terminal states never get dropped.
const PROGRESS_THROTTLE: Duration = Duration::from_millis(200);

/// Options accepted by [`TaskManager::spawn_with_options`].
pub struct SpawnOptions<'a> {
    pub label: String,
    pub command: &'a str,
    pub args: &'a [&'a str],
    pub cwd: &'a std::path::Path,
    pub cancellable: bool,
    pub kind: TaskKind,
    /// Optional stdin payload — written to the child's stdin then closed.
    /// Used by AI providers that read prompts from stdin (Claude Code).
    pub stdin: Option<String>,
}

/// Manages background tasks, their lifecycle, output, and cancellation.
pub struct TaskManager {
    pub(crate) tasks: Mutex<Vec<TaskHandle>>,
    pub(crate) next_id: AtomicU64,
    pub(crate) cancellation_tokens: Mutex<HashMap<TaskId, CancellationToken>>,
    pub(crate) sink: Arc<dyn TaskEventSink>,
    /// Woken once per terminal transition — listeners subscribe via
    /// `wait_for_terminal`.
    pub(crate) terminal_notify: tokio::sync::Notify,
    /// Snapshot emitter used to push task state into the unified tasks
    /// drawer. Defaults to [`NoopTaskEmitter`] — the Tauri shell swaps in a
    /// real emitter during setup via [`Self::set_emitter`].
    pub(crate) emitter: StdMutex<Arc<dyn TaskEmitter>>,
    /// Last time an emission fired for each task id, used to throttle
    /// progress spam to [`PROGRESS_THROTTLE`].
    pub(crate) last_emit: StdMutex<HashMap<TaskId, Instant>>,
}

impl TaskManager {
    /// Create a new task manager that emits events through the given sink.
    ///
    /// The snapshot emitter defaults to a no-op — install a real one via
    /// [`Self::set_emitter`] after construction when the transport layer
    /// (typically Tauri) is available.
    pub fn new(sink: Arc<dyn TaskEventSink>) -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
            next_id: AtomicU64::new(1),
            cancellation_tokens: Mutex::new(HashMap::new()),
            sink,
            terminal_notify: tokio::sync::Notify::new(),
            emitter: StdMutex::new(Arc::new(NoopTaskEmitter)),
            last_emit: StdMutex::new(HashMap::new()),
        }
    }

    /// Swap in a real snapshot emitter. Safe to call at any time after
    /// construction — subsequent lifecycle transitions will use the new
    /// emitter. Used during Tauri setup where the `AppHandle` only becomes
    /// available after `TaskManager` has been constructed.
    pub fn set_emitter(&self, emitter: Arc<dyn TaskEmitter>) {
        *self.emitter.lock().expect("emitter mutex poisoned") = emitter;
    }

    /// Returns `true` when tasks of this kind should stream snapshots to the
    /// unified tasks drawer.
    ///
    /// Gating policy: surface git ops and one-shot headless AI tasks
    /// (commit-message, code-review, …) directly through this emitter.
    /// `AiBackground` / `AiInteractive` and `AppUpdate` flow through
    /// their own frontend bridges (`aiBackgroundRuns`, `autoUpdateTask`),
    /// so emitting them here would duplicate rows in the drawer; they
    /// stay off the allowlist on purpose. `Generic` is excluded so
    /// shell tasks the drawer doesn't care about (e.g. ad-hoc internal
    /// commands) don't leak in.
    pub(crate) fn should_emit(kind: &TaskKind) -> bool {
        matches!(
            kind,
            TaskKind::GitFetch
                | TaskKind::GitPull
                | TaskKind::GitPush
                | TaskKind::GitClone
                | TaskKind::AiHeadless
        )
    }

    /// Fire a snapshot through the emitter if gating allows and the throttle
    /// budget has elapsed. Lifecycle transitions (`force = true`) always fire.
    pub(crate) fn maybe_emit(&self, info: &TaskInfo, force: bool) {
        if !Self::should_emit(&info.task_kind) {
            return;
        }

        if !force {
            let mut last = self.last_emit.lock().expect("last_emit mutex poisoned");
            let now = Instant::now();
            if let Some(prev) = last.get(&info.id)
                && now.duration_since(*prev) < PROGRESS_THROTTLE
            {
                return;
            }
            last.insert(info.id, now);
        } else {
            // Refresh the timestamp so an immediately-following throttled
            // emission still waits the full window.
            self.last_emit
                .lock()
                .expect("last_emit mutex poisoned")
                .insert(info.id, Instant::now());
        }

        let emitter = self.emitter.lock().expect("emitter mutex poisoned").clone();
        emitter.emit(info);
    }

    /// Snapshot of all tasks for initial frontend load.
    pub async fn list_tasks(&self) -> Vec<TaskInfo> {
        let tasks = self.tasks.lock().await;
        tasks.iter().map(|t| t.to_info()).collect()
    }

    /// O(1)-ish status read for a single task.
    pub async fn get_status(&self, task_id: TaskId) -> Option<TaskStatus> {
        let tasks = self.tasks.lock().await;
        tasks
            .iter()
            .find(|t| t.id == task_id)
            .map(|t| t.status.clone())
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
    /// Convenience wrapper around [`Self::spawn_with_options`] that uses
    /// [`TaskKind::Generic`] and no stdin payload. Existing call sites keep
    /// their simple 6-argument form.
    pub async fn spawn(
        self: &Arc<Self>,
        label: String,
        command: &str,
        args: &[&str],
        cwd: &Path,
        cancellable: bool,
    ) -> TaskId {
        self.spawn_with_options(SpawnOptions {
            label,
            command,
            args,
            cwd,
            cancellable,
            kind: TaskKind::Generic,
            stdin: None,
        })
        .await
    }

    /// Spawn a CLI command as a background task with explicit [`TaskKind`]
    /// and optional stdin payload.
    ///
    /// Returns `TaskId` immediately. The spawned tokio task holds an
    /// `Arc<Self>` so the manager stays alive for the duration of the
    /// subprocess.
    pub async fn spawn_with_options(self: &Arc<Self>, options: SpawnOptions<'_>) -> TaskId {
        let SpawnOptions {
            label,
            command,
            args,
            cwd,
            cancellable,
            kind,
            stdin,
        } = options;
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
            kind,
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
        // Emit a snapshot for gated kinds so the drawer picks the task up
        // at `Running` before any output has streamed.
        self.maybe_emit(&info, true);
        self.sink.on_task_started(info).await;

        // Build the child process with piped stdout, stderr, and (optionally) stdin.
        let mut cmd = Command::new(command);
        cmd.args(args)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if stdin.is_some() {
            cmd.stdin(std::process::Stdio::piped());
        }

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

        // Write the stdin payload (if any) then close the pipe so the child
        // sees EOF and proceeds.
        if let Some(payload) = stdin
            && let Some(mut child_stdin) = child.stdin.take()
        {
            use tokio::io::AsyncWriteExt;
            // Best-effort — if the child died early or closed stdin, fail
            // the task rather than leaving it stuck.
            if let Err(e) = child_stdin.write_all(payload.as_bytes()).await {
                self.finish_task(
                    id,
                    TaskStatus::Failed {
                        error: format!("failed to write prompt on stdin: {e}"),
                    },
                )
                .await;
                let _ = child.kill().await;
                return id;
            }
            // Drop closes stdin explicitly.
            drop(child_stdin);
        }

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

        // Drop the throttle entry so a new spawn reusing the same id (unlikely
        // but possible in long-running processes) starts with a clean budget.
        {
            let mut last = self.last_emit.lock().expect("last_emit mutex poisoned");
            last.remove(&task_id);
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

        // Lifecycle transition — push a snapshot through the drawer
        // emitter before the existing sink callbacks so the drawer sees
        // the terminal state first.
        self.maybe_emit(&info, true);

        match status {
            TaskStatus::Completed => self.sink.on_task_completed(info).await,
            TaskStatus::Failed { .. } => self.sink.on_task_failed(info).await,
            TaskStatus::Cancelled => self.sink.on_task_cancelled(info).await,
            _ => {}
        }

        self.terminal_notify.notify_waiters();
    }

    /// Emit a progress snapshot for a running task.
    ///
    /// Throttled to [`PROGRESS_THROTTLE`]. Intended for callers that have
    /// progress signals outside the normal stdout/stderr stream (e.g. a
    /// future libgit2 fetch callback). No-op for non-gated kinds.
    pub async fn emit_progress(&self, task_id: TaskId) {
        let info = {
            let tasks = self.tasks.lock().await;
            match tasks.iter().find(|t| t.id == task_id) {
                Some(handle) => handle.to_info(),
                None => return,
            }
        };
        self.maybe_emit(&info, false);
    }

    /// Wait until the given task reaches a terminal state (`Completed`,
    /// `Failed`, or `Cancelled`) and return that status.
    ///
    /// Returns `TaskError::NotFound` immediately if the task has been
    /// evicted from the registry. Otherwise this suspends until the
    /// internal `Notify` is tripped by `finish_task`, re-checks the
    /// status, and returns it.
    pub async fn wait_for_terminal(&self, task_id: TaskId) -> Result<TaskStatus, TaskError> {
        loop {
            // Subscribe *before* the status check so we don't miss a wake.
            let waiter = self.terminal_notify.notified();
            tokio::pin!(waiter);

            {
                let tasks = self.tasks.lock().await;
                let handle = tasks
                    .iter()
                    .find(|t| t.id == task_id)
                    .ok_or(TaskError::NotFound(task_id))?;
                match &handle.status {
                    TaskStatus::Completed | TaskStatus::Failed { .. } | TaskStatus::Cancelled => {
                        return Ok(handle.status.clone());
                    }
                    _ => {}
                }
            }

            waiter.await;
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
    async fn test_spawn_with_stdin_payload() {
        let (manager, events) = new_manager();
        let cwd = std::env::temp_dir();

        // `cat` echoes stdin to stdout — ideal for verifying the pipe.
        let id = manager
            .spawn_with_options(SpawnOptions {
                label: "cat stdin".into(),
                command: "cat",
                args: &[],
                cwd: &cwd,
                cancellable: false,
                kind: TaskKind::Generic,
                stdin: Some("hello from stdin\nsecond line\n".into()),
            })
            .await;

        wait_for(&events, |ev| {
            ev.iter().any(|e| matches!(e, TaskEvent::Completed(_)))
        })
        .await;

        let output = manager.get_output(id).await.expect("output present");
        let texts: Vec<&str> = output.iter().map(|l| l.text.as_str()).collect();
        assert!(
            texts.iter().any(|t| t.contains("hello from stdin")),
            "stdin not piped through: {texts:?}"
        );
        assert!(texts.iter().any(|t| t.contains("second line")));
    }

    #[tokio::test]
    async fn test_spawn_with_ai_background_kind() {
        let (manager, events) = new_manager();
        let cwd = std::env::temp_dir();

        let id = manager
            .spawn_with_options(SpawnOptions {
                label: "ai run".into(),
                command: "sh",
                args: &["-c", "echo ok"],
                cwd: &cwd,
                cancellable: false,
                kind: TaskKind::AiBackground {
                    session_id: "sess-1".into(),
                    provider: "claude_code".into(),
                    worktree_path: "/tmp/wt".into(),
                },
                stdin: None,
            })
            .await;

        wait_for(&events, |ev| {
            ev.iter().any(|e| matches!(e, TaskEvent::Completed(_)))
        })
        .await;

        let info = manager
            .list_tasks()
            .await
            .into_iter()
            .find(|t| t.id == id)
            .expect("task info present");
        match info.task_kind {
            TaskKind::AiBackground {
                session_id,
                provider,
                worktree_path,
            } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(provider, "claude_code");
                assert_eq!(worktree_path, "/tmp/wt");
            }
            other => panic!("expected AiBackground kind, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_wait_for_terminal_on_completed() {
        let (manager, _events) = new_manager();
        let cwd = std::env::temp_dir();
        let id = manager
            .spawn("quick".into(), "sh", &["-c", "echo ok"], &cwd, false)
            .await;

        // Should return quickly with a terminal status — not time out.
        let status = tokio::time::timeout(Duration::from_secs(3), manager.wait_for_terminal(id))
            .await
            .expect("wait_for_terminal did not return in 3s")
            .expect("task vanished unexpectedly");

        assert!(matches!(status, TaskStatus::Completed));
    }

    #[tokio::test]
    async fn test_wait_for_terminal_missing_task() {
        let (manager, _events) = new_manager();
        let result = manager.wait_for_terminal(9999).await;
        assert!(result.is_err(), "expected NotFound error for missing id");
    }

    #[tokio::test]
    async fn test_get_status_returns_none_for_missing() {
        let (manager, _events) = new_manager();
        assert!(manager.get_status(42).await.is_none());
    }

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

    // ── Snapshot emitter tests ────────────────────────────────────────────────

    use std::sync::Mutex as StdMutex;

    /// Mock emitter that captures every snapshot for assertion.
    struct MockEmitter {
        captured: Arc<StdMutex<Vec<TaskInfo>>>,
    }

    impl MockEmitter {
        fn new() -> (Self, Arc<StdMutex<Vec<TaskInfo>>>) {
            let captured = Arc::new(StdMutex::new(Vec::new()));
            (
                Self {
                    captured: Arc::clone(&captured),
                },
                captured,
            )
        }
    }

    impl crate::sink::TaskEmitter for MockEmitter {
        fn emit(&self, info: &TaskInfo) {
            self.captured
                .lock()
                .expect("mock emitter mutex poisoned")
                .push(info.clone());
        }
    }

    /// A gated kind (`GitFetch`) emits at least `Running` then terminal.
    #[tokio::test]
    async fn emitter_captures_lifecycle_for_gated_kind() {
        let (manager, events) = new_manager();
        let (mock, captured) = MockEmitter::new();
        manager.set_emitter(Arc::new(mock));

        let cwd = std::env::temp_dir();
        manager
            .spawn_with_options(SpawnOptions {
                label: "Fetch origin".into(),
                command: "sh",
                args: &["-c", "echo hello"],
                cwd: &cwd,
                cancellable: false,
                kind: TaskKind::GitFetch,
                stdin: None,
            })
            .await;

        wait_for(&events, |ev| {
            ev.iter().any(|e| matches!(e, TaskEvent::Completed(_)))
        })
        .await;

        let snapshots = captured.lock().expect("captured mutex").clone();
        assert!(
            snapshots.len() >= 2,
            "expected at least running + terminal, got {} snapshots: {:?}",
            snapshots.len(),
            snapshots.iter().map(|s| &s.status).collect::<Vec<_>>()
        );
        // First snapshot must be Running.
        assert!(
            matches!(snapshots.first().unwrap().status, TaskStatus::Running),
            "first snapshot not Running: {:?}",
            snapshots.first().unwrap().status
        );
        // Last snapshot must be terminal (Completed here since echo succeeds).
        assert!(matches!(
            snapshots.last().unwrap().status,
            TaskStatus::Completed
        ));
    }

    /// Non-gated kinds (`Generic`, `AiBackground`) must NOT flow to the
    /// drawer emitter — those surfaces are served by other bridges.
    #[tokio::test]
    async fn emitter_skips_non_gated_kinds() {
        let (manager, events) = new_manager();
        let (mock, captured) = MockEmitter::new();
        manager.set_emitter(Arc::new(mock));

        let cwd = std::env::temp_dir();

        // Generic (default `spawn()`).
        manager
            .spawn("generic".into(), "sh", &["-c", "echo generic"], &cwd, false)
            .await;

        // AiBackground with payload.
        manager
            .spawn_with_options(SpawnOptions {
                label: "ai bg".into(),
                command: "sh",
                args: &["-c", "echo ai"],
                cwd: &cwd,
                cancellable: false,
                kind: TaskKind::AiBackground {
                    session_id: "sess".into(),
                    provider: "claude_code".into(),
                    worktree_path: "/tmp/wt".into(),
                },
                stdin: None,
            })
            .await;

        wait_for(&events, |ev| {
            ev.iter()
                .filter(|e| matches!(e, TaskEvent::Completed(_)))
                .count()
                >= 2
        })
        .await;

        let snapshots = captured.lock().expect("captured mutex");
        assert!(
            snapshots.is_empty(),
            "non-gated kinds leaked to emitter: {:?}",
            snapshots.iter().map(|s| &s.task_kind).collect::<Vec<_>>()
        );
    }

    /// 100 rapid `emit_progress` calls on a running gated task must be
    /// throttled to well under 100 snapshots thanks to the 200 ms window.
    #[tokio::test]
    async fn emitter_throttles_progress_spam() {
        let (manager, _events) = new_manager();
        let (mock, captured) = MockEmitter::new();
        manager.set_emitter(Arc::new(mock));

        let cwd = std::env::temp_dir();
        // Long-running task so it stays Running throughout the spam.
        let id = manager
            .spawn_with_options(SpawnOptions {
                label: "long fetch".into(),
                command: "sleep",
                args: &["30"],
                cwd: &cwd,
                cancellable: true,
                kind: TaskKind::GitFetch,
                stdin: None,
            })
            .await;

        // Give spawn's Running emission a moment to land.
        sleep(Duration::from_millis(10)).await;

        // 100 progress ticks with no delay — all but the first should hit the
        // throttle window and be dropped.
        for _ in 0..100 {
            manager.emit_progress(id).await;
        }

        // The baseline Running snapshot (from spawn) + at most ~1 additional
        // from emit_progress (since we spent <200 ms here). Definitely under 6.
        let count_before_cancel = captured.lock().expect("captured mutex").len();
        assert!(
            count_before_cancel <= 6,
            "throttle failed: {} emissions in <200 ms",
            count_before_cancel
        );

        // Cleanup: cancel the long-running sleep so the test runtime doesn't
        // leave a zombie behind.
        let _ = manager.cancel(id).await;
    }

    /// Lifecycle emissions always bypass the throttle — even if a progress
    /// emission fired a millisecond earlier.
    #[tokio::test]
    async fn emitter_lifecycle_bypasses_throttle() {
        let (manager, events) = new_manager();
        let (mock, captured) = MockEmitter::new();
        manager.set_emitter(Arc::new(mock));

        let cwd = std::env::temp_dir();
        let id = manager
            .spawn_with_options(SpawnOptions {
                label: "fetch".into(),
                command: "sh",
                args: &["-c", "echo done"],
                cwd: &cwd,
                cancellable: false,
                kind: TaskKind::GitFetch,
                stdin: None,
            })
            .await;

        // Fire a progress emission immediately — sets the throttle clock.
        manager.emit_progress(id).await;

        // Wait for the task to complete on its own.
        wait_for(&events, |ev| {
            ev.iter().any(|e| matches!(e, TaskEvent::Completed(_)))
        })
        .await;

        let snapshots = captured.lock().expect("captured mutex");
        // Must contain a terminal Completed snapshot despite the throttle
        // having been armed by emit_progress just prior.
        assert!(
            snapshots
                .iter()
                .any(|s| matches!(s.status, TaskStatus::Completed)),
            "terminal snapshot lost to throttle: {:?}",
            snapshots.iter().map(|s| &s.status).collect::<Vec<_>>()
        );
    }
}
