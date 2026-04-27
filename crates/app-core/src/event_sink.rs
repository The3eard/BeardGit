//! Tauri event sinks that bridge domain crates to the Tauri event system.

use std::path::Path;
use std::sync::{Arc, Mutex};

use ai_provider::AiSession;
use async_trait::async_trait;
use mutation_events::{AiSource, MutationFlags, MutationKind, emit_mutation};
use serde::Serialize;
use task_runner::{OutputLine, TaskEventSink, TaskId, TaskInfo, TaskKind, TaskOutputEvent};
use tauri::{AppHandle, Emitter};

use crate::ai_background::{AiBackgroundCoordinator, AiBackgroundEventSink};

/// Payload emitted for every AI background output line.
#[derive(Debug, Clone, Serialize)]
pub struct AiBackgroundOutputEvent<'a> {
    pub session_id: &'a str,
    pub line: &'a str,
}

/// Forwards task events to the Tauri frontend via `AppHandle::emit`, and also
/// notifies the [`AiBackgroundCoordinator`] so it can recognise AI background
/// task lifecycle events and maintain its session registry.
pub struct TauriEventSink {
    app_handle: AppHandle,
    /// Late-bound — set from `src-tauri/src/lib.rs::setup` once the
    /// coordinator is constructed.
    coordinator: Mutex<Option<Arc<AiBackgroundCoordinator>>>,
}

impl TauriEventSink {
    /// Create a new [`TauriEventSink`] wrapping the given app handle.
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            coordinator: Mutex::new(None),
        }
    }

    /// Install the AI background coordinator so task lifecycle events can
    /// drive its internal state.
    pub fn install_ai_background_coordinator(&self, coord: Arc<AiBackgroundCoordinator>) {
        *self.coordinator.lock().expect("coordinator mutex") = Some(coord);
    }

    fn ai_coord(&self) -> Option<Arc<AiBackgroundCoordinator>> {
        self.coordinator.lock().expect("coordinator mutex").clone()
    }

    fn is_ai_background(info: &TaskInfo) -> bool {
        matches!(info.task_kind, TaskKind::AiBackground { .. })
    }
}

#[async_trait]
impl TaskEventSink for TauriEventSink {
    async fn on_task_started(&self, info: TaskInfo) {
        let _ = self.app_handle.emit("task-started", &info);
    }

    async fn on_task_output(&self, task_id: TaskId, line: OutputLine) {
        // Route AI background task output to the coordinator first so it can
        // parse usage and forward an `ai-background-output` event.
        if let Some(coord) = self.ai_coord() {
            coord.on_task_output(task_id, &line.text);
        }
        let _ = self
            .app_handle
            .emit("task-output", &TaskOutputEvent { task_id, line });
    }

    async fn on_task_completed(&self, info: TaskInfo) {
        if let Some(coord) = self.ai_coord()
            && Self::is_ai_background(&info)
        {
            coord.on_task_finished(info.id, info.exit_code, false, None);
        }
        let _ = self.app_handle.emit("task-completed", &info);
    }

    async fn on_task_failed(&self, info: TaskInfo) {
        if let Some(coord) = self.ai_coord()
            && Self::is_ai_background(&info)
        {
            let err_text = match &info.status {
                task_runner::TaskStatus::Failed { error } => Some(error.clone()),
                _ => None,
            };
            coord.on_task_finished(info.id, info.exit_code, false, err_text);
        }
        let _ = self.app_handle.emit("task-failed", &info);
    }

    async fn on_task_cancelled(&self, info: TaskInfo) {
        if let Some(coord) = self.ai_coord()
            && Self::is_ai_background(&info)
        {
            coord.on_task_finished(info.id, info.exit_code, true, None);
        }
        let _ = self.app_handle.emit("task-cancelled", &info);
    }
}

/// Concrete [`AiBackgroundEventSink`] that forwards session/output events to
/// the Tauri frontend.
pub struct TauriAiBackgroundEventSink {
    app_handle: AppHandle,
}

impl TauriAiBackgroundEventSink {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl AiBackgroundEventSink for TauriAiBackgroundEventSink {
    fn on_status(&self, session: &AiSession) {
        let _ = self.app_handle.emit("ai-background-status", session);
    }

    fn on_output(&self, session_id: &str, line: &str) {
        let _ = self.app_handle.emit(
            "ai-background-output",
            &AiBackgroundOutputEvent { session_id, line },
        );
    }

    fn on_repo_mutated(&self, worktree_path: &Path, source: AiSource, flags: MutationFlags) {
        if let Err(err) = emit_mutation(
            &self.app_handle,
            MutationKind::Ai { source },
            flags,
            worktree_path,
        ) {
            tracing::warn!(
                ?err,
                worktree = %worktree_path.display(),
                "failed to emit project-mutated for AI background run"
            );
        }
    }
}
