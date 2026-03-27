//! High-level git write operations: committing, branching, and checking out.
//!
//! Extends [`Repository`] with methods that modify repository state using
//! `libgit2`. For operations that `libgit2` cannot handle (merge, rebase,
//! push, etc.) see the [`cli`](crate::cli) module.

// Operations module — commit, branch, checkout

use crate::error::GitError;
use crate::repository::Repository;

impl Repository {
    /// Create a commit from the current index state.
    pub fn create_commit(
        &self,
        message: &str,
        name: &str,
        email: &str,
    ) -> Result<String, GitError> {
        let repo = self.inner();
        let sig = git2::Signature::now(name, email)?;
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
        Ok(oid.to_string())
    }

    /// Create a new branch at HEAD.
    pub fn create_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.inner();
        let head = repo.head()?.peel_to_commit()?;
        repo.branch(name, &head, false)?;
        Ok(())
    }

    /// Delete a local branch by name.
    pub fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.inner();
        let mut branch = repo.find_branch(name, git2::BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    /// Switch HEAD to an existing branch.
    pub fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.inner();
        let obj = repo.revparse_single(&format!("refs/heads/{name}"))?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(&format!("refs/heads/{name}"))?;
        Ok(())
    }

    /// Return the short name of the current branch, or None if detached.
    pub fn get_current_branch(&self) -> Result<Option<String>, GitError> {
        let repo = self.inner();
        match repo.head() {
            Ok(head) => Ok(head.shorthand().map(String::from)),
            Err(_) => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "content\n").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git_repo.find_tree(tree_id).unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        (dir, repo)
    }

    #[test]
    fn test_create_commit() {
        let (dir, repo) = create_test_repo();
        fs::write(dir.path().join("file.txt"), "updated\n").unwrap();
        repo.stage_files(&["file.txt".to_string()]).unwrap();
        let oid = repo
            .create_commit("Test commit", "Author", "author@test.com")
            .unwrap();
        assert!(!oid.is_empty());
        let commits = repo.walk_commits(10).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "Test commit");
    }

    #[test]
    fn test_create_and_delete_branch() {
        let (_dir, repo) = create_test_repo();
        repo.create_branch("feature/test").unwrap();
        let branches = repo.branches().unwrap();
        assert!(branches.iter().any(|b| b.name == "feature/test"));

        repo.delete_branch("feature/test").unwrap();
        let branches = repo.branches().unwrap();
        assert!(!branches.iter().any(|b| b.name == "feature/test"));
    }

    #[test]
    fn test_checkout_branch() {
        let (_dir, repo) = create_test_repo();
        repo.create_branch("develop").unwrap();
        repo.checkout_branch("develop").unwrap();
        let current = repo.get_current_branch().unwrap();
        assert_eq!(current, Some("develop".to_string()));
    }

    #[test]
    fn test_get_current_branch() {
        let (_dir, repo) = create_test_repo();
        let branch = repo.get_current_branch().unwrap();
        assert!(branch.is_some());
    }
}
