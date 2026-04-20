//! Commit creation and amendment commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Create a new commit from the current index with the given message and author.
///
/// # Parameters
/// - `message` – Commit message (subject + optional body).
///
/// # Returns
/// The OID of the newly created commit as a hex string.
#[tauri::command]
#[instrument(skip(state), name = "cmd::commit::create")]
pub fn create_commit(message: String, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.create_commit(&message).map_err(|e| e.to_string())
    })
}

/// Amend the most recent commit with a new message.
///
/// Any currently staged changes are included in the amended commit,
/// mirroring `git commit --amend -m <message>`.
///
/// # Arguments
/// - `message` – The replacement commit message.
///
/// # Returns
/// `Ok(())` on success, or an error string if `git commit --amend` fails.
#[tauri::command]
#[instrument(skip(state), name = "cmd::commit::amend")]
pub async fn amend_commit(message: String, state: State<'_, AppState>) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.amend_commit(&message).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return the commit message of the current HEAD commit.
///
/// Useful for pre-filling an amend dialog with the existing message.
///
/// # Returns
/// The raw commit message string, or an error string if HEAD cannot be
/// resolved.
#[tauri::command]
pub fn get_head_message(state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_head_message().map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    //! Tests exercise the same `git_engine::Repository` call sites these
    //! commands delegate to; the `State`-wrapping outer layer is covered by
    //! `helpers.rs`/`graph.rs` and e2e tests.

    use git_engine::Repository;
    use git_engine::test_support::{create_repo_with_n_commits, create_repo_with_staged_changes};

    #[test]
    fn create_commit_with_staged_changes_succeeds() {
        let (_tmp, path) = create_repo_with_staged_changes(&[("hello.txt", "hello world\n")]);
        let repo = Repository::open(&path).unwrap();

        let oid = repo
            .create_commit("feat: hello")
            .expect("create_commit succeeds when index has staged files");

        // OID is a 40-char hex string (SHA-1).
        assert_eq!(oid.len(), 40, "OID should be a hex SHA-1 string");
        // HEAD message should match what we just committed.
        assert_eq!(repo.get_head_message().unwrap().trim(), "feat: hello");
    }

    #[test]
    fn get_head_message_on_empty_repo_errors() {
        // Fresh repo with no commits — there is no HEAD commit to read a
        // message from. Mirrors the command layer's error when invoked on an
        // empty repo tab.
        let dir = tempfile::TempDir::new().unwrap();
        let _ = git2::Repository::init(dir.path()).unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        assert!(
            repo.get_head_message().is_err(),
            "get_head_message should err when repo has no HEAD"
        );
    }

    #[test]
    fn amend_commit_on_headless_repo_errors() {
        // Brand-new init, no commits → no HEAD → `git commit --amend` fails.
        let dir = tempfile::TempDir::new().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        // Configure identity so the failure reason is "no HEAD" and not
        // "missing identity".
        let mut config = git_repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "t@t.io").unwrap();
        drop(config);
        drop(git_repo);

        let repo = Repository::open(dir.path()).unwrap();
        let err = repo.amend_commit("should fail").err();
        assert!(err.is_some(), "amend on HEAD-less repo should error");
    }

    #[test]
    fn get_head_message_returns_commit_subject() {
        let (_tmp, path) = create_repo_with_n_commits(2);
        let repo = Repository::open(&path).unwrap();
        let msg = repo.get_head_message().unwrap();
        assert!(msg.contains("Commit 2"), "expected 'Commit 2', got {msg:?}");
    }

    #[test]
    fn revert_commit_creates_inverse_commit() {
        // Build a repo with a concrete content change we can observe the
        // revert of.
        let (_tmp, path) = create_repo_with_staged_changes(&[("a.txt", "v1\n")]);
        let repo = Repository::open(&path).unwrap();
        let _ = repo.create_commit("add a.txt").unwrap();

        // Change a.txt and commit again — this is the commit we revert.
        std::fs::write(path.join("a.txt"), "v2\n").unwrap();
        repo.stage_files(&["a.txt".to_string()]).unwrap();
        let change_oid = repo.create_commit("bump a.txt").unwrap();

        // Revert the "bump" commit — the CLI produces a new HEAD whose tree
        // matches the pre-bump state.
        let result = repo.revert_commit(&change_oid).unwrap();
        assert!(
            result.success,
            "git revert should succeed, stderr: {}",
            result.stderr
        );
        let content = std::fs::read_to_string(path.join("a.txt")).unwrap();
        assert_eq!(content, "v1\n", "revert should restore pre-bump content");
    }
}
