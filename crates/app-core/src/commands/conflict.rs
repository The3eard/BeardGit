//! Conflict detection, resolution, abort, and continue commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Return the current conflict state and list of conflicted file paths.
#[tauri::command]
pub fn get_conflict_status(
    state: State<'_, AppState>,
) -> Result<git_engine::ConflictStatus, String> {
    with_active_repo(&state, |repo| {
        repo.conflict_status().map_err(|e| e.to_string())
    })
}

/// Get the ours/theirs/base content of a conflicted file from the index.
#[tauri::command]
pub fn get_conflict_file_contents(
    path: String,
    state: State<'_, AppState>,
) -> Result<git_engine::ConflictFileContents, String> {
    with_active_repo(&state, |repo| {
        repo.get_conflict_file_contents(&path)
            .map_err(|e| e.to_string())
    })
}

/// Write resolved content to disk and mark the file as resolved in the index.
#[tauri::command]
pub fn write_resolved_file(
    path: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.write_resolved_file(&path, &content)
            .map_err(|e| e.to_string())
    })
}

/// Abort the current mid-operation git state (merge/rebase/cherry-pick/revert).
#[tauri::command]
pub async fn abort_operation(state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let conflict_state = repo.detect_conflict_state();
        let result = match conflict_state {
            git_engine::ConflictState::Merging => repo.abort_merge().map_err(|e| e.to_string())?,
            git_engine::ConflictState::Rebasing => {
                repo.abort_rebase().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::CherryPicking => {
                repo.abort_cherry_pick().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Reverting => {
                repo.abort_revert().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::None => {
                return Err("No operation in progress to abort".to_string());
            }
        };
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Continue the current mid-operation git state after conflicts are resolved.
#[tauri::command]
pub async fn continue_operation(state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let status = repo.conflict_status().map_err(|e| e.to_string())?;
        if status.state == git_engine::ConflictState::None {
            return Err("No operation in progress to continue".to_string());
        }
        if !status.can_continue {
            return Err("Cannot continue: unresolved conflicts remain".to_string());
        }
        let result = match status.state {
            git_engine::ConflictState::Merging => {
                repo.continue_merge().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Rebasing => {
                repo.continue_rebase().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::CherryPicking => {
                repo.continue_cherry_pick().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Reverting => {
                repo.continue_revert().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::None => unreachable!(),
        };
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
