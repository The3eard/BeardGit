//! Worktree listing, creation, and removal commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// List all worktrees for the active repository, including the main worktree.
///
/// Returns a [`WorktreeInfo`] for each worktree. The first element is always
/// the main worktree.
#[tauri::command]
pub async fn list_worktrees(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::WorktreeInfo>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.list_worktrees().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Create a new linked worktree at `path` on `branch`.
///
/// # Parameters
/// - `path` – Absolute filesystem path where the new worktree directory will be created.
/// - `branch` – Branch name to check out (or create when `create_branch` is `true`).
/// - `create_branch` – When `true`, create a new branch with `-b`; when `false`, check
///   out an existing branch.
#[tauri::command]
#[instrument(skip(state), name = "cmd::worktree::create")]
pub async fn create_worktree(
    path: String,
    branch: String,
    create_branch: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.create_worktree(&path, &branch, create_branch)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Remove a linked worktree at `path`.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the worktree directory to remove.
/// - `force` – When `true`, remove the worktree even if it has uncommitted changes
///   or is locked.
#[tauri::command]
#[instrument(skip(state), name = "cmd::worktree::remove")]
pub async fn remove_worktree(
    path: String,
    force: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.remove_worktree(&path, force)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Lock a linked worktree, preventing accidental removal.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the worktree directory.
/// - `reason` – Optional human-readable reason for the lock.
#[tauri::command]
pub async fn worktree_lock(
    path: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.lock_worktree(&path, reason.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Unlock a previously locked worktree.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the worktree directory.
#[tauri::command]
pub async fn worktree_unlock(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.unlock_worktree(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    //! Drive `Repository::*` worktree helpers against fixture repos.

    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn list_worktrees_on_fresh_repo_returns_main_only() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let worktrees = repo.list_worktrees().expect("list_worktrees");
        assert_eq!(
            worktrees.len(),
            1,
            "fresh repo should only expose the main worktree, got {worktrees:?}"
        );
        assert!(
            worktrees[0].is_main,
            "first worktree must be marked as main"
        );
    }

    #[test]
    fn create_worktree_adds_linked_worktree() {
        let (tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        // Place the new worktree as a sibling of the repo inside the same
        // tempdir so it's cleaned up automatically.
        let wt_path = tmp.path().join("wt-feature");
        let wt_str = wt_path.to_str().unwrap();
        repo.create_worktree(wt_str, "feature-one", true)
            .expect("create_worktree");

        let worktrees = repo.list_worktrees().unwrap();
        assert_eq!(
            worktrees.len(),
            2,
            "expected main + new worktree, got {worktrees:?}"
        );
        assert!(
            worktrees.iter().any(|w| !w.is_main),
            "at least one worktree should not be marked main"
        );
    }

    #[test]
    fn remove_worktree_on_missing_path_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = repo.remove_worktree("/no/such/path", false).err();
        assert!(
            err.is_some(),
            "remove_worktree should error when the path isn't a worktree"
        );
    }

    #[test]
    fn lock_unlock_roundtrip_on_linked_worktree() {
        let (tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        let wt_path = tmp.path().join("wt-lockme");
        let wt_str = wt_path.to_str().unwrap();
        repo.create_worktree(wt_str, "lock-branch", true).unwrap();

        repo.lock_worktree(wt_str, Some("under test"))
            .expect("lock_worktree should succeed");
        // Unlock reverses the lock — second unlock would error, so only call once.
        repo.unlock_worktree(wt_str)
            .expect("unlock_worktree after lock");
    }
}
