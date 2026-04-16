//! Stash push, pop, apply, drop, list, and show commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Push the current working-tree changes onto the stash stack.
///
/// # Parameters
/// - `message` – Optional stash description (equivalent to `git stash push -m <msg>`).
///
/// # Returns
/// The stdout of `git stash push` on success, or stderr as an error.
#[tauri::command]
pub async fn stash_push(
    message: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo
            .stash_push(message.as_deref())
            .map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Pop (apply and drop) a stash entry.
///
/// # Parameters
/// - `index` – Zero-based stash index to pop (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash pop` on success, or stderr as an error.
#[tauri::command]
pub async fn stash_pop(index: Option<usize>, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.stash_pop(index).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return a list of stash entry descriptions (one per stash entry).
///
/// Each string corresponds to a line from `git stash list`.
#[tauri::command]
pub async fn stash_list(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.stash_list().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Apply a stash entry without removing it.
///
/// # Parameters
/// - `index` – Zero-based stash index to apply (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash apply` on success, or stderr as an error.
#[tauri::command]
pub async fn stash_apply(
    index: Option<usize>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.stash_apply(index).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Restore a single file from a stash entry into the working directory.
///
/// # Parameters
/// - `index` – Zero-based stash index.
/// - `path` – Repo-relative file path to restore.
///
/// # Returns
/// The stdout of `git restore` on success, or stderr as an error.
#[tauri::command]
pub async fn stash_apply_file(
    index: usize,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo
            .stash_apply_file(index, &path)
            .map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Drop a stash entry without applying it.
///
/// # Parameters
/// - `index` – Zero-based stash index to drop (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash drop` on success, or stderr as an error.
#[tauri::command]
pub async fn stash_drop(
    index: Option<usize>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.stash_drop(index).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return structured stash entries with parsed metadata.
///
/// Each entry includes index, message, branch, timestamp, and commit OID.
#[tauri::command]
pub async fn stash_entries(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::StashEntry>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.stash_entries().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return the diff of a stash entry as structured `FileDiff` objects.
///
/// # Parameters
/// - `index` – Zero-based stash index (defaults to 0, i.e. the latest stash).
#[tauri::command]
pub async fn stash_show_parsed(
    index: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileDiff>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.stash_show_parsed(index).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
