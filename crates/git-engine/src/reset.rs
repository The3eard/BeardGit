//! Reset and amend operations.
//!
//! Extends [`Repository`] with `git reset` and `git commit --amend` support,
//! plus a helper to read the HEAD commit message for pre-filling amend UIs.

use crate::error::GitError;
use crate::repository::Repository;

impl Repository {
    /// Reset HEAD to a specific commit.
    ///
    /// # Parameters
    /// - `oid`  – Full or abbreviated SHA of the target commit.
    /// - `mode` – One of `"soft"`, `"mixed"`, or `"hard"`.
    ///
    /// # Errors
    /// Returns [`GitError::RepoNotFound`] for an unrecognised `mode` or when
    /// the underlying `git reset` invocation exits with a non-zero status.
    pub fn reset_to_commit(&self, oid: &str, mode: &str) -> Result<(), GitError> {
        let flag = match mode {
            "soft" => "--soft",
            "mixed" => "--mixed",
            "hard" => "--hard",
            _ => {
                return Err(GitError::RepoNotFound(format!(
                    "Invalid reset mode: {mode}"
                )));
            }
        };
        let result = self.git_cmd(&["reset", flag, oid])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::RepoNotFound(result.stderr))
        }
    }

    /// Amend the most recent commit with a new message.
    ///
    /// Any changes currently staged in the index are included in the amended
    /// commit, mirroring the behaviour of `git commit --amend -m <message>`.
    ///
    /// # Parameters
    /// - `message` – The replacement commit message.
    ///
    /// # Errors
    /// Returns [`GitError::RepoNotFound`] when `git commit --amend` exits
    /// with a non-zero status (e.g. nothing to amend, detached HEAD, etc.).
    pub fn amend_commit(&self, message: &str) -> Result<(), GitError> {
        let result = self.git_cmd(&["commit", "--amend", "-m", message])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::RepoNotFound(result.stderr))
        }
    }

    /// Return the commit message of the current HEAD commit.
    ///
    /// Useful for pre-filling an amend dialog with the existing message.
    ///
    /// # Errors
    /// Returns a [`GitError::Git`] when HEAD cannot be resolved or the commit
    /// object cannot be loaded.
    pub fn get_head_message(&self) -> Result<String, GitError> {
        let head = self.inner().head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.message().unwrap_or("").to_string())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::repository::Repository;

    /// Initialise a temporary git repository with two commits and return an
    /// open [`Repository`] pointed at it.
    fn init_repo_with_commits(dir: &std::path::Path) -> Repository {
        let git_repo = git2::Repository::init(dir).unwrap();
        {
            let mut config = git_repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();
        }

        // First commit
        let first_oid = {
            fs::write(dir.join("file.txt"), "first").unwrap();
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "first commit", &tree, &[])
                .unwrap()
        };

        // Second commit
        {
            fs::write(dir.join("file.txt"), "second").unwrap();
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            let first = git_repo.find_commit(first_oid).unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "second commit", &tree, &[&first])
                .unwrap();
        }

        drop(git_repo);
        Repository::open(dir).unwrap()
    }

    #[test]
    fn test_get_head_message() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commits(tmp.path());
        let msg = repo.get_head_message().unwrap();
        assert_eq!(msg.trim(), "second commit");
    }

    #[test]
    fn test_amend_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commits(tmp.path());
        repo.amend_commit("amended message").unwrap();
        let msg = repo.get_head_message().unwrap();
        assert_eq!(msg.trim(), "amended message");
    }

    #[test]
    fn test_reset_soft() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commits(tmp.path());
        let first_oid = {
            let head = repo.inner().head().unwrap().peel_to_commit().unwrap();
            head.parent(0).unwrap().id().to_string()
        };
        repo.reset_to_commit(&first_oid, "soft").unwrap();
        let msg = repo.get_head_message().unwrap();
        assert_eq!(msg.trim(), "first commit");
        // Working directory must still have the "second" content (soft reset
        // only moves HEAD; it does not touch the index or working tree).
        let content = fs::read_to_string(tmp.path().join("file.txt")).unwrap();
        assert_eq!(content, "second");
    }

    #[test]
    fn test_reset_hard() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commits(tmp.path());
        let first_oid = {
            let head = repo.inner().head().unwrap().peel_to_commit().unwrap();
            head.parent(0).unwrap().id().to_string()
        };
        repo.reset_to_commit(&first_oid, "hard").unwrap();
        let msg = repo.get_head_message().unwrap();
        assert_eq!(msg.trim(), "first commit");
        // Hard reset must restore the working-tree file to the first-commit content.
        let content = fs::read_to_string(tmp.path().join("file.txt")).unwrap();
        assert_eq!(content, "first");
    }

    #[test]
    fn test_reset_invalid_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commits(tmp.path());
        let result = repo.reset_to_commit("HEAD", "invalid");
        assert!(result.is_err());
    }
}
