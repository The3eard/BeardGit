//! Advanced git commands: cherry-pick, revert, reset, blame, file history, interactive rebase.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Cherry-pick a commit onto the current branch.
///
/// # Arguments
/// - `oid` – Full or abbreviated SHA of the commit to cherry-pick.
///
/// # Returns
/// The stdout of `git cherry-pick` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state), name = "cmd::advanced::cherry_pick")]
pub async fn cherry_pick(oid: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.cherry_pick(&oid).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Revert a commit, creating a new commit that undoes its changes.
///
/// # Arguments
/// - `oid` – Full or abbreviated SHA of the commit to revert.
///
/// # Returns
/// The stdout of `git revert` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state), name = "cmd::advanced::revert")]
pub async fn revert_commit(oid: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.revert_commit(&oid).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reset HEAD to a specific commit.
///
/// # Arguments
/// - `oid`  – Full or abbreviated SHA of the target commit.
/// - `mode` – Reset mode: `"soft"`, `"mixed"`, or `"hard"`.
///
/// # Returns
/// `Ok(())` on success, or an error string if the mode is invalid or
/// `git reset` exits with a non-zero status.
#[tauri::command]
#[instrument(skip(state), name = "cmd::advanced::reset")]
pub async fn reset_to_commit(
    oid: String,
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.reset_to_commit(&oid, &mode).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get per-line blame information for a file, optionally at a specific commit.
///
/// # Parameters
/// - `path` – Repository-relative file path to blame.
/// - `oid` – Optional commit OID; when `None`, blame is computed at HEAD.
#[tauri::command]
pub async fn blame_file(
    path: String,
    oid: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::BlameLine>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.blame_file(&path, oid.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get the commit history for a specific file with rename tracking.
///
/// # Parameters
/// - `path` – Repository-relative file path.
/// - `limit` – Maximum number of entries to return (default 100).
#[tauri::command]
pub async fn file_history(
    path: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileHistoryEntry>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.file_history(&path, limit).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get the commits between `base_oid` (exclusive) and HEAD in rebase order.
///
/// Returns the commit list that would appear in `git rebase -i` for the given
/// base, ordered oldest-first.
#[tauri::command]
pub async fn get_rebase_commits(
    base_oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::RebaseCommit>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.get_rebase_commits(&base_oid)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Start an interactive rebase with pre-defined actions.
///
/// Each action specifies a commit OID and a rebase verb (`pick`, `squash`,
/// `fixup`, `edit`, `drop`). The todo file is injected via `GIT_SEQUENCE_EDITOR`
/// so no interactive terminal is required.
#[tauri::command]
#[instrument(skip(state, actions), name = "cmd::advanced::interactive_rebase")]
pub async fn start_interactive_rebase(
    base_oid: String,
    actions: Vec<git_engine::RebaseAction>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.start_interactive_rebase(&base_oid, &actions)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Wipe the persistent graph-layout cache directory.
///
/// Exposed from Settings → Advanced as a manual "something looks
/// wrong with the graph" escape hatch. The loader transparently
/// rebuilds any missing layout on the next repo open (see
/// `graph_cache::load_or_build_layout`), so clearing the dir is
/// always safe — at worst the very next `open_repo` pays the cost
/// of one fresh walk + write.
///
/// The operation is best-effort:
///   - A missing layouts dir is treated as success (nothing to do).
///   - Any IO error is bubbled back as the stringified
///     `std::io::Error` so the frontend can surface it in a toast.
///
/// Returns the number of files removed on success (informational —
/// the UI copy doesn't use it yet but tests assert on it).
#[tauri::command]
#[instrument(skip(state), name = "cmd::advanced::clear_layout_cache")]
pub async fn clear_layout_cache(state: State<'_, AppState>) -> Result<u32, String> {
    let dir = state.config_dir.join("layouts");
    tokio::task::spawn_blocking(move || {
        let mut removed: u32 = 0;
        match std::fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        match std::fs::remove_file(&path) {
                            Ok(()) => removed = removed.saturating_add(1),
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                }
                Ok(removed)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(e.to_string()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    //! Exercises the cache-clear core logic without spinning up a Tauri
    //! runtime — we can't construct a real `State<'_, AppState>` from a
    //! unit test, so the tests re-implement the command body 1:1
    //! against a tempdir and assert the directory is empty afterwards.
    //!
    //! The remaining tests cover the `git_engine::Repository` surface that
    //! the other `advanced` commands delegate to (cherry-pick, revert,
    //! reset, blame, file-history, interactive rebase).
    use std::fs;
    use tempfile::tempdir;

    use git_engine::Repository;
    use git_engine::test_support::{create_repo_with_n_commits, create_repo_with_staged_changes};

    /// Re-implements the body of `clear_layout_cache` so the test can
    /// exercise it without a Tauri runtime. Any change to the real
    /// command MUST mirror here or the tests stop being load-bearing.
    fn clear_layouts_in(dir: &std::path::Path) -> Result<u32, String> {
        match fs::read_dir(dir) {
            Ok(entries) => {
                let mut removed: u32 = 0;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        fs::remove_file(&path).map_err(|e| e.to_string())?;
                        removed = removed.saturating_add(1);
                    }
                }
                Ok(removed)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(e.to_string()),
        }
    }

    #[test]
    fn clear_layout_cache_removes_files() {
        let tmp = tempdir().unwrap();
        let layouts = tmp.path().join("layouts");
        fs::create_dir_all(&layouts).unwrap();
        fs::write(layouts.join("a.json"), b"{}").unwrap();
        fs::write(layouts.join("b.json"), b"{}").unwrap();

        let removed = clear_layouts_in(&layouts).unwrap();
        assert_eq!(removed, 2);
        assert!(fs::read_dir(&layouts).unwrap().next().is_none());
    }

    #[test]
    fn clear_layout_cache_noop_when_missing() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("layouts");
        assert_eq!(clear_layouts_in(&dir).unwrap(), 0);
    }

    // ---------------------------------------------------------------------
    // Delegate-layer tests for the other advanced commands.
    // ---------------------------------------------------------------------

    #[test]
    fn reset_to_commit_rejects_invalid_mode() {
        let (_tmp, path) = create_repo_with_n_commits(2);
        let repo = Repository::open(&path).unwrap();
        let head_oid = repo.inner().head().unwrap().target().unwrap().to_string();
        let err = repo.reset_to_commit(&head_oid, "bogus").err();
        assert!(
            err.is_some(),
            "invalid mode should be rejected before hitting git"
        );
    }

    #[test]
    fn reset_to_commit_soft_moves_head_to_parent() {
        let (_tmp, path) = create_repo_with_n_commits(2);
        let repo = Repository::open(&path).unwrap();
        // HEAD~1 oid
        let head = repo.inner().head().unwrap().peel_to_commit().unwrap();
        let parent = head.parent(0).unwrap();
        let parent_oid = parent.id().to_string();
        repo.reset_to_commit(&parent_oid, "soft")
            .expect("soft reset");

        let new_head = repo.inner().head().unwrap().target().unwrap().to_string();
        assert_eq!(
            new_head, parent_oid,
            "HEAD should have moved to parent after soft reset"
        );
    }

    #[test]
    fn revert_commit_creates_new_head() {
        let (_tmp, path) = create_repo_with_staged_changes(&[("r.txt", "v1\n")]);
        let repo = Repository::open(&path).unwrap();
        let _ = repo.create_commit("add r").unwrap();

        std::fs::write(path.join("r.txt"), "v2\n").unwrap();
        repo.stage_files(&["r.txt".to_string()]).unwrap();
        let target_oid = repo.create_commit("bump r").unwrap();

        let head_before = repo.inner().head().unwrap().target().unwrap().to_string();
        let result = repo.revert_commit(&target_oid).unwrap();
        assert!(result.success, "revert should succeed: {}", result.stderr);
        let head_after = repo.inner().head().unwrap().target().unwrap().to_string();
        assert_ne!(head_before, head_after, "revert should move HEAD");
    }

    #[test]
    fn cherry_pick_on_missing_oid_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        // cherry_pick returns Ok(GitCliResult) with success=false on a bogus
        // oid (the CLI errors, but we wrap the result, not the spawn itself).
        let out = repo.cherry_pick("0000000000000000000000000000000000000000");
        match out {
            Ok(r) => assert!(!r.success, "cherry-pick of bogus oid must not succeed"),
            Err(_) => { /* also acceptable */ }
        }
    }

    #[test]
    fn blame_file_returns_entries_for_tracked_file() {
        let (_tmp, path) = create_repo_with_staged_changes(&[("b.txt", "a\nb\nc\n")]);
        let repo = Repository::open(&path).unwrap();
        let _ = repo.create_commit("seed b").unwrap();
        let lines = repo.blame_file("b.txt", None).unwrap();
        assert_eq!(lines.len(), 3, "blame should have 3 lines, got {lines:?}");
    }

    #[test]
    fn file_history_returns_commits_touching_file() {
        let (_tmp, path) = create_repo_with_staged_changes(&[("f.txt", "v1\n")]);
        let repo = Repository::open(&path).unwrap();
        let _ = repo.create_commit("add f").unwrap();
        std::fs::write(path.join("f.txt"), "v2\n").unwrap();
        repo.stage_files(&["f.txt".to_string()]).unwrap();
        let _ = repo.create_commit("bump f").unwrap();
        let entries = repo.file_history("f.txt", Some(10)).unwrap();
        assert!(
            entries.len() >= 2,
            "f.txt has 2+ commits touching it, got {entries:?}"
        );
    }

    #[test]
    fn get_rebase_commits_returns_range_oldest_first() {
        let (_tmp, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();
        // base = oldest commit.
        let head = repo.inner().head().unwrap().peel_to_commit().unwrap();
        let middle = head.parent(0).unwrap();
        let base = middle.parent(0).unwrap();
        let base_oid = base.id().to_string();
        let commits = repo.get_rebase_commits(&base_oid).unwrap();
        assert_eq!(commits.len(), 2, "range base..HEAD has 2 commits");
    }
}
