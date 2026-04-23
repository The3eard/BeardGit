//! Multi-project tab management commands (open, close, switch, restore).

use std::path::PathBuf;

use rayon::prelude::*;

use tauri::{AppHandle, State};
use tracing::instrument;

use super::graph_cache::load_or_build_layout;
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
            is_worktree: existing.is_worktree,
        });
    }

    // Read lightweight metadata without building the graph
    let repo_path = PathBuf::from(&path);
    let temp_repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
    let status = temp_repo.status().map_err(|e| e.to_string())?;
    let change_count = temp_repo.file_statuses().map(|s| s.len()).unwrap_or(0);
    let is_worktree = temp_repo.is_worktree();

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
        is_worktree,
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
        is_worktree,
    })
}

/// Pure helper: adjust the active-project index after closing `closed_index`
/// from a list that had `prior_len` entries.
///
/// Mirrors the core logic of [`close_project`] without touching any state,
/// so it can be unit-tested in isolation.
pub(super) fn adjust_active_after_close(
    active: Option<usize>,
    closed_index: usize,
    prior_len: usize,
) -> Option<usize> {
    let new_len = prior_len.saturating_sub(1);
    if new_len == 0 {
        return None;
    }
    match active {
        None => None,
        Some(current) if current == closed_index => Some(closed_index.min(new_len - 1)),
        Some(current) if current > closed_index => Some(current - 1),
        other => other,
    }
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
    let prior_len = projects.len();
    projects.remove(index);

    // Adjust active index
    let previous_active = *active;
    *active = adjust_active_after_close(previous_active, index, prior_len);
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
    let config_dir = state.config_dir.clone();
    let (repo, layout, status) = tokio::task::spawn_blocking(move || {
        let repo =
            git_engine::Repository::open(PathBuf::from(&path_clone)).map_err(|e| e.to_string())?;
        let (layout, _was_cached) = load_or_build_layout(&repo, &path_clone, &config_dir)?;
        let status = repo.status().map_err(|e| e.to_string())?;
        Ok::<_, String>((repo, layout, status))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: String| e)?;

    // 4. Start filesystem watcher. The watcher emits `project-mutated`
    //    with `MutationKind::External` directly — no manual shim needed.
    let repo_path = PathBuf::from(&path);
    let new_watcher = watcher::RepoWatcher::start(app_handle.clone(), repo_path).ok();

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
            is_worktree: slot.is_worktree,
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
    let results: Vec<(String, String, Option<String>, usize, bool)> = paths
        .par_iter()
        .filter_map(|path| {
            let repo_path = PathBuf::from(path);
            let temp_repo = git_engine::Repository::open(repo_path).ok()?;
            let status = temp_repo.status().ok()?;
            let change_count = temp_repo.file_statuses().map(|s| s.len()).unwrap_or(0);
            let is_worktree = temp_repo.is_worktree();
            let name = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            Some((path.clone(), name, status.head_branch, change_count, is_worktree))
        })
        .collect();

    let mut valid_paths = Vec::new();
    let mut infos = Vec::new();

    // Sequential phase: populate the shared projects vec (must hold the mutex).
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;

        // Clear existing slots to prevent duplicates on repeated calls.
        projects.clear();

        for (path, name, head_branch, change_count, is_worktree) in results {
            let slot = ProjectSlot {
                path: path.clone(),
                name: name.clone(),
                repo: None,
                layout: None,
                watcher: None,
                head_branch: head_branch.clone(),
                change_count,
                is_worktree,
            };

            projects.push(slot);
            valid_paths.push(path.clone());

            infos.push(ProjectInfo {
                path,
                name,
                head_branch,
                change_count,
                is_worktree,
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
    Ok(filter_recent_repos(&config.recent_repos, &open_paths))
}

/// Pure helper: given a recent-repos list and a set of currently open paths,
/// return the recent entries not currently open, preserving order.
///
/// Factored out so the most-recent-first filtering logic can be unit-tested
/// without building an `AppState`.
pub(super) fn filter_recent_repos(recent: &[String], open: &[&String]) -> Vec<RecentRepo> {
    recent
        .iter()
        .filter(|r| !open.contains(r))
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
        .collect()
}

#[cfg(test)]
mod tests {
    use super::filter_recent_repos;

    #[test]
    fn recent_repos_preserves_insertion_order() {
        // The config module inserts most-recent-first with `.insert(0, path)`,
        // so the caller passes them in that order. The filter must preserve it.
        let recent = vec![
            "/home/adolfo/most-recent".to_string(),
            "/home/adolfo/older".to_string(),
            "/home/adolfo/oldest".to_string(),
        ];
        let open: Vec<&String> = vec![];
        let result = filter_recent_repos(&recent, &open);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].path, "/home/adolfo/most-recent");
        assert_eq!(result[0].name, "most-recent");
        assert_eq!(result[2].path, "/home/adolfo/oldest");
    }

    #[test]
    fn recent_repos_excludes_open_paths() {
        let recent = vec!["/a".to_string(), "/b".to_string(), "/c".to_string()];
        let open_b = "/b".to_string();
        let open: Vec<&String> = vec![&open_b];
        let result = filter_recent_repos(&recent, &open);
        assert_eq!(
            result.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
            vec!["/a".to_string(), "/c".to_string()]
        );
    }

    #[test]
    fn recent_repos_empty_input_returns_empty() {
        let recent: Vec<String> = vec![];
        let result = filter_recent_repos(&recent, &[]);
        assert!(result.is_empty());
    }

    use super::adjust_active_after_close;

    #[test]
    fn adjust_active_after_close_shifts_when_active_follows_closed() {
        // [A, B(active), C] → close A → active should shift from 1 to 0.
        assert_eq!(adjust_active_after_close(Some(1), 0, 3), Some(0));
    }

    #[test]
    fn adjust_active_after_close_picks_neighbour_when_active_closed() {
        // [A(active), B, C] → close A → new active should be 0 (was-B).
        assert_eq!(adjust_active_after_close(Some(0), 0, 3), Some(0));
        // [A, B(active), C] → close B → new active should clamp to 1 (was-C).
        assert_eq!(adjust_active_after_close(Some(1), 1, 3), Some(1));
        // [A, B(active)] → close B → new active should clamp to 0 (was-A).
        assert_eq!(adjust_active_after_close(Some(1), 1, 2), Some(0));
    }

    #[test]
    fn adjust_active_after_close_returns_none_when_empty() {
        // Closing the only tab leaves nothing active.
        assert_eq!(adjust_active_after_close(Some(0), 0, 1), None);
    }

    #[test]
    fn adjust_active_after_close_leaves_earlier_active_untouched() {
        // [A(active), B, C] → close C → active stays 0.
        assert_eq!(adjust_active_after_close(Some(0), 2, 3), Some(0));
    }
}
