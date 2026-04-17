//! Core types for the background task system.

use std::time::Instant;

use serde::Serialize;

/// Unique identifier for a background task.
pub type TaskId = u64;

/// Current lifecycle state of a task.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum TaskStatus {
    /// Reserved for future queuing support. Currently, tasks transition
    /// directly to `Running` when spawned.
    Queued,
    Running,
    Completed,
    /// The task finished with a non-zero exit code. `error` contains stderr text.
    Failed {
        error: String,
    },
    Cancelled,
}

/// Discriminator for the *kind* of task that's running.
///
/// Internally tagged with `"kind"` so new variants are additive — the
/// frontend task panel can branch on `kind` to render task-type-specific UI
/// (progress badges, worktree path, session id, etc.) without breaking older
/// clients that don't know the new variants.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskKind {
    /// Generic shell task (default — used by `TaskManager::spawn`).
    #[default]
    Generic,
    /// Headless AI background run launched from the AI background dialog.
    ///
    /// Payload is carried alongside so the task panel can jump straight to
    /// the associated session.
    AiBackground {
        session_id: String,
        provider: String,
        worktree_path: String,
    },
}

/// Which output stream a line came from.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    Stdout,
    Stderr,
}

/// A single line of output captured from a running task.
#[derive(Clone, Debug, Serialize)]
pub struct OutputLine {
    pub stream: Stream,
    pub text: String,
    /// Skipped during serialization because `Instant` is not meaningful across processes.
    #[serde(skip)]
    pub timestamp: Instant,
}

/// Internal representation of a task with its full output buffer.
///
/// Not serialized directly — converted to [`TaskInfo`] for the frontend.
pub struct TaskHandle {
    pub id: TaskId,
    pub label: String,
    pub status: TaskStatus,
    pub cancellable: bool,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
    pub output: Vec<OutputLine>,
    /// The full command string (program + args) for display purposes.
    pub command: String,
    /// Wall-clock time when the task was started, as milliseconds since Unix epoch.
    pub started_at_ms: Option<u64>,
    /// Process exit code, captured after the child process terminates.
    pub exit_code: Option<i32>,
    /// Which task category this is — generic shell task or an AI background run.
    pub kind: TaskKind,
}

/// Serializable subset of a task sent to the frontend via events.
#[derive(Clone, Debug, Serialize)]
pub struct TaskInfo {
    pub id: TaskId,
    pub label: String,
    pub status: TaskStatus,
    pub cancellable: bool,
    /// Wall-clock seconds since the task started, or `None` if not yet started.
    pub elapsed_secs: Option<f64>,
    /// The full command string (program + args) for display purposes.
    pub command: String,
    /// Wall-clock time when the task was started, as milliseconds since Unix epoch.
    pub started_at_ms: Option<u64>,
    /// Process exit code, captured after the child process terminates.
    pub exit_code: Option<i32>,
    /// Which task category this is — generic shell task or an AI background run.
    pub task_kind: TaskKind,
}

/// Payload for the `task-output` Tauri event.
#[derive(Clone, Debug, Serialize)]
pub struct TaskOutputEvent {
    pub task_id: TaskId,
    pub line: OutputLine,
}

/// Errors that can occur when interacting with the task manager.
#[derive(thiserror::Error, Debug)]
pub enum TaskError {
    /// The requested task ID does not exist.
    #[error("task {0} not found")]
    NotFound(TaskId),
    /// The task exists but is not in the `Running` state.
    #[error("task {0} is not running")]
    NotRunning(TaskId),
    /// The task was spawned with `cancellable = false`.
    #[error("task {0} is not cancellable")]
    NotCancellable(TaskId),
    /// An I/O error occurred (e.g. spawning the child process).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl TaskHandle {
    pub fn to_info(&self) -> TaskInfo {
        let elapsed_secs = self.started_at.map(|start| {
            let end = self.finished_at.unwrap_or_else(Instant::now);
            end.duration_since(start).as_secs_f64()
        });
        TaskInfo {
            id: self.id,
            label: self.label.clone(),
            status: self.status.clone(),
            cancellable: self.cancellable,
            elapsed_secs,
            command: self.command.clone(),
            started_at_ms: self.started_at_ms,
            exit_code: self.exit_code,
            task_kind: self.kind.clone(),
        }
    }
}
