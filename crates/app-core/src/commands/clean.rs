//! Clean (untracked/ignored file removal) commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Preview untracked/ignored files that would be removed by `git clean`.
#[tauri::command]
pub fn clean_dry_run(
    include_directories: bool,
    include_ignored: bool,
    only_ignored: bool,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CleanItem>, String> {
    with_active_repo(&state, |repo| {
        repo.clean_dry_run(include_directories, include_ignored, only_ignored)
            .map_err(|e| e.to_string())
    })
}

/// Permanently remove the specified paths from the working directory.
///
/// Returns the number of items successfully deleted.
#[tauri::command]
pub fn clean_paths(paths: Vec<String>, state: State<'_, AppState>) -> Result<u32, String> {
    with_active_repo(&state, |repo| {
        repo.clean_paths(&paths).map_err(|e| e.to_string())
    })
}
