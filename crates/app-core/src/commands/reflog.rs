//! Reflog entry listing commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Return the HEAD reflog entries, limited to the given count (default 100).
#[tauri::command]
pub async fn get_reflog(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::ReflogEntry>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.get_reflog(limit.unwrap_or(100))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
