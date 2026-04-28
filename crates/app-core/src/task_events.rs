//! Wire-level `TaskEvent` payload streamed to the Svelte frontend over the
//! `task://update` Tauri event.
//!
//! # Phase 0 inventory (captured 2026-04-20)
//!
//! - `TaskManager` lives in `crates/task-runner/` — spawns CLI subprocesses and
//!   already exposes lifecycle callbacks via [`task_runner::TaskEventSink`]
//!   (`on_task_started`, `_output`, `_completed`, `_failed`, `_cancelled`).
//! - Existing `TaskHandle` fields: id, label, status, cancellable, started_at,
//!   finished_at, output, command, started_at_ms, exit_code, kind. No progress
//!   emission exists yet.
//! - `task_runner::TaskKind` previously only had `Generic` / `AiBackground`;
//!   the `GitFetch`/`Pull`/`Push`/`Clone`, `AiInteractive`, `AppUpdate` variants
//!   were added alongside this module so callers can tag git operations.
//! - AI background runs flow through `aiBackgroundRuns`; auto-update state lives
//!   in `src/lib/stores/autoUpdate.ts` with its own `updateTask` derived
//!   `TaskEntry` — the spec aligns this module's wire shape with that TS type.
//! - Current `StatusBar.svelte` (24 px) shows repo/branch/provider — will be
//!   replaced wholesale in Phase 4.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use task_runner::{
    TaskEmitter, TaskInfo, TaskKind as RuntimeTaskKind, TaskStatus as RuntimeTaskStatus,
};

/// Kind of task, flattened to a simple snake_case string for the frontend.
///
/// This is the wire-level taxonomy the unified tasks drawer understands — it
/// drops the structured payload that [`task_runner::TaskKind`] carries for
/// `AiBackground` (session id, provider, worktree path) because the drawer
/// surfaces that metadata through the existing `aiBackgroundRuns` bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Headless AI background run.
    AiBackground,
    /// Interactive AI PTY session (Claude Code, Codex, OpenCode).
    AiInteractive,
    /// One-shot AI command (commit message, code review, PR review,
    /// analyze). Surfaced in the drawer alongside the other AI kinds.
    AiHeadless,
    /// `git fetch <remote>`.
    GitFetch,
    /// `git pull <remote> <branch>`.
    GitPull,
    /// `git push <remote> <branch>`.
    GitPush,
    /// `git clone <url> <path>`.
    GitClone,
    /// Auto-update download driven by `tauri-plugin-updater`.
    AppUpdate,
}

/// Lifecycle phase of a task as seen by the drawer.
///
/// Note: `task_runner::TaskStatus::Completed` maps to [`TaskStatus::Success`]
/// for semantic clarity on the UI side ("success" vs. "error" vs. "cancelled").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is running.
    Running,
    /// Task completed with a zero exit code (or equivalent).
    Success,
    /// Task failed — see [`TaskEvent::error_message`] for details.
    Error,
    /// Task was cancelled by the user.
    Cancelled,
}

/// Optional progress information attached to a [`TaskEvent`].
///
/// Producers that only know "I'm working" emit `determinate = false` with
/// `current`/`total`/`percent` all `None`. Progress-aware producers fill in
/// what they can (bytes/count + percent).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgress {
    /// `true` when `total` is known and `percent` is meaningful.
    pub determinate: bool,
    /// Units already processed (bytes, objects, chunks, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<u64>,
    /// Total units expected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Percent complete (0..=100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
}

/// Wire payload emitted over the `task://update` Tauri event for every
/// lifecycle transition (and throttled progress update) of a task.
///
/// Mirrors the `TaskEntry` TypeScript type declared by the spec — the two
/// shapes must stay in sync so the Svelte bridge can consume events without
/// an adapter layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEvent {
    /// Stable identifier — the `TaskId` formatted as a string so the
    /// frontend aggregator can key against AI session ids and auto-update
    /// IDs uniformly.
    pub id: String,
    /// Which kind of operation this task represents.
    pub kind: TaskKind,
    /// Human-readable, translated title (e.g. `"Fetch origin"`).
    pub title: String,
    /// Optional one-line secondary text (remote, session id, repo, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Wall-clock ms since Unix epoch when the task started running.
    pub started_at_ms: u64,
    /// Wall-clock ms since Unix epoch when the task finished (or `None` while
    /// still running).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<u64>,
    /// Current lifecycle phase.
    pub status: TaskStatus,
    /// Optional progress snapshot. Absent when the producer has nothing to
    /// report (yet).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<TaskProgress>,
    /// Error message when `status == Error`. Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl TaskEvent {
    /// Convenience constructor for unit tests and the emitter impl.
    pub fn new(
        id: impl Into<String>,
        kind: TaskKind,
        title: impl Into<String>,
        status: TaskStatus,
        started_at_ms: u64,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            title: title.into(),
            subtitle: None,
            started_at_ms,
            finished_at_ms: None,
            status,
            progress: None,
            error_message: None,
        }
    }
}

