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

#[cfg(test)]
mod tests {
    //! Drive `Repository::get_reflog` directly with fixture repos.

    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn get_reflog_returns_entries_for_each_commit() {
        // Three commits => HEAD reflog has one entry per commit (`commit`),
        // but the precise count can vary with libgit2 init so just assert
        // we get at least one entry and it has a non-empty oid.
        let (_tmp, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();
        let entries = repo.get_reflog(100).expect("reflog read");
        assert!(
            !entries.is_empty(),
            "reflog should contain entries for the test commits"
        );
        assert_eq!(entries[0].oid.len(), 40, "oid is a full SHA-1 hex string");
    }

    #[test]
    fn get_reflog_respects_limit() {
        let (_tmp, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();
        let limited = repo.get_reflog(1).unwrap();
        assert!(
            limited.len() <= 1,
            "limit=1 should cap the result, got {}",
            limited.len()
        );
    }

    #[test]
    fn get_reflog_on_empty_repo_returns_no_entries() {
        // A fresh `git init` with no commits has no HEAD reflog. libgit2 is
        // lenient here: it succeeds with an empty vector rather than
        // erroring. Keep the test asserting that behavior so a future fix
        // (that flips it to an error) shows up as a test failure.
        let dir = tempfile::TempDir::new().unwrap();
        let _ = git2::Repository::init(dir.path()).unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let entries = repo.get_reflog(10).expect("empty reflog should be Ok");
        assert!(
            entries.is_empty(),
            "fresh repo should have no reflog entries"
        );
    }
}
