//! Patch creation, preview, and application commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Create patch files from one or more commits.
///
/// Returns the list of file paths created by `git format-patch`.
#[tauri::command]
pub fn create_commit_patches(
    oids: Vec<String>,
    output_dir: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    with_active_repo(&state, |repo| {
        repo.create_commit_patches(&oids, &output_dir)
            .map_err(|e| e.to_string())
    })
}

/// Create a patch from working tree changes.
///
/// Returns the raw patch text. Use the Tauri dialog to let the user
/// choose where to save it; the frontend writes the file.
#[tauri::command]
pub fn create_working_tree_patch(
    staged_only: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.create_working_tree_patch(staged_only)
            .map_err(|e| e.to_string())
    })
}

/// Preview a patch file (stats and clean-apply check).
#[tauri::command]
pub fn preview_patch(
    path: String,
    state: State<'_, AppState>,
) -> Result<git_engine::PatchPreview, String> {
    with_active_repo(&state, |repo| {
        repo.preview_patch(&path).map_err(|e| e.to_string())
    })
}

/// Save raw patch text to a file on disk.
///
/// Used by the frontend to write working-tree patches after the user
/// chooses a save location via the native dialog.
#[tauri::command]
pub fn save_patch_to_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Apply a patch file to the working tree.
///
/// When `three_way` is true, uses `--3way` for conflict-generating fallback.
#[tauri::command]
pub fn apply_patch(
    path: String,
    three_way: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.apply_patch_file(&path, three_way)
            .map_err(|e| e.to_string())
    })
}
