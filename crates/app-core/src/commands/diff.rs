//! Diff and file content commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// List files changed by a commit, including their change status and patch.
///
/// # Parameters
/// - `oid` – Full or abbreviated commit SHA.
#[tauri::command]
pub fn get_commit_files(
    oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitFileChange>, String> {
    with_active_repo(&state, |repo| {
        repo.commit_files(&oid).map_err(|e| e.to_string())
    })
}

/// Return files changed between two arbitrary commits.
///
/// # Parameters
/// - `from_oid` – SHA of the base commit.
/// - `to_oid` – SHA of the target commit.
#[tauri::command]
pub fn get_diff_between_commits(
    from_oid: String,
    to_oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitFileChange>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_commits(&from_oid, &to_oid)
            .map_err(|e| e.to_string())
    })
}

/// Return the full diff (hunks + lines) for a single file in a commit.
#[tauri::command]
#[instrument(skip(state), name = "cmd::diff::commit_file_diff")]
pub async fn get_commit_file_diff(
    oid: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileDiff>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.commit_file_diff(&oid, &path)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Returns raw file content at a specific commit.
///
/// # Parameters
/// - `oid` – Full or abbreviated commit SHA.
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw UTF-8 file content (binary blobs are lossy-decoded), or an error string
/// if the OID or path is invalid.
#[tauri::command]
pub fn get_file_at_commit(
    oid: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_at_commit(&oid, &path)
            .map_err(|e| e.to_string())
    })
}

/// Returns raw file content from the working directory.
///
/// # Parameters
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw file content, or an IO error string if the file does not exist.
#[tauri::command]
pub fn get_file_workdir(path: String, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_workdir(&path).map_err(|e| e.to_string())
    })
}

/// Returns raw file content from the index (staged version).
///
/// # Parameters
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw staged file content, or an error string if the file is not staged.
#[tauri::command]
pub fn get_file_index(path: String, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_index(&path).map_err(|e| e.to_string())
    })
}

/// Return the unstaged diff between the working tree and the index.
///
/// Equivalent to `git diff` (without `--cached`).
#[tauri::command]
pub fn get_diff_workdir(state: State<'_, AppState>) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_workdir().map_err(|e| e.to_string())
    })
}

/// Return the staged diff between the index and HEAD.
///
/// Equivalent to `git diff --cached`.
#[tauri::command]
pub fn get_diff_index(state: State<'_, AppState>) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| repo.diff_index().map_err(|e| e.to_string()))
}
