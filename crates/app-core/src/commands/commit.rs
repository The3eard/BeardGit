//! Commit creation and amendment commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Create a new commit from the current index with the given message and author.
///
/// # Parameters
/// - `message` – Commit message (subject + optional body).
///
/// # Returns
/// The OID of the newly created commit as a hex string.
#[tauri::command]
#[instrument(skip(state), name = "cmd::commit::create")]
pub fn create_commit(message: String, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.create_commit(&message).map_err(|e| e.to_string())
    })
}

/// Amend the most recent commit with a new message.
///
/// Any currently staged changes are included in the amended commit,
/// mirroring `git commit --amend -m <message>`.
///
/// # Arguments
/// - `message` – The replacement commit message.
///
/// # Returns
/// `Ok(())` on success, or an error string if `git commit --amend` fails.
#[tauri::command]
#[instrument(skip(state), name = "cmd::commit::amend")]
pub async fn amend_commit(message: String, state: State<'_, AppState>) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.amend_commit(&message).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return the commit message of the current HEAD commit.
///
/// Useful for pre-filling an amend dialog with the existing message.
///
/// # Returns
/// The raw commit message string, or an error string if HEAD cannot be
/// resolved.
#[tauri::command]
pub fn get_head_message(state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_head_message().map_err(|e| e.to_string())
    })
}