/// Map the runtime [`task_runner::TaskKind`] onto the wire [`TaskKind`].
///
/// Returns `None` for variants that should not flow through the unified
/// tasks drawer — notably [`task_runner::TaskKind::Generic`], which is used
/// by shell tasks the drawer doesn't care about (the caller is expected to
/// re-tag those with an explicit kind before spawning).
pub fn kind_from_runtime(kind: &RuntimeTaskKind) -> Option<TaskKind> {
    match kind {
        RuntimeTaskKind::Generic => None,
        RuntimeTaskKind::AiBackground { .. } => Some(TaskKind::AiBackground),
        RuntimeTaskKind::AiInteractive => Some(TaskKind::AiInteractive),
        RuntimeTaskKind::AiHeadless => Some(TaskKind::AiHeadless),
        RuntimeTaskKind::GitFetch => Some(TaskKind::GitFetch),
        RuntimeTaskKind::GitPull => Some(TaskKind::GitPull),
        RuntimeTaskKind::GitPush => Some(TaskKind::GitPush),
        RuntimeTaskKind::GitClone => Some(TaskKind::GitClone),
        RuntimeTaskKind::AppUpdate => Some(TaskKind::AppUpdate),
    }
}

/// Project a runtime [`TaskInfo`] into a wire [`TaskEvent`].
///
/// `started_at_ms` defaults to `0` if the handle didn't capture a wall-clock
/// timestamp (shouldn't happen in practice since `spawn_with_options` always
/// sets it, but the type system allows it).
impl From<&TaskInfo> for TaskEvent {
    fn from(info: &TaskInfo) -> Self {
        let kind = kind_from_runtime(&info.task_kind).unwrap_or(TaskKind::GitFetch);

        let (status, error_message) = match &info.status {
            RuntimeTaskStatus::Queued | RuntimeTaskStatus::Running => (TaskStatus::Running, None),
            RuntimeTaskStatus::Completed => (TaskStatus::Success, None),
            RuntimeTaskStatus::Failed { error } => (TaskStatus::Error, Some(error.clone())),
            RuntimeTaskStatus::Cancelled => (TaskStatus::Cancelled, None),
        };

        let started_at_ms = info.started_at_ms.unwrap_or(0);

        // Compute finished_at_ms from elapsed_secs when the task is in a
        // terminal state. Avoids carrying a second wall-clock field through
        // `TaskHandle`.
        let finished_at_ms = match status {
            TaskStatus::Success | TaskStatus::Error | TaskStatus::Cancelled => info
                .elapsed_secs
                .map(|secs| started_at_ms.saturating_add((secs * 1000.0) as u64)),
            TaskStatus::Running => None,
        };

        Self {
            id: info.id.to_string(),
            kind,
            title: info.label.clone(),
            subtitle: None,
            started_at_ms,
            finished_at_ms,
            status,
            progress: None,
            error_message,
        }
    }
}

// ─── Tauri transport ──────────────────────────────────────────────────────────

/// Tauri event name streamed to the Svelte frontend for every snapshot.
///
/// Consumers listen via `listen<TaskEvent>("task://update", …)` in
/// `src/lib/stores/tasks.ts`.
pub const TASK_UPDATE_EVENT: &str = "task://update";

/// Concrete [`TaskEmitter`] that forwards runtime [`TaskInfo`] snapshots to
/// the Tauri frontend as a serialized [`TaskEvent`].
///
/// Non-gated kinds never reach this emitter — the gating happens upstream in
/// [`task_runner::TaskManager::maybe_emit`], so `emit` can do a blind
/// projection and `app.emit` without re-checking.
pub struct TauriEmitter {
    app_handle: AppHandle,
}

