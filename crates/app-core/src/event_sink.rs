//! Tauri event sink that bridges the `task-runner` crate to the Tauri event system.

use async_trait::async_trait;
use task_runner::{OutputLine, TaskEventSink, TaskId, TaskInfo, TaskOutputEvent};
use tauri::{AppHandle, Emitter};

/// Forwards task events to the Tauri frontend via `AppHandle::emit`.
pub struct TauriEventSink {
    app_handle: AppHandle,
}

impl TauriEventSink {
    /// Create a new [`TauriEventSink`] wrapping the given app handle.
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl TaskEventSink for TauriEventSink {
    async fn on_task_started(&self, info: TaskInfo) {
        let _ = self.app_handle.emit("task-started", &info);
    }

    async fn on_task_output(&self, task_id: TaskId, line: OutputLine) {
        let _ = self
            .app_handle
            .emit("task-output", &TaskOutputEvent { task_id, line });
    }

    async fn on_task_completed(&self, info: TaskInfo) {
        let _ = self.app_handle.emit("task-completed", &info);
    }

    async fn on_task_failed(&self, info: TaskInfo) {
        let _ = self.app_handle.emit("task-failed", &info);
    }

    async fn on_task_cancelled(&self, info: TaskInfo) {
        let _ = self.app_handle.emit("task-cancelled", &info);
    }
}
