//! Advanced git commands: cherry-pick, revert, reset, blame, file history, interactive rebase.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Cherry-pick a commit onto the current branch.
///
/// # Arguments
/// - `oid` – Full or abbreviated SHA of the commit to cherry-pick.
///
/// # Returns
/// The stdout of `git cherry-pick` on success, or stderr as an error.
#[tauri::command]
pub async fn cherry_pick(oid: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.cherry_pick(&oid).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Revert a commit, creating a new commit that undoes its changes.
///
/// # Arguments
/// - `oid` – Full or abbreviated SHA of the commit to revert.
///
/// # Returns
/// The stdout of `git revert` on success, or stderr as an error.
#[tauri::command]
pub async fn revert_commit(oid: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.revert_commit(&oid).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reset HEAD to a specific commit.
///
/// # Arguments
/// - `oid`  – Full or abbreviated SHA of the target commit.
/// - `mode` – Reset mode: `"soft"`, `"mixed"`, or `"hard"`.
///
/// # Returns
/// `Ok(())` on success, or an error string if the mode is invalid or
/// `git reset` exits with a non-zero status.
#[tauri::command]
pub async fn reset_to_commit(
    oid: String,
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.reset_to_commit(&oid, &mode).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get per-line blame information for a file, optionally at a specific commit.
///
/// # Parameters
/// - `path` – Repository-relative file path to blame.
/// - `oid` – Optional commit OID; when `None`, blame is computed at HEAD.
#[tauri::command]
pub async fn blame_file(
    path: String,
    oid: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::BlameLine>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.blame_file(&path, oid.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get the commit history for a specific file with rename tracking.
///
/// # Parameters
/// - `path` – Repository-relative file path.
/// - `limit` – Maximum number of entries to return (default 100).
#[tauri::command]
pub async fn file_history(
    path: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileHistoryEntry>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.file_history(&path, limit).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get the commits between `base_oid` (exclusive) and HEAD in rebase order.
///
/// Returns the commit list that would appear in `git rebase -i` for the given
/// base, ordered oldest-first.
#[tauri::command]
pub async fn get_rebase_commits(
    base_oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::RebaseCommit>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.get_rebase_commits(&base_oid)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Start an interactive rebase with pre-defined actions.
///
/// Each action specifies a commit OID and a rebase verb (`pick`, `squash`,
/// `fixup`, `edit`, `drop`). The todo file is injected via `GIT_SEQUENCE_EDITOR`
/// so no interactive terminal is required.
#[tauri::command]
pub async fn start_interactive_rebase(
    base_oid: String,
    actions: Vec<git_engine::RebaseAction>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.start_interactive_rebase(&base_oid, &actions)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