impl TauriEmitter {
    /// Wrap a Tauri [`AppHandle`] into a [`TaskEmitter`].
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl TaskEmitter for TauriEmitter {
    fn emit(&self, info: &TaskInfo) {
        let event: TaskEvent = info.into();
        // Best-effort: emit errors (e.g. serialization failure or app
        // shutdown) are swallowed — the drawer will reconcile on the next
        // lifecycle transition.
        let _ = self.app_handle.emit(TASK_UPDATE_EVENT, &event);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip a fixture `TaskEvent` through serde_json and verify every
    /// field survives intact.
    #[test]
    fn task_event_json_roundtrip() {
        let event = TaskEvent {
            id: "42".into(),
            kind: TaskKind::GitFetch,
            title: "Fetch origin".into(),
            subtitle: Some("beardgit".into()),
            started_at_ms: 1_700_000_000_000,
            finished_at_ms: Some(1_700_000_003_500),
            status: TaskStatus::Success,
            progress: Some(TaskProgress {
                determinate: true,
                current: Some(14),
                total: Some(42),
                percent: Some(33),
            }),
            error_message: None,
        };

        let json = serde_json::to_string(&event).expect("serialize");
        let back: TaskEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, back);
    }

    /// The wire payload must use snake_case literals for enum variants so the
    /// Svelte `TaskEntry.kind` union matches byte-for-byte.
    #[test]
    fn task_event_uses_snake_case_enums() {
        let event = TaskEvent::new(
            "1",
            TaskKind::AiBackground,
            "Claude run",
            TaskStatus::Running,
            1_700_000_000_000,
        );

        let json = serde_json::to_string(&event).expect("serialize");

        assert!(
            json.contains("\"ai_background\""),
            "missing snake_case ai_background: {json}"
        );
        assert!(
            json.contains("\"running\""),
            "missing snake_case running: {json}"
        );
        assert!(
            !json.contains("\"AiBackground\""),
            "leaked PascalCase in payload: {json}"
        );
    }

    /// Progress field is optional — omitted entirely when absent.
    #[test]
    fn task_event_omits_optional_fields_when_none() {
        let event = TaskEvent::new(
            "7",
            TaskKind::GitFetch,
            "Fetch",
            TaskStatus::Running,
            1_700_000_000_000,
        );

        let json = serde_json::to_string(&event).expect("serialize");
        assert!(!json.contains("progress"), "progress leaked: {json}");
        assert!(!json.contains("subtitle"), "subtitle leaked: {json}");
        assert!(
            !json.contains("error_message"),
            "error_message leaked: {json}"
        );
    }

    /// `TaskStatus::Completed` (runtime) → `Success` (wire) is the one
    /// non-obvious mapping — pin it down explicitly.
    #[test]
    fn status_success_renders_as_success() {
        use serde_json::json;
        let event = TaskEvent {
            id: "3".into(),
            kind: TaskKind::AppUpdate,
            title: "Downloading BeardGit v0.2.1".into(),
            subtitle: None,
            started_at_ms: 0,
            finished_at_ms: None,
            status: TaskStatus::Success,
            progress: None,
            error_message: None,
        };
        let value = serde_json::to_value(&event).expect("serialize");
        assert_eq!(value["status"], json!("success"));
        assert_eq!(value["kind"], json!("app_update"));
    }

    /// `From<&TaskInfo>` should project a running git-fetch task cleanly.
    #[test]
    fn from_task_info_running_git_fetch() {
        let info = TaskInfo {
            id: 9,
            label: "Fetch origin".into(),
            status: RuntimeTaskStatus::Running,
            cancellable: true,
            elapsed_secs: Some(1.5),
            command: "git fetch origin".into(),
            started_at_ms: Some(1_700_000_000_000),
            exit_code: None,
            task_kind: RuntimeTaskKind::GitFetch,
        };

        let event: TaskEvent = (&info).into();
        assert_eq!(event.id, "9");
        assert_eq!(event.kind, TaskKind::GitFetch);
        assert_eq!(event.title, "Fetch origin");
        assert_eq!(event.status, TaskStatus::Running);
        assert_eq!(event.started_at_ms, 1_700_000_000_000);
        assert!(event.finished_at_ms.is_none());
        assert!(event.error_message.is_none());
    }

    /// Failed tasks carry the stderr payload through `error_message`.
    #[test]
    fn from_task_info_failed_carries_error_message() {
        let info = TaskInfo {
            id: 10,
            label: "Push origin/main".into(),
            status: RuntimeTaskStatus::Failed {
                error: "remote rejected".into(),
            },
            cancellable: true,
            elapsed_secs: Some(0.25),
            command: "git push origin main".into(),
            started_at_ms: Some(1_700_000_000_000),
            exit_code: Some(1),
            task_kind: RuntimeTaskKind::GitPush,
        };

        let event: TaskEvent = (&info).into();
        assert_eq!(event.status, TaskStatus::Error);
        assert_eq!(event.error_message.as_deref(), Some("remote rejected"));
        assert_eq!(event.finished_at_ms, Some(1_700_000_000_250));
    }

    /// `Completed` runtime status maps to `Success` on the wire.
    #[test]
    fn from_task_info_completed_maps_to_success() {
        let info = TaskInfo {
            id: 11,
            label: "Fetch".into(),
            status: RuntimeTaskStatus::Completed,
            cancellable: true,
            elapsed_secs: Some(0.5),
            command: "git fetch".into(),
            started_at_ms: Some(1_700_000_000_000),
            exit_code: Some(0),
            task_kind: RuntimeTaskKind::GitFetch,
        };

        let event: TaskEvent = (&info).into();
        assert_eq!(event.status, TaskStatus::Success);
        assert_eq!(event.finished_at_ms, Some(1_700_000_000_500));
    }

    /// AiBackground runtime kind (with its payload) collapses to the flat
    /// wire variant.
    #[test]
    fn from_task_info_ai_background_flattens_payload() {
        let info = TaskInfo {
            id: 12,
            label: "Claude Code run".into(),
            status: RuntimeTaskStatus::Running,
            cancellable: true,
            elapsed_secs: Some(10.0),
            command: "claude ...".into(),
            started_at_ms: Some(1_700_000_000_000),
            exit_code: None,
            task_kind: RuntimeTaskKind::AiBackground {
                session_id: "sess-1".into(),
                provider: "claude_code".into(),
                worktree_path: "/tmp/wt".into(),
            },
        };

        let event: TaskEvent = (&info).into();
        assert_eq!(event.kind, TaskKind::AiBackground);
    }
}
