//! Staging and unstaging commands (index manipulation).

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Stage a specific list of files by path (equivalent to `git add <paths>`).
///
/// # Parameters
/// - `paths` – Workspace-relative paths to stage.
#[tauri::command]
#[instrument(skip(state), name = "cmd::staging::stage_files")]
pub fn stage_files(paths: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.stage_files(&paths).map_err(|e| e.to_string())
    })
}

/// Unstage a specific list of files (equivalent to `git restore --staged <paths>`).
///
/// # Parameters
/// - `paths` – Workspace-relative paths to unstage.
#[tauri::command]
#[instrument(skip(state), name = "cmd::staging::unstage_files")]
pub fn unstage_files(paths: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.unstage_files(&paths).map_err(|e| e.to_string())
    })
}

/// Stage all modified and untracked files (equivalent to `git add -A`).
#[tauri::command]
#[instrument(skip(state), name = "cmd::staging::stage_all")]
pub fn stage_all(state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| repo.stage_all().map_err(|e| e.to_string()))
}

/// Unstage all staged changes (equivalent to `git restore --staged .`).
#[tauri::command]
#[instrument(skip(state), name = "cmd::staging::unstage_all")]
pub fn unstage_all(state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| repo.unstage_all().map_err(|e| e.to_string()))
}

/// Stage selected hunks or individual lines from the working directory.
///
/// # Parameters
/// - `path` – Workspace-relative file path.
/// - `selections` – Which hunks/lines to stage.
#[tauri::command]
#[instrument(skip(state), name = "cmd::staging::stage_hunks")]
pub fn stage_hunks(
    path: String,
    selections: Vec<git_engine::HunkSelection>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.stage_hunks(&path, &selections)
            .map_err(|e| e.to_string())
    })
}

/// Unstage selected hunks or individual lines from the index.
///
/// # Parameters
/// - `path` – Workspace-relative file path.
/// - `selections` – Which hunks/lines to unstage.
#[tauri::command]
#[instrument(skip(state), name = "cmd::staging::unstage_hunks")]
pub fn unstage_hunks(
    path: String,
    selections: Vec<git_engine::HunkSelection>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.unstage_hunks(&path, &selections)
            .map_err(|e| e.to_string())
    })
}

/// Discard selected hunks or individual lines from the working directory.
///
/// # Parameters
/// - `path` – Workspace-relative file path.
/// - `selections` – Which hunks/lines to discard.
#[tauri::command]
#[instrument(skip(state), name = "cmd::staging::discard_hunks")]
pub fn discard_hunks(
    path: String,
    selections: Vec<git_engine::HunkSelection>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.discard_hunks(&path, &selections)
            .map_err(|e| e.to_string())
    })
}
