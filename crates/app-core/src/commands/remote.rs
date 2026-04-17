//! Remote fetch, pull, push, rename, and remove commands.

use std::sync::Arc;

use task_runner::{TaskId, TaskManager};
use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Fetch all updates from a remote as a background task.
///
/// Spawns `git fetch <remote>` via the task manager and returns immediately
/// with the task ID. Progress streams to the task popover.
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::remote::fetch")]
pub async fn fetch_remote(
    remote: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Fetch {}", remote);
    let id = task_manager
        .spawn(label, "git", &["fetch", &remote], &cwd, true)
        .await;

    Ok(id)
}

/// Pull a branch from a remote (merge strategy) as a background task.
///
/// Spawns `git pull <remote> <branch>` via the task manager.
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::remote::pull")]
pub async fn pull_remote(
    remote: String,
    branch: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Pull {}/{}", remote, branch);
    let id = task_manager
        .spawn(label, "git", &["pull", &remote, &branch], &cwd, true)
        .await;

    Ok(id)
}

/// Push a branch to a remote as a background task.
///
/// Spawns `git push <remote> <branch>` via the task manager.
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::remote::push")]
pub async fn push_remote(
    remote: String,
    branch: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Push {}/{}", remote, branch);
    let id = task_manager
        .spawn(label, "git", &["push", &remote, &branch], &cwd, true)
        .await;

    Ok(id)
}

/// Renames a remote in the active repository.
///
/// Equivalent to `git remote rename <old_name> <new_name>`. Returns an error
/// if `old_name` does not exist or `new_name` is already taken.
#[tauri::command]
#[instrument(skip(state), name = "cmd::remote::rename")]
pub async fn rename_remote(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.rename_remote(&old_name, &new_name)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Removes a remote from the active repository.
///
/// Equivalent to `git remote remove <name>`. Returns an error if the remote
/// does not exist.
#[tauri::command]
#[instrument(skip(state), name = "cmd::remote::remove")]
pub async fn remove_remote(name: String, state: State<'_, AppState>) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.remove_remote(&name).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
