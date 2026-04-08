//! Gitignore file management — read, write, and append patterns.
//!
//! Extends [`Repository`] with methods to manage the repository's root
//! `.gitignore` file. All operations are direct filesystem I/O on
//! `<repo_root>/.gitignore`.

use std::fs;
use std::path::PathBuf;

use crate::error::GitError;
use crate::repository::Repository;

impl Repository {
    /// Return the absolute path to the repository's root `.gitignore` file.
    fn gitignore_path(&self) -> PathBuf {
        self.path().join(".gitignore")
    }

    /// Read the content of the repository's `.gitignore` file.
    ///
    /// Returns an empty string if the file does not exist.
    pub fn read_gitignore(&self) -> Result<String, GitError> {
        let path = self.gitignore_path();
        if path.exists() {
            Ok(fs::read_to_string(&path)?)
        } else {
            Ok(String::new())
        }
    }

    /// Write the full content of the repository's `.gitignore` file.
    ///
    /// Creates the file if it does not exist.
    pub fn write_gitignore(&self, content: &str) -> Result<(), GitError> {
        let path = self.gitignore_path();
        Ok(fs::write(&path, content)?)
    }

    /// Append a pattern to the repository's `.gitignore` file.
    ///
    /// Checks for duplicate patterns before appending. If the pattern already
    /// exists (as an exact line match), this is a no-op. Ensures the file ends
    /// with a newline before appending.
    ///
    /// Creates the file if it does not exist.
    pub fn add_gitignore_pattern(&self, pattern: &str) -> Result<(), GitError> {
        let path = self.gitignore_path();
        let existing = if path.exists() {
            fs::read_to_string(&path)?
        } else {
            String::new()
        };

        // Check for duplicate (exact line match, trimmed)
        let trimmed_pattern = pattern.trim();
        if existing.lines().any(|line| line.trim() == trimmed_pattern) {
            return Ok(()); // Already present
        }

        // Ensure trailing newline before appending
        let mut content = existing;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(trimmed_pattern);
        content.push('\n');

        Ok(fs::write(&path, content)?)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::repository::Repository;

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let tmp = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(tmp.path()).unwrap();
        {
            let mut cfg = git_repo.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@test.com").unwrap();
        }
        fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        {
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        drop(git_repo);
        let repo = Repository::open(tmp.path()).unwrap();
        (tmp, repo)
    }

    #[test]
    fn test_read_gitignore_missing_file() {
        let (_tmp, repo) = create_test_repo();
        let content = repo.read_gitignore().unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_read_gitignore_existing_file() {
        let (tmp, repo) = create_test_repo();
        fs::write(tmp.path().join(".gitignore"), "*.log\nbuild/\n").unwrap();
        let content = repo.read_gitignore().unwrap();
        assert_eq!(content, "*.log\nbuild/\n");
    }

    #[test]
    fn test_write_gitignore_creates_file() {
        let (tmp, repo) = create_test_repo();
        assert!(!tmp.path().join(".gitignore").exists());
        repo.write_gitignore("node_modules/\n").unwrap();
        assert!(tmp.path().join(".gitignore").exists());
        let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "node_modules/\n");
    }

    #[test]
    fn test_write_gitignore_overwrites() {
        let (tmp, repo) = create_test_repo();
        fs::write(tmp.path().join(".gitignore"), "old\n").unwrap();
        repo.write_gitignore("new\n").unwrap();
        let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "new\n");
    }

    #[test]
    fn test_add_pattern_new_file() {
        let (tmp, repo) = create_test_repo();
        repo.add_gitignore_pattern("*.log").unwrap();
        let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "*.log\n");
    }

    #[test]
    fn test_add_pattern_appends() {
        let (tmp, repo) = create_test_repo();
        fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
        repo.add_gitignore_pattern("build/").unwrap();
        let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "*.log\nbuild/\n");
    }

    #[test]
    fn test_add_pattern_no_trailing_newline() {
        let (tmp, repo) = create_test_repo();
        fs::write(tmp.path().join(".gitignore"), "*.log").unwrap();
        repo.add_gitignore_pattern("build/").unwrap();
        let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "*.log\nbuild/\n");
    }

    #[test]
    fn test_add_pattern_duplicate_noop() {
        let (tmp, repo) = create_test_repo();
        fs::write(tmp.path().join(".gitignore"), "*.log\nbuild/\n").unwrap();
        repo.add_gitignore_pattern("*.log").unwrap();
        let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "*.log\nbuild/\n"); // Unchanged
    }

    #[test]
    fn test_add_pattern_trims_whitespace() {
        let (tmp, repo) = create_test_repo();
        fs::write(tmp.path().join(".gitignore"), "  *.log  \n").unwrap();
        repo.add_gitignore_pattern("  *.log  ").unwrap();
        // Should be a no-op — trimmed pattern matches trimmed line
        let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "  *.log  \n");
    }
}
