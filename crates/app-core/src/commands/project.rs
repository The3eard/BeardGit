//! Multi-project tab management commands (open, close, switch, restore).

use std::path::PathBuf;

use rayon::prelude::*;

use graph_builder::{Dag, GraphCommit, GraphLayout};
use tauri::{AppHandle, Emitter, State};
use tracing::instrument;

use super::helpers::*;
use crate::state::{AppState, ProjectSlot};

/// Open a repo as a new tab with lightweight metadata only.
///
/// If the path is already open, returns its existing slot info without duplicating.
/// Does NOT fully load the repo (no graph, no watcher). Call [`switch_project`] to activate.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the repository root.
///
/// # Returns
/// [`ProjectInfo`] with lightweight metadata on success, or an error string if the path
/// is not a valid git repository.
#[tauri::command]
#[instrument(skip(state), name = "cmd::project::open")]
pub fn open_project(path: String, state: State<'_, AppState>) -> Result<ProjectInfo, String> {
    let mut projects = state.projects.lock().map_err(|e| e.to_string())?;

    // Check if already open
    if let Some(existing) = projects.iter().find(|p| p.path == path) {
        return Ok(ProjectInfo {
            path: existing.path.clone(),
            name: existing.name.clone(),
            head_branch: existing.head_branch.clone(),
            change_count: existing.change_count,
        });
    }

    // Read lightweight metadata without building the graph
    let repo_path = PathBuf::from(&path);
    let temp_repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
    let status = temp_repo.status().map_err(|e| e.to_string())?;
    let change_count = temp_repo.file_statuses().map(|s| s.len()).unwrap_or(0);

    let name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let slot = ProjectSlot {
        path: path.clone(),
        name: name.clone(),
        repo: None,
        layout: None,
        watcher: None,
        head_branch: status.head_branch.clone(),
        change_count,
    };

    projects.push(slot);

    // Persist to config
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        if !config.open_projects.contains(&path) {
            config.open_projects.push(path.clone());
        }
        config.recent_repos.retain(|r| r != &path);
        config.recent_repos.insert(0, path.clone());
        config.recent_repos.truncate(20);
        config.save(&state.config_path).map_err(|e| e.to_string())?;
    }

    Ok(ProjectInfo {
        path,
        name,
        head_branch: status.head_branch,
        change_count,
    })
}

/// Close a tab and remove it from the persisted list.
///
/// Adds the closed path to `recent_repos` (front, capped at 20). Adjusts the
/// active index if the closed tab was active or preceded the active tab.
///
/// # Parameters
/// - `index` – Zero-based index of the tab to close.
#[tauri::command]
#[instrument(skip(state), name = "cmd::project::close")]
pub fn close_project(index: usize, state: State<'_, AppState>) -> Result<(), String> {
    let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
    let mut active = state.active_index.lock().map_err(|e| e.to_string())?;

    if index >= projects.len() {
        return Err("Project index out of bounds".to_string());
    }

    let closed_path = projects[index].path.clone();
    projects.remove(index);

    // Adjust active index
    let previous_active = *active;
    if projects.is_empty() {
        *active = None;
    } else if let Some(current) = *active {
        if current == index {
            *active = Some(index.min(projects.len() - 1));
        } else if current > index {
            *active = Some(current - 1);
        }
    }
    let active_changed = previous_active != *active;

    // Persist to config
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.open_projects.retain(|p| p != &closed_path);
        config.active_project_index = *active;
        config.recent_repos.retain(|r| r != &closed_path);
        config.recent_repos.insert(0, closed_path);
        config.recent_repos.truncate(20);
        config.save(&state.config_path).map_err(|e| e.to_string())?;
    }

    // Drop the locks before invalidating the forge provider cache.
    drop(active);
    drop(projects);
    if active_changed {
        invalidate_forge_provider_cache(&state);
    }

    Ok(())
}

