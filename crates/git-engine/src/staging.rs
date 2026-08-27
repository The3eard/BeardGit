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

    /// Discard unstaged changes for the given files.
    ///
    /// For tracked files (modified, deleted, or renamed in the working tree)
    /// the working-tree copy is reset to match the index — i.e. `git checkout -- <path>`
    /// semantics. Staged content is preserved.
    ///
    /// For untracked files (status `"new"` and not staged) the entry is deleted
    /// from disk. The path's *parent* is canonicalized and verified to be inside
    /// the repo root before deletion, to guard against path traversal without
    /// resolving a trailing symlink — an untracked symlink is removed as a link,
    /// never followed to its target.
    ///
    /// Paths that match neither category (e.g. unknown / already clean) are
    /// silently ignored, as is a path whose entry is already gone. A delete that
    /// is *attempted and fails* is not: the surviving paths come back as
    /// [`GitError::DiscardFailed`].
    ///
    /// Note that a directory is never one of these paths in practice —
    /// `status_file` rejects one as an ambiguous path, and `file_statuses`
    /// recurses untracked directories, so callers only ever see files. The
    /// recursive branch stays for a real directory that reaches here anyway.
    ///
    /// # Safety
    /// This permanently destroys uncommitted work. Callers must confirm with
    /// the user before invoking.
    pub fn discard_files(&self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let repo = self.inner();
        let mut tracked: Vec<&String> = Vec::new();
        let mut untracked: Vec<&String> = Vec::new();

        let dirty_wt = git2::Status::WT_MODIFIED
            | git2::Status::WT_DELETED
            | git2::Status::WT_RENAMED
            | git2::Status::WT_TYPECHANGE;

        for p in paths {
            // status_file inspects a single path against the working tree and
            // index. Files that are both staged and re-modified will report
            // both INDEX_* and WT_* bits — we route on the WT_* bits because
            // the operation only affects the working tree.
            let Ok(s) = repo.status_file(Path::new(p)) else {
                continue;
            };
            if s.contains(git2::Status::WT_NEW) {
                untracked.push(p);
            } else if s.intersects(dirty_wt) {
                tracked.push(p);
            }
        }

        if !tracked.is_empty() {
            let mut checkout = git2::build::CheckoutBuilder::new();
            checkout.force();
            for p in &tracked {
                checkout.path(p.as_str());
            }
            repo.checkout_index(None, Some(&mut checkout))?;
        }

        if !untracked.is_empty() {
            let repo_root = self.path().canonicalize().map_err(GitError::Io)?;
            let mut failed: Vec<String> = Vec::new();

            for p in untracked {
                let full = repo_root.join(p);

                // Canonicalize the PARENT, never the path itself. Resolving the
                // full path follows a trailing symlink, and the delete then
                // lands on whatever the link points at — which is how
                // discarding an untracked link to a tracked directory used to
                // erase committed content. The parent still has to resolve for
                // the containment check below to mean anything.
                let (Some(parent), Some(name)) = (full.parent(), full.file_name()) else {
                    failed.push(p.clone());
                    continue;
                };
                let Ok(parent) = parent.canonicalize() else {
                    // Parent is gone, so there is nothing left at this path.
                    continue;
                };
                if !parent.starts_with(&repo_root) {
                    failed.push(p.clone());
                    continue;
                }
                let target = parent.join(name);

                // `symlink_metadata` does not follow the link, so a symlink is
                // classified as a symlink regardless of its target. Only a real
                // directory takes the recursive delete.
                let Ok(meta) = std::fs::symlink_metadata(&target) else {
                    continue; // already gone
                };
                let removed = if meta.file_type().is_dir() {
                    std::fs::remove_dir_all(&target)
                } else {
                    std::fs::remove_file(&target)
                };
                if removed.is_err() {
                    failed.push(p.clone());
                }
            }

            if !failed.is_empty() {
                return Err(GitError::DiscardFailed { paths: failed });
            }
        }

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
        // Windows CI sets core.autocrlf=true globally; checkout-backed
        // operations would rewrite \n to \r\n and break content asserts.
        git_repo
            .config()
            .unwrap()
            .set_str("core.autocrlf", "false")
            .unwrap();
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
    fn test_discard_files_restores_modified_to_index() {
        let (dir, repo) = create_repo_with_committed_file();
        // Working-tree change with nothing staged → discard reverts it.
        fs::write(dir.path().join("existing.txt"), "modified\n").unwrap();

        repo.discard_files(&["existing.txt".to_string()]).unwrap();

        let content = fs::read_to_string(dir.path().join("existing.txt")).unwrap();
        assert_eq!(
            content, "original content\n",
            "discard should restore from index"
        );
        let statuses = repo.file_statuses().unwrap();
        assert!(statuses.is_empty(), "no pending changes after discard");
    }

    #[test]
    fn test_discard_files_deletes_untracked() {
        let (dir, repo) = create_repo_with_committed_file();
        fs::write(dir.path().join("junk.txt"), "garbage\n").unwrap();

        repo.discard_files(&["junk.txt".to_string()]).unwrap();

        assert!(
            !dir.path().join("junk.txt").exists(),
            "untracked file should be deleted"
        );
    }

    #[test]
    fn test_discard_files_preserves_staged_content() {
        // Stage a change, then make a further unstaged change. Discard must
        // only reset the working tree to the staged version, not to HEAD.
        let (dir, repo) = create_repo_with_committed_file();
        fs::write(dir.path().join("existing.txt"), "staged\n").unwrap();
        repo.stage_files(&["existing.txt".to_string()]).unwrap();
        fs::write(dir.path().join("existing.txt"), "staged + more\n").unwrap();

        repo.discard_files(&["existing.txt".to_string()]).unwrap();

        let content = fs::read_to_string(dir.path().join("existing.txt")).unwrap();
        assert_eq!(content, "staged\n", "discard should keep staged content");
    }

    #[test]
    fn test_discard_files_ignores_clean_paths() {
        // Path with no pending change is a no-op; must not error.
        let (_dir, repo) = create_repo_with_committed_file();
        repo.discard_files(&["existing.txt".to_string()]).unwrap();
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

    // ── Discarding untracked symlinks ─────────────────────────────────────
    //
    // `git status` reports an untracked symlink as `WT_NEW`, so discarding
    // one lands in the delete branch. The link is what the user asked to
    // remove; whatever it points at is not part of the request. These tests
    // exist because resolving the link before deciding what to delete once
    // destroyed committed content — see `discard_files`.
    //
    // Unix-gated: creating a symlink on Windows needs elevation, so these
    // would fail for an unrelated reason on the CI matrix's third leg.

    #[cfg(unix)]
    #[test]
    fn test_discard_untracked_symlink_keeps_target_dir() {
        let (dir, repo) = create_repo_with_committed_file();
        fs::create_dir(dir.path().join("tracked_dir")).unwrap();
        fs::write(dir.path().join("tracked_dir/important.txt"), "keep me\n").unwrap();
        repo.stage_files(&["tracked_dir/important.txt".to_string()])
            .unwrap();

        std::os::unix::fs::symlink("tracked_dir", dir.path().join("link")).unwrap();
        repo.discard_files(&["link".to_string()]).unwrap();

        assert!(
            dir.path().join("tracked_dir/important.txt").exists(),
            "discarding the link must not delete the directory it points at"
        );
        assert!(
            fs::symlink_metadata(dir.path().join("link")).is_err(),
            "the link itself should be gone"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_discard_untracked_symlink_keeps_target_file() {
        let (dir, repo) = create_repo_with_committed_file();
        std::os::unix::fs::symlink("existing.txt", dir.path().join("link.txt")).unwrap();

        repo.discard_files(&["link.txt".to_string()]).unwrap();

        assert_eq!(
            fs::read_to_string(dir.path().join("existing.txt")).unwrap(),
            "original content\n",
            "discarding the link must leave the file it points at untouched"
        );
        assert!(fs::symlink_metadata(dir.path().join("link.txt")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_discard_untracked_symlink_pointing_outside_repo() {
        let (dir, repo) = create_repo_with_committed_file();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("untouchable.txt");
        fs::write(&outside_file, "not ours\n").unwrap();

        std::os::unix::fs::symlink(&outside_file, dir.path().join("escape")).unwrap();
        repo.discard_files(&["escape".to_string()]).unwrap();

        assert!(
            outside_file.exists(),
            "a link out of the repo must not delete anything outside it"
        );
        assert!(
            fs::symlink_metadata(dir.path().join("escape")).is_err(),
            "the link is inside the repo, so removing it is exactly what was asked"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_discard_untracked_broken_symlink() {
        let (dir, repo) = create_repo_with_committed_file();
        std::os::unix::fs::symlink("nowhere.txt", dir.path().join("dangling")).unwrap();

        repo.discard_files(&["dangling".to_string()]).unwrap();

        assert!(
            fs::symlink_metadata(dir.path().join("dangling")).is_err(),
            "a dangling link has no target to canonicalize, and still has to go"
        );
    }

    #[test]
    fn test_discard_untracked_directory_path_is_a_no_op() {
        // Pins the reason the recursive-delete branch is unreachable for real
        // directories: `status_file` refuses a directory ("ambiguous path"), so
        // the path is skipped before any delete. `file_statuses` recurses
        // untracked directories, so the UI only ever sends the files inside.
        // Worth a test because the alternative reading — "directories are
        // deleted recursively" — is what made the symlink bug above dangerous.
        let (dir, repo) = create_repo_with_committed_file();
        fs::create_dir_all(dir.path().join("scratch/nested")).unwrap();
        fs::write(dir.path().join("scratch/nested/a.txt"), "temp\n").unwrap();

        repo.discard_files(&["scratch".to_string()]).unwrap();
        assert!(dir.path().join("scratch/nested/a.txt").exists());

        // The file path, which is what the UI actually reports, does delete.
        repo.discard_files(&["scratch/nested/a.txt".to_string()])
            .unwrap();
        assert!(!dir.path().join("scratch/nested/a.txt").exists());
    }

    /// A discard that cannot delete has to say so — reporting success tells the
    /// user their working tree is clean while the file is still on disk.
    ///
    /// Unix-gated: the unwritable-directory trick is a POSIX mode bit, and on
    /// Windows the delete would simply succeed.
    #[cfg(unix)]
    #[test]
    fn test_discard_reports_paths_it_could_not_delete() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, repo) = create_repo_with_committed_file();
        let locked = dir.path().join("locked");
        fs::create_dir(&locked).unwrap();
        fs::write(locked.join("stuck.txt"), "can't touch this\n").unwrap();

        // Read + execute only: the entry is visible but cannot be unlinked.
        fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500)).unwrap();

        // Mode bits don't apply to root, which would make the delete succeed
        // and this assertion a false red. Probe before asserting.
        let unwritable = fs::write(locked.join("probe.txt"), "x").is_err();

        let result = repo.discard_files(&["locked/stuck.txt".to_string()]);

        // Restore write permission so the TempDir can clean itself up.
        fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();

        if !unwritable {
            return; // running as root — nothing to assert
        }
        match result.unwrap_err() {
            GitError::DiscardFailed { paths } => {
                assert_eq!(paths, vec!["locked/stuck.txt".to_string()]);
            }
            other => panic!("expected DiscardFailed, got {other:?}"),
        }
    }
}
