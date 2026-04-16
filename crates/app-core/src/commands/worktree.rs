//! Worktree listing, creation, and removal commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// List all worktrees for the active repository, including the main worktree.
///
/// Returns a [`WorktreeInfo`] for each worktree. The first element is always
/// the main worktree.
#[tauri::command]
pub async fn list_worktrees(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::WorktreeInfo>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.list_worktrees().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Create a new linked worktree at `path` on `branch`.
///
/// # Parameters
/// - `path` – Absolute filesystem path where the new worktree directory will be created.
/// - `branch` – Branch name to check out (or create when `create_branch` is `true`).
/// - `create_branch` – When `true`, create a new branch with `-b`; when `false`, check
///   out an existing branch.
#[tauri::command]
pub async fn create_worktree(
    path: String,
    branch: String,
    create_branch: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.create_worktree(&path, &branch, create_branch)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Remove a linked worktree at `path`.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the worktree directory to remove.
/// - `force` – When `true`, remove the worktree even if it has uncommitted changes
///   or is locked.
#[tauri::command]
pub async fn remove_worktree(
    path: String,
    force: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.remove_worktree(&path, force)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