/// Switch the active tab. Unloads the previous project's heavy data,
/// loads the target project fully (repo, graph, watcher).
///
/// # Parameters
/// - `index` – Zero-based index of the tab to switch to.
///
/// # Returns
/// [`RepoInfo`] for the newly active repo on success.
#[tauri::command]
pub async fn switch_project(
    index: usize,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<RepoInfo, String> {
    // 1. Unload the previous active project's heavy data
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        if let Some(prev_idx) = *active
            && let Some(prev_slot) = projects.get_mut(prev_idx)
        {
            if let Some(repo) = &prev_slot.repo {
                if let Ok(status) = repo.status() {
                    prev_slot.head_branch = status.head_branch;
                }
                prev_slot.change_count = repo.file_statuses().map(|s| s.len()).unwrap_or(0);
            }
            prev_slot.repo = None;
            prev_slot.layout = None;
            prev_slot.watcher = None;
        }
    }

    // 2. Read the target path
    let path = {
        let projects = state.projects.lock().map_err(|e| e.to_string())?;
        let slot = projects
            .get(index)
            .ok_or_else(|| "Project index out of bounds".to_string())?;
        slot.path.clone()
    };

    // 3. Fully load the target project off-thread
    let path_clone = path.clone();
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

    // 4. Start filesystem watcher
    let repo_path = PathBuf::from(&path);
    let handle = app_handle.clone();
    let new_watcher = watcher::RepoWatcher::start(&repo_path, move || {
        let _ = handle.emit("repo-changed", ());
    })
    .ok();

    let change_count = repo.file_statuses().map(|s| s.len()).unwrap_or(0);

    // 5. Store in slot and update active index
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        if let Some(slot) = projects.get_mut(index) {
            slot.repo = Some(repo);
            slot.layout = Some(layout);
            slot.watcher = new_watcher;
            slot.head_branch = status.head_branch.clone();
            slot.change_count = change_count;
        }
        let mut active = state.active_index.lock().map_err(|e| e.to_string())?;
        *active = Some(index);
    }

    // 6. Persist active index
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.active_project_index = Some(index);
        config.save(&state.config_path).map_err(|e| e.to_string())?;
    }

    // 7. Re-detect active provider for the new repo
    detect_active_provider(&state).await;

    Ok(RepoInfo {
        path: status.path,
        head_branch: status.head_branch,
        head_oid: status.head_oid,
        branch_count: status.branch_count,
    })
}

/// Return lightweight metadata for all open tabs.
#[tauri::command]
pub fn get_open_projects(state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    Ok(projects
        .iter()
        .map(|slot| ProjectInfo {
            path: slot.path.clone(),
            name: slot.name.clone(),
            head_branch: slot.head_branch.clone(),
            change_count: slot.change_count,
        })
        .collect())
}

/// Return the index of the currently active project.
#[tauri::command]
pub fn get_active_project_index(state: State<'_, AppState>) -> Result<Option<usize>, String> {
    Ok(*state.active_index.lock().map_err(|e| e.to_string())?)
}

/// Restore persisted project tabs from config on app startup.
///
/// Opens each path in `config.open_projects` as a lightweight slot (no graph).
/// Invalid paths (deleted repos) are silently skipped and removed from config.
/// If called multiple times, existing slots are cleared first to prevent duplicates.
///
/// # Returns
/// A [`Vec<ProjectInfo>`] of all successfully restored projects.
#[tauri::command]
pub fn restore_projects(state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, String> {
    // Extract the paths from config then drop the lock immediately.
    let paths = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.open_projects.clone()
    };
    // config lock is dropped here.

    // Parallel phase: open repos and gather metadata (I/O-heavy, benefits from parallelism).
    // Invalid paths (deleted/moved repos) are silently dropped via `filter_map`.
    let results: Vec<(String, String, Option<String>, usize)> = paths
        .par_iter()
        .filter_map(|path| {
            let repo_path = PathBuf::from(path);
            let temp_repo = git_engine::Repository::open(repo_path).ok()?;
            let status = temp_repo.status().ok()?;
            let change_count = temp_repo.file_statuses().map(|s| s.len()).unwrap_or(0);
            let name = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            Some((path.clone(), name, status.head_branch, change_count))
        })
        .collect();

    let mut valid_paths = Vec::new();
    let mut infos = Vec::new();

    // Sequential phase: populate the shared projects vec (must hold the mutex).
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;

        // Clear existing slots to prevent duplicates on repeated calls.
        projects.clear();

        for (path, name, head_branch, change_count) in results {
            let slot = ProjectSlot {
                path: path.clone(),
                name: name.clone(),
                repo: None,
                layout: None,
                watcher: None,
                head_branch: head_branch.clone(),
                change_count,
            };

            projects.push(slot);
            valid_paths.push(path.clone());

            infos.push(ProjectInfo {
                path,
                name,
                head_branch,
                change_count,
            });
        }
    }
    // projects lock is dropped here before acquiring config again.

    // Update config to remove invalid paths.
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.open_projects = valid_paths;
        let _ = config.save(&state.config_path);
    }

    Ok(infos)
}

/// Return recent repos filtered to exclude already-open paths.
#[tauri::command]
pub fn get_recent_repos(state: State<'_, AppState>) -> Result<Vec<RecentRepo>, String> {
    // Acquire projects first, then config — consistent with the ordering used
    // elsewhere to avoid ABBA deadlocks.
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let open_paths: Vec<&String> = projects.iter().map(|p| &p.path).collect();

    Ok(config
        .recent_repos
        .iter()
        .filter(|r| !open_paths.contains(r))
        .map(|r| {
            let name = std::path::Path::new(r)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| r.clone());
            RecentRepo {
                path: r.clone(),
                name,
            }
        })
        .collect())
}
