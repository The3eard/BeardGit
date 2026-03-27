//! Event sink trait for decoupling task events from the transport layer.
//!
//! The `task-runner` crate emits events through this trait. Consumers (e.g.
//! `app-core`) implement it to forward events to Tauri, a test harness, etc.

use async_trait::async_trait;

use crate::types::{OutputLine, TaskId, TaskInfo};

/// Receives task lifecycle and output events.
///
/// All methods take `&self` so the sink can be shared across tokio tasks.
#[async_trait]
pub trait TaskEventSink: Send + Sync {
    /// A task has started executing.
    async fn on_task_started(&self, info: TaskInfo);

    /// A task produced a line of output (stdout or stderr).
    async fn on_task_output(&self, task_id: TaskId, line: OutputLine);

    /// A task completed successfully (exit code 0).
    async fn on_task_completed(&self, info: TaskInfo);

    /// A task failed (non-zero exit code).
    async fn on_task_failed(&self, info: TaskInfo);

    /// A task was cancelled by the user.
    async fn on_task_cancelled(&self, info: TaskInfo);
}
