//! Multi-project tab management commands (open, close, switch, restore).

use std::path::PathBuf;

use rayon::prelude::*;

use tauri::{AppHandle, State};
use tracing::instrument;

use super::graph_cache::{GraphLayoutOptions, load_or_build_layout};
use super::helpers::*;
use crate::state::{AppState, ProjectSlot};

/// Tagged error returned by [`open_project`] so the frontend can branch
/// on `NotARepo` specifically and surface the init dialog instead of a
/// generic toast.
#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpenProjectError {
    /// The given path exists but is not (and is not inside) a git
    /// repository. The frontend uses this to offer "init repo here?".
    NotARepo {
        /// Echo of the path the user attempted to open, so the frontend
        /// can show it in the init dialog.
        path: String,
    },
    /// Any other failure (lock poisoning, I/O, libgit2 misbehaviour…).
    /// Surfaced as a generic toast on the frontend.
    Other {
        /// Human-readable message, suitable for a toast.
        message: String,
    },
}

impl OpenProjectError {
    /// Classify a `git_engine::Repository::open` failure. Anything that
    /// looks like "the path isn't a git repository" maps to
    /// [`Self::NotARepo`]; everything else is [`Self::Other`].
    pub fn from_open_error(path: &str, err: git_engine::GitError) -> Self {
        // Match case-insensitively because libgit2 and our own
        // `RepoNotFound` variant capitalise differently across versions.
        let msg_lc = err.to_string().to_lowercase();
        let looks_like_not_a_repo = msg_lc.contains("could not find repository")
            || msg_lc.contains("not a git repository")
            || msg_lc.contains("repository not found");
        if looks_like_not_a_repo {
            Self::NotARepo {
                path: path.to_string(),
            }
        } else {
            Self::Other {
                message: err.to_string(),
            }
        }
    }
}

