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
