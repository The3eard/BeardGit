//! Remote management operations — rename and remove configured remotes.
//!
//! These operations shell out to the system `git` binary via [`Repository::git_cmd`]
//! because `libgit2` does not expose remote rename/delete in a stable cross-platform way.

use tracing::instrument;

use crate::error::GitError;
use crate::repository::Repository;

impl Repository {
    /// Renames a remote.
    ///
    /// Equivalent to `git remote rename <old_name> <new_name>`. Fails if
    /// `old_name` does not exist or `new_name` is already taken.
    #[instrument(skip(self), fields(old = %old_name, new = %new_name))]
    pub fn rename_remote(&self, old_name: &str, new_name: &str) -> Result<(), GitError> {
        let result = self.git_cmd(&["remote", "rename", old_name, new_name])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Removes a remote.
    ///
    /// Equivalent to `git remote remove <name>`. Fails if the remote does not exist.
    #[instrument(skip(self), fields(remote = %name))]
    pub fn remove_remote(&self, name: &str) -> Result<(), GitError> {
        let result = self.git_cmd(&["remote", "remove", name])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo_with_remote(dir: &std::path::Path) -> Repository {
        let git_repo = git2::Repository::init(dir).unwrap();
        git_repo
            .remote("origin", "https://github.com/test/repo.git")
            .unwrap();
        git_repo
            .remote("upstream", "https://github.com/upstream/repo.git")
            .unwrap();
        drop(git_repo);
        Repository::open(dir).unwrap()
    }

    #[test]
    fn test_rename_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_remote(tmp.path());
        repo.rename_remote("origin", "my-origin").unwrap();
        let git_repo = repo.inner();
        let remotes = git_repo.remotes().unwrap();
        let names: Vec<&str> = remotes.iter().flatten().collect();
        assert!(names.contains(&"my-origin"));
        assert!(!names.contains(&"origin"));
    }

    #[test]
    fn test_remove_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_remote(tmp.path());
        repo.remove_remote("upstream").unwrap();
        let git_repo = repo.inner();
        let remotes = git_repo.remotes().unwrap();
        let names: Vec<&str> = remotes.iter().flatten().collect();
        assert!(!names.contains(&"upstream"));
        assert!(names.contains(&"origin"));
    }

    #[test]
    fn test_rename_nonexistent_remote_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_remote(tmp.path());
        let result = repo.rename_remote("nonexistent", "new-name");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_nonexistent_remote_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_remote(tmp.path());
        let result = repo.remove_remote("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_to_existing_name_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_remote(tmp.path());
        // Both "origin" and "upstream" already exist — renaming origin to upstream should fail
        let result = repo.rename_remote("origin", "upstream");
        assert!(result.is_err());
    }
}
