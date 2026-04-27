//! Raw file content retrieval for CodeMirror diff views.
//!
//! Provides three methods on [`Repository`] to fetch file content from:
//! - a specific commit (by OID)
//! - the working directory
//! - the index (staged version)

use crate::error::GitError;
use crate::repository::Repository;

impl Repository {
    /// Returns the raw content of a file at a specific commit.
    ///
    /// # Parameters
    /// - `oid` – Full or abbreviated commit SHA.
    /// - `path` – Repo-relative file path.
    ///
    /// # Errors
    /// Returns [`GitError`] if the OID is invalid, the path does not exist in
    /// the commit tree, or the blob content is not valid UTF-8 (lossy decode).
    pub fn get_file_at_commit(&self, oid: &str, path: &str) -> Result<String, GitError> {
        let obj = self.inner().revparse_single(oid)?;
        let commit = obj.peel_to_commit()?;
        let tree = commit.tree()?;
        let entry = tree.get_path(std::path::Path::new(path))?;
        let blob = self.inner().find_blob(entry.id())?;
        let content = blob.content();
        let sniff_len = content.len().min(8192);
        if content[..sniff_len].contains(&0u8) {
            return Err(GitError::Binary);
        }
        Ok(String::from_utf8_lossy(content).into_owned())
    }

    /// Returns the raw content of a file from the working directory.
    ///
    /// # Parameters
    /// - `path` – Repo-relative file path.
    ///
    /// # Errors
    /// Returns [`GitError::Io`] if the file does not exist or cannot be read.
    pub fn get_file_workdir(&self, path: &str) -> Result<String, GitError> {
        let workdir = self.path().to_path_buf();
        let full_path = workdir.join(path);
        std::fs::read_to_string(&full_path).map_err(GitError::Io)
    }

    /// Returns the raw content of a file from the index (staged version).
    ///
    /// # Parameters
    /// - `path` – Repo-relative file path.
    ///
    /// # Errors
    /// Returns [`GitError::RepoNotFound`] if the file is not staged, or
    /// [`GitError::Git`] if the blob cannot be resolved.
    pub fn get_file_index(&self, path: &str) -> Result<String, GitError> {
        let index = self.inner().index()?;
        let entry = index
            .get_path(std::path::Path::new(path), 0)
            .ok_or_else(|| GitError::RepoNotFound(format!("File not in index: {path}")))?;
        let blob = self.inner().find_blob(entry.id)?;
        Ok(String::from_utf8_lossy(blob.content()).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::repository::Repository;

    fn init_repo_with_file(dir: &std::path::Path) -> Repository {
        let git_repo = git2::Repository::init(dir).unwrap();
        let mut config = git_repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        fs::write(dir.join("hello.txt"), "initial content").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(std::path::Path::new("hello.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        {
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }

        drop(config);
        drop(git_repo);
        Repository::open(dir).unwrap()
    }

    #[test]
    fn test_get_file_at_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        let head = repo.inner().head().unwrap().peel_to_commit().unwrap();
        let content = repo
            .get_file_at_commit(&head.id().to_string(), "hello.txt")
            .unwrap();
        assert_eq!(content, "initial content");
    }

    #[test]
    fn test_get_file_workdir() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        fs::write(tmp.path().join("hello.txt"), "modified content").unwrap();
        let content = repo.get_file_workdir("hello.txt").unwrap();
        assert_eq!(content, "modified content");
    }

    #[test]
    fn test_get_file_index() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        fs::write(tmp.path().join("hello.txt"), "staged content").unwrap();
        let mut index = repo.inner().index().unwrap();
        index.add_path(std::path::Path::new("hello.txt")).unwrap();
        index.write().unwrap();
        let content = repo.get_file_index("hello.txt").unwrap();
        assert_eq!(content, "staged content");
    }

    #[test]
    fn test_get_file_at_commit_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        let head = repo.inner().head().unwrap().peel_to_commit().unwrap();
        let result = repo.get_file_at_commit(&head.id().to_string(), "nonexistent.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_file_workdir_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        let result = repo.get_file_workdir("does_not_exist.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_file_index_empty_index() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        // File exists on disk but is not staged — index only holds what was added
        // After init_repo_with_file the initial commit was made, index is clean.
        // Asking for a file never staged should return an error.
        let result = repo.get_file_index("unstaged.txt");
        assert!(result.is_err());
    }

    #[test]
    fn get_file_at_commit_returns_binary_error_for_blobs_with_nul() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        // Overwrite with binary content (PNG header start, includes 0x00).
        fs::write(
            tmp.path().join("bin.bin"),
            [0x89, b'P', b'N', b'G', 0x00, 0x01],
        )
        .unwrap();
        let head_sha = {
            let git_repo = git2::Repository::open(tmp.path()).unwrap();
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("bin.bin")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            let parent = git_repo.head().unwrap().peel_to_commit().unwrap();
            let oid = git_repo
                .commit(Some("HEAD"), &sig, &sig, "add bin", &tree, &[&parent])
                .unwrap();
            oid.to_string()
        };

        let err = repo
            .get_file_at_commit(&head_sha, "bin.bin")
            .expect_err("binary blob must error");
        assert!(matches!(err, crate::error::GitError::Binary));
    }
}
