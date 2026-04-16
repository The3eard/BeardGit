//! Gitignore read, write, and pattern management commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Read the content of the repository's root `.gitignore` file.
///
/// Returns an empty string if the file does not exist.
#[tauri::command]
pub fn read_gitignore(state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.read_gitignore().map_err(|e| e.to_string())
    })
}

/// Write the full content of the repository's `.gitignore` file.
///
/// Creates the file if it does not exist.
#[tauri::command]
pub fn write_gitignore(content: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.write_gitignore(&content).map_err(|e| e.to_string())
    })
}

/// Add a single pattern to the repository's `.gitignore` file.
///
/// Checks for duplicates before appending. Creates the file if needed.
#[tauri::command]
pub fn add_gitignore_pattern(pattern: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.add_gitignore_pattern(&pattern)
            .map_err(|e| e.to_string())
    })
}
