//! Submodule listing, initialization, update, add, and remove commands.

use std::sync::Arc;

use task_runner::{TaskId, TaskManager};
use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// List all submodules in the active repository.
#[tauri::command]
pub fn list_submodules(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::SubmoduleInfo>, String> {
    with_active_repo(&state, |repo| {
        repo.list_submodules().map_err(|e| e.to_string())
    })
}

/// Initialize a submodule (register + set up working tree).
#[tauri::command]
#[instrument(skip(state), name = "cmd::submodule::init")]
pub fn init_submodule(path: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.init_submodule(&path).map_err(|e| e.to_string())
    })
}

/// Deinitialize a submodule.
#[tauri::command]
#[instrument(skip(state), name = "cmd::submodule::deinit")]
pub fn deinit_submodule(
    path: String,
    force: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.deinit_submodule(&path, force)
            .map_err(|e| e.to_string())
    })
}

/// Add a new submodule to the repository.
///
/// # Parameters
/// - `url` – Remote URL of the submodule repository.
/// - `path` – Relative path where the submodule will be placed.
#[tauri::command]
#[instrument(skip(state), name = "cmd::submodule::add")]
pub fn add_submodule(url: String, path: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.add_submodule(&url, &path).map_err(|e| e.to_string())
    })
}

/// Remove a submodule completely (deinit + rm).
///
/// # Parameters
/// - `path` – Relative path of the submodule to remove.
#[tauri::command]
#[instrument(skip(state), name = "cmd::submodule::remove")]
pub fn remove_submodule(path: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.remove_submodule(&path).map_err(|e| e.to_string())
    })
}

/// Get the absolute filesystem path of a submodule.
#[tauri::command]
pub fn submodule_abs_path(
    submodule_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.submodule_abs_path(&submodule_path)
            .map_err(|e| e.to_string())
    })
}

/// Update a single submodule (background task, returns TaskId).
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::submodule::update")]
pub async fn update_submodule(
    path: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Submodule update: {path}");
    let id = task_manager
        .spawn(
            label,
            "git",
            &["submodule", "update", "--init", "--recursive", "--", &path],
            &cwd,
            true,
        )
        .await;

    Ok(id)
}

/// Update all submodules (background task, returns TaskId).
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::submodule::update_all")]
pub async fn update_all_submodules(
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = "Submodule update: all".to_string();
    let id = task_manager
        .spawn(
            label,
            "git",
            &["submodule", "update", "--init", "--recursive"],
            &cwd,
            true,
        )
        .await;

    Ok(id)
}
