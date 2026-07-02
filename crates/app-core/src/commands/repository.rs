//! Repository open/reload and basic metadata commands.

use std::path::PathBuf;

use tauri::{AppHandle, State};

use super::graph_cache::{GraphLayoutOptions, load_or_build_layout, ref_snapshot};
use super::helpers::*;
use crate::ipc_error::IpcError;
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
/// [`IpcError`] (`code = "repo_not_found"` when the path is not a valid git
/// repository, else a generic code) so the frontend gets a structured value.
#[tauri::command]
pub async fn open_repo(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<RepoInfo, IpcError> {
    let path_clone = path.clone();
    let config_dir = state.config_dir.clone();

    // Run the expensive graph computation off the main thread
    let (repo, layout, status, change_count, ref_snap) = tokio::task::spawn_blocking(move || {
        let repo =
            git_engine::Repository::open(PathBuf::from(&path_clone)).map_err(IpcError::from)?;

        let (layout, _was_cached) = load_or_build_layout(
            &repo,
            &path_clone,
            &config_dir,
            &GraphLayoutOptions::default(),
        )
        .map_err(IpcError::from)?;
        let status = repo.status().map_err(IpcError::from)?;
        // Compute the working-tree change count here, on the blocking thread,
        // instead of a second `file_statuses()` walk back on the async runtime.
        let change_count = repo.file_statuses().map(|s| s.len()).unwrap_or(0);
        // Baseline refs for the incremental graph-refresh fast path.
        let ref_snap = ref_snapshot(&repo);

        Ok::<_, IpcError>((repo, layout, status, change_count, ref_snap))
    })
    .await
    .map_err(|e| IpcError::new("internal", e.to_string()))??;

    // Start filesystem watcher for the new repo. The watcher now emits
    // `project-mutated` with `MutationKind::External` directly via the
    // mutation-events pipeline, so no manual `repo-changed` shim is needed.
    let repo_path = PathBuf::from(&path);
    let new_watcher = watcher::RepoWatcher::start(app_handle.clone(), repo_path)
        .inspect_err(|err| {
            // A swallowed start failure (e.g. the OS watch limit on a large
            // tree) silently disables real-time refresh for this repo with no
            // user-visible signal. Log it so a "changes don't appear live"
            // report is diagnosable from the log file.
            tracing::warn!(?err, path = %path, "repo watcher failed to start — real-time refresh disabled for this repo");
        })
        .ok();

    // Derive lightweight metadata
    let name = PathBuf::from(&path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    let head_branch = status.head_branch.clone();
    let is_worktree = repo.is_worktree();

    let slot = ProjectSlot {
        path: path.clone(),
        name,
        repo: Some(repo),
        layout: Some(layout),
        layout_options: GraphLayoutOptions::default(),
        watcher: new_watcher,
        head_branch,
        change_count,
        is_worktree,
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
    state.store_layout_ref_snapshot(&path, ref_snap);

    Ok(RepoInfo {
        path: status.path,
        head_branch: status.head_branch,
        head_oid: status.head_oid,
        branch_count: status.branch_count,
    })
}

/// Wholesale-clear the ahead/behind cache past this many entries. Keys are
/// `(tip, upstream_tip)` OID pairs that accumulate as branches move over a
/// session; clearing is safe — the next `get_branches` just recomputes.
const AHEAD_BEHIND_CACHE_CAP: usize = 8_192;

/// List all local and remote branches in the open repository.
///
/// Ahead/behind counts are served from [`AppState::ahead_behind_cache`], keyed
/// on `(branch_tip, upstream_tip)`, so a branch whose tips haven't moved since
/// the last call skips the `graph_ahead_behind` walk entirely — this command
/// fires on every `head_changed || refs_changed`, and most refs don't move.
#[tauri::command]
pub fn get_branches(state: State<'_, AppState>) -> Result<Vec<git_engine::BranchInfo>, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let active = state.active_index.lock().map_err(|e| e.to_string())?;
    let idx = active.ok_or_else(|| "No active project".to_string())?;
    let slot = projects
        .get(idx)
        .ok_or_else(|| "Active project index out of bounds".to_string())?;
    let repo = slot
        .repo
        .as_ref()
        .ok_or_else(|| "No repository open".to_string())?;
    let mut cache = state.ahead_behind_cache.lock().map_err(|e| e.to_string())?;
    if cache.len() > AHEAD_BEHIND_CACHE_CAP {
        cache.clear();
    }
    repo.branches_cached(&mut cache).map_err(|e| e.to_string())
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

/// Return [`RepoInfo`] (path + HEAD branch/OID + branch count) for the active
/// repository. Lets the mutation pipeline refresh `repoInfo` after a HEAD move
/// (e.g. a checkout to an existing branch) without re-opening the repo.
#[tauri::command]
pub fn get_repo_info(state: State<'_, AppState>) -> Result<RepoInfo, String> {
    with_active_repo(&state, |repo| {
        let status = repo.status().map_err(|e| e.to_string())?;
        Ok(RepoInfo {
            path: status.path,
            head_branch: status.head_branch,
            head_oid: status.head_oid,
            branch_count: status.branch_count,
        })
    })
}

/// List all configured remotes for the active repository.
#[tauri::command]
pub fn get_remotes(state: State<'_, AppState>) -> Result<Vec<RemoteInfo>, String> {
    with_active_repo(&state, collect_remotes)
}

/// Collect `RemoteInfo` for every configured remote of `repo`.
///
/// Extracted so it can be tested without the Tauri `State` plumbing.
pub(super) fn collect_remotes(repo: &git_engine::Repository) -> Result<Vec<RemoteInfo>, String> {
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
}

#[cfg(test)]
mod tests {
    use super::collect_remotes;
    use git_engine::Repository;
    use git_engine::test_support::{create_repo_with_branches, create_repo_with_n_commits};

    #[test]
    fn open_repo_on_valid_git_dir_returns_status() {
        let (_tmp, path) = create_repo_with_n_commits(2);
        let repo = Repository::open(&path).expect("open should succeed");
        let status = repo.status().unwrap();
        assert!(!status.is_empty);
        assert!(status.head_oid.is_some());
    }

    #[test]
    fn open_repo_on_non_git_dir_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        let err = Repository::open(dir.path()).err();
        assert!(
            err.is_some(),
            "opening a non-git directory must produce an error"
        );
    }

    #[test]
    fn get_branches_returns_branches_fixture() {
        let (_tmp, path) = create_repo_with_branches(&["feat-x", "feat-y"]);
        let repo = Repository::open(&path).unwrap();
        let branches = repo.branches().unwrap();
        for name in ["feat-x", "feat-y"] {
            assert!(
                branches.iter().any(|b| b.name == name),
                "missing expected branch {name}"
            );
        }
    }

    #[test]
    fn collect_remotes_on_repo_without_remotes_returns_empty() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let remotes = collect_remotes(&repo).unwrap();
        assert!(remotes.is_empty(), "fresh repo has no remotes");
    }

    #[test]
    fn collect_remotes_returns_origin_name_and_url() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        // Register a fake remote; no network traffic — git only writes config.
        let git_repo = git2::Repository::open(&path).unwrap();
        git_repo
            .remote("origin", "https://example.invalid/owner/repo.git")
            .unwrap();
        drop(git_repo);

        let repo = Repository::open(&path).unwrap();
        let remotes = collect_remotes(&repo).unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(
            remotes[0].url.as_deref(),
            Some("https://example.invalid/owner/repo.git")
        );
    }
}
