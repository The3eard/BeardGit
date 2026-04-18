//! Repository open/reload and basic metadata commands.

use std::path::PathBuf;

use graph_builder::{Dag, GraphCommit, GraphLayout};
use tauri::{AppHandle, Emitter, State};

use super::helpers::*;
use crate::state::{AppState, ProjectSlot};

/// Open a git repository at `path`, build the full commit DAG, and store the
/// result in [`AppState`] as a [`ProjectSlot`].
///
/// The heavy graph computation (commit walk, DAG build, layout) runs on a
/// blocking thread so the UI remains responsive on large repositories.
///
/// If the path is already open in an existing slot, that slot is replaced with
/// the freshly loaded data and made active. Otherwise a new slot is appended
/// and made active.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the repository root.
///
/// # Returns
/// [`RepoInfo`] with HEAD branch, HEAD OID, and branch count on success, or an
/// error string if the path is not a valid git repository.
#[tauri::command]
pub async fn open_repo(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<RepoInfo, String> {
    let path_clone = path.clone();

    // Run the expensive graph computation off the main thread
    let (repo, layout, status) = tokio::task::spawn_blocking(move || {
        let repo =
            git_engine::Repository::open(PathBuf::from(&path_clone)).map_err(|e| e.to_string())?;

        let commits = repo.walk_commits(50_000).map_err(|e| e.to_string())?;

        let graph_commits: Vec<GraphCommit> = commits
            .iter()
            .map(|c| GraphCommit {
                oid: c.oid.clone(),
                parents: c.parents.clone(),
                timestamp: c.timestamp,
                refs: c.refs.clone(),
                summary: c.summary.clone(),
                author: c.author.clone(),
                email: c.email.clone(),
            })
            .collect();

        let dag = Dag::build(&graph_commits);
        let layout = GraphLayout::compute(&dag);
        let status = repo.status().map_err(|e| e.to_string())?;

        Ok::<_, String>((repo, layout, status))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: String| e)?;

    // Start filesystem watcher for the new repo (emits `repo-changed` events)
    let repo_path = PathBuf::from(&path);
    let handle = app_handle.clone();
    let new_watcher = watcher::RepoWatcher::start(&repo_path, move || {
        let _ = handle.emit("repo-changed", ());
    })
    .ok();

    // Derive lightweight metadata
    let name = PathBuf::from(&path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    let head_branch = status.head_branch.clone();
    let change_count = repo.file_statuses().map(|s| s.len()).unwrap_or(0);

    let slot = ProjectSlot {
        path: path.clone(),
        name,
        repo: Some(repo),
        layout: Some(layout),
        watcher: new_watcher,
        head_branch,
        change_count,
    };

    // Insert or replace slot, then set as active
    let active_idx = {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        if let Some(pos) = projects.iter().position(|s| s.path == path) {
            projects[pos] = slot;
            pos
        } else {
            projects.push(slot);
            projects.len() - 1
        }
    };
    *state.active_index.lock().map_err(|e| e.to_string())? = Some(active_idx);
    invalidate_forge_provider_cache(&state);

    Ok(RepoInfo {
        path: status.path,
        head_branch: status.head_branch,
        head_oid: status.head_oid,
        branch_count: status.branch_count,
    })
}

/// List all local branches in the open repository with their HEAD OIDs.
#[tauri::command]
pub fn get_branches(state: State<'_, AppState>) -> Result<Vec<git_engine::BranchInfo>, String> {
    with_active_repo(&state, |repo| repo.branches().map_err(|e| e.to_string()))
}

/// Return the last N commits on a specific branch.
#[tauri::command]
pub fn get_branch_commits(
    branch_name: String,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitInfo>, String> {
    with_active_repo(&state, |repo| {
        repo.branch_commits(&branch_name, limit as usize)
            .map_err(|e| e.to_string())
    })
}

/// Return the working-tree and index status for every changed file.
///
/// Used to populate the staging area panel in the UI.
#[tauri::command]
pub fn get_file_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileStatus>, String> {
    with_active_repo(&state, |repo| {
        repo.file_statuses().map_err(|e| e.to_string())
    })
}

/// Starship-style status summary for the title bar.
#[tauri::command]
pub fn get_status_summary(state: State<'_, AppState>) -> Result<git_engine::StatusSummary, String> {
    with_active_repo(&state, |repo| {
        repo.status_summary().map_err(|e| e.to_string())
    })
}

/// List all configured remotes for the active repository.
#[tauri::command]
pub fn get_remotes(state: State<'_, AppState>) -> Result<Vec<RemoteInfo>, String> {
    with_active_repo(&state, |repo| {
        let git_repo = repo.inner();
        let remotes = git_repo.remotes().map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for name in remotes.iter().flatten() {
            let url = git_repo
                .find_remote(name)
                .ok()
                .and_then(|r| r.url().map(|u| u.to_string()));
            result.push(RemoteInfo {
                name: name.to_string(),
                url,
            });
        }
        Ok(result)
    })
}
