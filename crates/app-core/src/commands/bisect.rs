//! Bisect workflow commands.

use tauri::State;

use super::helpers::get_active_project_path;
use crate::state::AppState;

/// Start a bisect session, optionally providing the initial bad and good commits.
#[tauri::command]
pub async fn bisect_start(
    bad: Option<String>,
    good: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_start(&cwd, bad.as_deref(), good.as_deref())
}

/// Mark a commit (or current HEAD) as good.
#[tauri::command]
pub async fn bisect_good(
    commit: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_good(&cwd, commit.as_deref())
}

/// Mark a commit (or current HEAD) as bad.
#[tauri::command]
pub async fn bisect_bad(
    commit: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_bad(&cwd, commit.as_deref())
}

/// Skip the current commit.
#[tauri::command]
pub async fn bisect_skip(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_skip(&cwd)
}

/// Reset (end) the bisect session.
#[tauri::command]
pub async fn bisect_reset(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_reset(&cwd)
}

/// Get the current bisect session state.
#[tauri::command]
pub async fn bisect_get_state(
    state: State<'_, AppState>,
) -> Result<git_engine::BisectState, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_state(&cwd)
}

/// Get the bisect log.
#[tauri::command]
pub async fn bisect_get_log(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_log(&cwd)
}

/// Run an automated bisect with a test command.
#[tauri::command]
pub async fn bisect_run_auto(
    test_command: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_run(&cwd, &test_command)
}