/// Open a repo as a new tab with lightweight metadata only.
///
/// If the path is already open, returns its existing slot info without duplicating.
/// Does NOT fully load the repo (no graph, no watcher). Call [`switch_project`] to activate.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the repository root.
///
/// # Returns
/// [`ProjectInfo`] with lightweight metadata on success, or an [`OpenProjectError`]
/// distinguishing "not a git repository" (so the frontend can offer to init one)
/// from any other failure.
#[tauri::command]
#[instrument(skip(state), name = "cmd::project::open")]
pub fn open_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<ProjectInfo, OpenProjectError> {
    let mut projects = state.projects.lock().map_err(|e| OpenProjectError::Other {
        message: e.to_string(),
    })?;

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
    let temp_repo = git_engine::Repository::open(repo_path)
        .map_err(|e| OpenProjectError::from_open_error(&path, e))?;
    let status = temp_repo.status().map_err(|e| OpenProjectError::Other {
        message: e.to_string(),
    })?;
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
        layout_options: GraphLayoutOptions::default(),
        watcher: None,
        head_branch: status.head_branch.clone(),
        change_count,
        is_worktree,
    };

    projects.push(slot);
    // Release the `projects` lock before taking `config` — AppState's
    // one-mutex-at-a-time invariant (see CLAUDE.md) forbids holding two.
    drop(projects);

    // Persist to config
    {
        let mut config = state.config.lock().map_err(|e| OpenProjectError::Other {
            message: e.to_string(),
        })?;
        if !config.open_projects.contains(&path) {
            config.open_projects.push(path.clone());
        }
        config.recent_repos.retain(|r| r != &path);
        config.recent_repos.insert(0, path.clone());
        config.recent_repos.truncate(20);
        config
            .save(&state.config_path)
            .map_err(|e| OpenProjectError::Other {
                message: e.to_string(),
            })?;
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

/// Pure helper: adjust the active-project index after moving a project
/// from `from` to `to` (remove-then-insert semantics), keeping the active
/// index pointed at the same project.
///
/// Mirrors the core logic of [`reorder_project`] without touching any
/// state, so it can be unit-tested in isolation.
pub(super) fn adjust_active_after_move(
    active: Option<usize>,
    from: usize,
    to: usize,
) -> Option<usize> {
    match active {
        None => None,
        Some(current) if current == from => Some(to),
        Some(current) => {
            let after_remove = if current > from { current - 1 } else { current };
            let final_idx = if after_remove >= to {
                after_remove + 1
            } else {
                after_remove
            };
            Some(final_idx)
        }
    }
}

/// Reorder an open project tab, moving it from one index to another.
///
/// Moves the project within both the in-memory list and the persisted
/// `open_projects` order (remove at `from`, insert at `to`), keeping the
/// active index pointed at the same project, then persists the new order.
///
/// Tab order is presentation state, so this does NOT go through a
/// [`mutation_events::MutationGuard`] — no repo state changes.
///
/// # Parameters
/// - `from` – Current zero-based index of the project to move.
/// - `to` – Destination zero-based index.
#[tauri::command]
#[instrument(skip(state), name = "cmd::project::reorder")]
pub fn reorder_project(from: usize, to: usize, state: State<'_, AppState>) -> Result<(), String> {
    if from == to {
        return Ok(());
    }

    // 1. Reorder the in-memory project list.
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        if from >= projects.len() || to >= projects.len() {
            return Err("Project index out of bounds".to_string());
        }
        let slot = projects.remove(from);
        projects.insert(to, slot);
    }

    // 2. Track the active project through the move.
    let new_active = {
        let mut active = state.active_index.lock().map_err(|e| e.to_string())?;
        *active = adjust_active_after_move(*active, from, to);
        *active
    };

    // 3. Persist the new order + active index.
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        if from < config.open_projects.len() && to < config.open_projects.len() {
            let path = config.open_projects.remove(from);
            config.open_projects.insert(to, path);
        }
        config.active_project_index = new_active;
        config.save(&state.config_path).map_err(|e| e.to_string())?;
    }

    Ok(())
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
    // Each AppState mutex is taken in its own scope so that no two are ever
    // held simultaneously — the `with_active_repo` chain is the only reason
    // the lock ordering elsewhere is deadlock-safe, and any divergence here
    // would invalidate that invariant.

    // 1. Mutate projects and capture the data the next two scopes need.
    let (closed_path, prior_len) = {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        if index >= projects.len() {
            return Err("Project index out of bounds".to_string());
        }
        let closed_path = projects[index].path.clone();
        let prior_len = projects.len();
        projects.remove(index);
        (closed_path, prior_len)
    };

    // 2. Recompute the active index without holding the projects lock.
    let (active_changed, new_active) = {
        let mut active = state.active_index.lock().map_err(|e| e.to_string())?;
        let previous_active = *active;
        *active = adjust_active_after_close(previous_active, index, prior_len);
        (previous_active != *active, *active)
    };

    // 3. Persist to config.
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.open_projects.retain(|p| p != &closed_path);
        config.active_project_index = new_active;
        config.recent_repos.retain(|r| r != &closed_path);
        config.recent_repos.insert(0, closed_path);
        config.recent_repos.truncate(20);
        config.save(&state.config_path).map_err(|e| e.to_string())?;
    }

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
    // 1. Read the previous active index. We DON'T unload it yet — unloading
    //    before the target finishes loading would strand the previous tab with
    //    `repo = None` if the load fails (active_index would still point at it).
    //    The unload happens in step 5, only after a successful load.
    let prev_idx = {
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        *active
    };

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
        let (layout, _was_cached) = load_or_build_layout(
            &repo,
            &path_clone,
            &config_dir,
            &GraphLayoutOptions::default(),
        )?;
        let status = repo.status().map_err(|e| e.to_string())?;
        Ok::<_, String>((repo, layout, status))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: String| e)?;

    // 4. Start filesystem watcher. The watcher emits `project-mutated`
    //    with `MutationKind::External` directly — no manual shim needed.
    let repo_path = PathBuf::from(&path);
    let new_watcher = watcher::RepoWatcher::start(app_handle.clone(), repo_path)
        .inspect_err(|err| {
            // See repository.rs: don't swallow watcher-start failures silently
            // or "changes don't appear live" becomes undiagnosable.
            tracing::warn!(?err, path = %path, "repo watcher failed to start — real-time refresh disabled for this repo");
        })
        .ok();

    let change_count = repo.file_statuses().map(|s| s.len()).unwrap_or(0);

    // 5. Target loaded successfully. Now (and only now) unload the previous
    //    tab's heavy state — capturing its lightweight metadata first — and
    //    install the target, both under a single `projects` lock. Deferring
    //    the unload to here means a failed load above leaves the previous tab
    //    fully intact instead of stranding it with `repo = None`.
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        if let Some(prev_idx) = prev_idx
            && prev_idx != index
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
        if let Some(slot) = projects.get_mut(index) {
            slot.repo = Some(repo);
            slot.layout = Some(layout);
            slot.layout_options = GraphLayoutOptions::default();
            slot.watcher = new_watcher;
            slot.head_branch = status.head_branch.clone();
            slot.change_count = change_count;
        }
    }
    {
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
            Some((
                path.clone(),
                name,
                status.head_branch,
                change_count,
                is_worktree,
            ))
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
                layout_options: GraphLayoutOptions::default(),
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
    // Snapshot the open paths under the `projects` lock, drop it, then take
    // `config` separately. Holding both locks at once would break the no-two-
    // locks-simultaneously invariant that the rest of `app-core` relies on.
    let open_paths: Vec<String> = {
        let projects = state.projects.lock().map_err(|e| e.to_string())?;
        projects.iter().map(|p| p.path.clone()).collect()
    };
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let open_path_refs: Vec<&String> = open_paths.iter().collect();
    Ok(filter_recent_repos(&config.recent_repos, &open_path_refs))
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
    use super::adjust_active_after_move;

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

    #[test]
    fn adjust_active_after_move_follows_the_moved_project() {
        // [A(active), B, C] → move A to index 2 → active follows to 2.
        assert_eq!(adjust_active_after_move(Some(0), 0, 2), Some(2));
        // [A, B, C(active)] → move C to index 0 → active follows to 0.
        assert_eq!(adjust_active_after_move(Some(2), 2, 0), Some(0));
    }

    #[test]
    fn adjust_active_after_move_shifts_bystander_projects() {
        // [A, B(active), C, D] → move A (from 0) to index 3.
        // Result [B, C, D, A] → B is now at index 0.
        assert_eq!(adjust_active_after_move(Some(1), 0, 3), Some(0));
        // [A, B, C, D(active)] → move D (from 3) to index 0.
        // Result [D, A, B, C] → the was-C active shifts 2 → 3.
        assert_eq!(adjust_active_after_move(Some(2), 3, 0), Some(3));
        // No active project stays None.
        assert_eq!(adjust_active_after_move(None, 0, 2), None);
    }
}

#[cfg(test)]
mod open_project_error_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn not_a_repo_serialises_with_path() {
        let err = OpenProjectError::NotARepo {
            path: "/tmp/foo".into(),
        };
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v, json!({ "kind": "not_a_repo", "path": "/tmp/foo" }));
    }

    #[test]
    fn other_serialises_with_message() {
        let err = OpenProjectError::Other {
            message: "boom".into(),
        };
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v, json!({ "kind": "other", "message": "boom" }));
    }

    #[test]
    fn classify_missing_dotgit_is_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let err = git_engine::Repository::open(&path).err().unwrap();
        let classified = OpenProjectError::from_open_error(&path, err);
        match classified {
            OpenProjectError::NotARepo { path: echoed } => assert_eq!(echoed, path),
            other => panic!("expected NotARepo, got {other:?}"),
        }
    }
}
