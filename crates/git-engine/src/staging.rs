//! Staging area (index) inspection and manipulation.
//!
//! Extends [`Repository`] with methods to query the working-directory status
//! and to move files between the working directory and the index.

use serde::Serialize;
use std::path::Path;

use crate::error::GitError;
use crate::repository::Repository;

/// Status of a single file in the working directory or index.
#[derive(Debug, Clone, Serialize)]
pub struct FileStatus {
    /// Repo-relative path of the file.
    pub path: String,
    /// Human-readable status: `"new"`, `"modified"`, `"deleted"`, or `"renamed"`.
    pub status: String,
    /// `true` if the change is in the index (staged); `false` if it is only in the working directory.
    pub is_staged: bool,
}

impl Repository {
    /// Get the status of all files in the working directory.
    pub fn file_statuses(&self) -> Result<Vec<FileStatus>, GitError> {
        let repo = self.inner();
        let statuses = repo.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(true)
                .include_ignored(false)
                .renames_head_to_index(true),
        ))?;

        let mut result = Vec::new();
        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let s = entry.status();

            let (status, is_staged) = if s.contains(git2::Status::INDEX_NEW) {
                ("new".to_string(), true)
            } else if s.contains(git2::Status::INDEX_MODIFIED) {
                ("modified".to_string(), true)
            } else if s.contains(git2::Status::INDEX_DELETED) {
                ("deleted".to_string(), true)
            } else if s.contains(git2::Status::INDEX_RENAMED) {
                ("renamed".to_string(), true)
            } else if s.contains(git2::Status::WT_NEW) {
                ("new".to_string(), false)
            } else if s.contains(git2::Status::WT_MODIFIED) {
                ("modified".to_string(), false)
            } else if s.contains(git2::Status::WT_DELETED) {
                ("deleted".to_string(), false)
            } else if s.contains(git2::Status::WT_RENAMED) {
                ("renamed".to_string(), false)
            } else {
                continue;
            };

            result.push(FileStatus {
                path,
                status,
                is_staged,
            });
        }
        Ok(result)
    }

    /// Stage specific files (add to index).
    pub fn stage_files(&self, paths: &[String]) -> Result<(), GitError> {
        let repo = self.inner();
        let mut index = repo.index()?;
        for path in paths {
            let full_path = self.path().join(path);
            if full_path.exists() {
                index.add_path(Path::new(path))?;
            } else {
                index.remove_path(Path::new(path))?;
            }
        }
        index.write()?;
        Ok(())
    }

    /// Unstage specific files (reset from index to HEAD).
    pub fn unstage_files(&self, paths: &[String]) -> Result<(), GitError> {
        let repo = self.inner();
        let head = repo.head()?.peel_to_commit()?;
        let head_tree = head.tree()?;

        let mut index = repo.index()?;
        for path in paths {
            if let Ok(entry) = head_tree.get_path(Path::new(path)) {
                // Reset to HEAD version
                index.add(&git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: entry.filemode() as u32,
                    uid: 0,
                    gid: 0,
                    file_size: 0,
                    id: entry.id(),
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_bytes().to_vec(),
                })?;
            } else {
                // File didn't exist in HEAD, remove from index
                index.remove_path(Path::new(path))?;
            }
        }
        index.write()?;
        Ok(())
    }

    /// Stage all changes.
    pub fn stage_all(&self) -> Result<(), GitError> {
        let repo = self.inner();
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Unstage all changes.
    pub fn unstage_all(&self) -> Result<(), GitError> {
        let repo = self.inner();
        let head = repo.head()?.peel_to_commit()?;
        repo.reset(head.as_object(), git2::ResetType::Mixed, None)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_repo_with_committed_file() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        let file_path = dir.path().join("existing.txt");
        fs::write(&file_path, "original content\n").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("existing.txt")).unwrap();
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
    fn test_file_statuses_clean() {
        let (_dir, repo) = create_repo_with_committed_file();
        let statuses = repo.file_statuses().unwrap();
        assert!(statuses.is_empty());
    }

    #[test]
    fn test_file_statuses_modified() {
        let (dir, repo) = create_repo_with_committed_file();
        fs::write(dir.path().join("existing.txt"), "modified\n").unwrap();
        let statuses = repo.file_statuses().unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].status, "modified");
        assert!(!statuses[0].is_staged);
    }

    #[test]
    fn test_file_statuses_new_file() {
        let (dir, repo) = create_repo_with_committed_file();
        fs::write(dir.path().join("new.txt"), "new file\n").unwrap();
        let statuses = repo.file_statuses().unwrap();
        let new = statuses.iter().find(|s| s.path == "new.txt").unwrap();
        assert_eq!(new.status, "new");
        assert!(!new.is_staged);
    }

    #[test]
    fn test_stage_and_unstage_files() {
        let (dir, repo) = create_repo_with_committed_file();
        fs::write(dir.path().join("existing.txt"), "modified\n").unwrap();

        repo.stage_files(&["existing.txt".to_string()]).unwrap();
        let statuses = repo.file_statuses().unwrap();
        let staged = statuses.iter().find(|s| s.path == "existing.txt").unwrap();
        assert!(staged.is_staged);

        repo.unstage_files(&["existing.txt".to_string()]).unwrap();
        let statuses = repo.file_statuses().unwrap();
        let unstaged = statuses.iter().find(|s| s.path == "existing.txt").unwrap();
        assert!(!unstaged.is_staged);
    }

    #[test]
    fn test_stage_all() {
        let (dir, repo) = create_repo_with_committed_file();
        fs::write(dir.path().join("existing.txt"), "modified\n").unwrap();
        fs::write(dir.path().join("new.txt"), "new file\n").unwrap();

        repo.stage_all().unwrap();
        let statuses = repo.file_statuses().unwrap();
        assert!(statuses.iter().all(|s| s.is_staged));
    }

    #[test]
    fn test_unstage_all() {
        let (dir, repo) = create_repo_with_committed_file();

        // Modify existing file and create a new file
        fs::write(dir.path().join("existing.txt"), "modified\n").unwrap();
        fs::write(dir.path().join("new.txt"), "new file\n").unwrap();

        // Stage both files
        repo.stage_files(&["existing.txt".to_string(), "new.txt".to_string()])
            .unwrap();
        let statuses = repo.file_statuses().unwrap();
        assert!(
            statuses.iter().all(|s| s.is_staged),
            "all files should be staged before unstage_all"
        );

        // Unstage all
        repo.unstage_all().unwrap();
        let statuses = repo.file_statuses().unwrap();
        assert!(
            statuses.iter().all(|s| !s.is_staged),
            "all files should be unstaged after unstage_all"
        );
        assert_eq!(statuses.len(), 2, "should still have 2 changed files");
    }
}
