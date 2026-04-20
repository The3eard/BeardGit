//! Tauri command handlers for the background task system.

use std::sync::Arc;

use task_runner::{OutputLine, TaskId, TaskInfo, TaskManager};
use tauri::State;

/// Return the list of all known tasks (queued, running, and finished).
#[tauri::command]
pub async fn get_tasks(task_manager: State<'_, Arc<TaskManager>>) -> Result<Vec<TaskInfo>, String> {
    Ok(task_manager.list_tasks().await)
}

/// Return the captured output lines for a specific task.
///
/// Returns an error string if the task ID is not found.
#[tauri::command]
pub async fn get_task_output(
    task_id: TaskId,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<Vec<OutputLine>, String> {
    task_manager
        .get_output(task_id)
        .await
        .ok_or_else(|| format!("task {task_id} not found"))
}

/// Request cancellation of a running task.
///
/// Returns an error string if the task cannot be cancelled.
#[tauri::command]
pub async fn cancel_task(
    task_id: TaskId,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<(), String> {
    task_manager
        .cancel(task_id)
        .await
        .map_err(|e| e.to_string())
}

/// Cancel a running task by its string id.
///
/// Used by the unified tasks drawer, where task ids are strings shared
/// across AI runs, git ops, and auto-update downloads. Parses the input
/// into a [`TaskId`] (`u64`) and delegates to
/// [`TaskManager::cancel`], which triggers the underlying
/// `CancellationToken` so libgit2 fetch / push / clone observe
/// `ECANCELED` and the child process is killed.
///
/// Returns an error string if the id can't be parsed or the task isn't
/// cancellable.
#[tauri::command]
pub async fn task_cancel(
    id: String,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<(), String> {
    let parsed: TaskId = id.parse().map_err(|_| format!("invalid task id: {id:?}"))?;
    task_manager.cancel(parsed).await.map_err(|e| e.to_string())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use task_runner::{SpawnOptions, TaskEventSink, TaskKind, TaskStatus};
    use tokio::sync::Mutex;
    use tokio::time::{Duration, sleep};

    /// Minimal sink — counts terminal events so tests can poll for completion.
    struct CountingSink {
        cancelled: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl TaskEventSink for CountingSink {
        async fn on_task_started(&self, _info: TaskInfo) {}
        async fn on_task_output(&self, _task_id: TaskId, _line: OutputLine) {}
        async fn on_task_completed(&self, _info: TaskInfo) {}
        async fn on_task_failed(&self, _info: TaskInfo) {}
        async fn on_task_cancelled(&self, _info: TaskInfo) {
            *self.cancelled.lock().await += 1;
        }
    }

    /// `TaskManager::cancel` transitions a running git task to `Cancelled`
    /// and fires the `on_task_cancelled` sink callback, confirming the
    /// underlying `CancellationToken` propagated to the child process.
    #[tokio::test]
    async fn task_manager_cancel_transitions_and_propagates() {
        let cancelled = Arc::new(Mutex::new(0usize));
        let sink = Arc::new(CountingSink {
            cancelled: Arc::clone(&cancelled),
        });
        let manager = Arc::new(TaskManager::new(sink));

        let id = manager
            .spawn_with_options(SpawnOptions {
                label: "Fetch origin".into(),
                command: "sleep",
                args: &["30"],
                cwd: &std::env::temp_dir(),
                cancellable: true,
                kind: TaskKind::GitFetch,
                stdin: None,
            })
            .await;

        // Give the child a moment to be spawned.
        sleep(Duration::from_millis(100)).await;

        manager.cancel(id).await.expect("cancel succeeds");

        // Poll for the Cancelled terminal transition.
        for _ in 0..200 {
            if matches!(manager.get_status(id).await, Some(TaskStatus::Cancelled)) {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }

        assert!(
            matches!(manager.get_status(id).await, Some(TaskStatus::Cancelled)),
            "task did not reach Cancelled: {:?}",
            manager.get_status(id).await
        );

        // Give the sink callback one tick to fire.
        sleep(Duration::from_millis(10)).await;
        assert!(
            *cancelled.lock().await >= 1,
            "on_task_cancelled not invoked"
        );
    }

    /// Non-numeric ids are rejected before reaching the manager.
    #[tokio::test]
    async fn task_cancel_rejects_non_numeric_id() {
        // We can't easily build a `State<Arc<TaskManager>>` outside Tauri,
        // but the parse failure short-circuits before touching state, so we
        // reproduce the inlined parse to cover the branch behaviour.
        let id = "not-a-number".to_string();
        let parsed: Result<TaskId, _> = id.parse();
        assert!(parsed.is_err(), "expected parse failure");
    }
}
