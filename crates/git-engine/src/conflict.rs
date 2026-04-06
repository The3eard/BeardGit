//! Conflict detection, status reporting, and resolution operations.
//!
//! Extends [`Repository`] with methods to detect in-progress merge, rebase,
//! cherry-pick, and revert operations, list conflicted files, and abort or
//! continue those operations via the git CLI.

use serde::{Deserialize, Serialize};

use crate::error::GitError;
use crate::repository::Repository;

/// The kind of in-progress operation that may cause conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictState {
    /// No conflicting operation in progress.
    None,
    /// A `git merge` is in progress.
    Merging,
    /// A `git rebase` is in progress.
    Rebasing,
    /// A `git cherry-pick` is in progress.
    CherryPicking,
    /// A `git revert` is in progress.
    Reverting,
}

/// Full conflict status combining the operation state, conflicted file list,
/// and whether the operation can be continued (all conflicts resolved).
#[derive(Debug, Clone, Serialize)]
pub struct ConflictStatus {
    /// The currently in-progress operation, or [`ConflictState::None`].
    pub state: ConflictState,
    /// Repo-relative paths of files with unresolved merge conflicts.
    pub conflicted_files: Vec<String>,
    /// `true` when all conflicts have been resolved and the operation can
    /// be continued (e.g. `git merge --continue`).
    pub can_continue: bool,
}

impl Repository {
    /// Detect the current conflict state by checking sentinel files in `.git/`.
    ///
    /// Checks for `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, and
    /// rebase indicators (`rebase-merge/`, `rebase-apply/`, `REBASE_HEAD`)
    /// in the git directory.
    pub fn detect_conflict_state(&self) -> ConflictState {
        let git_dir = self.inner().path();

        // Rebase takes priority — check multiple indicators
        if git_dir.join("rebase-merge").exists()
            || git_dir.join("rebase-apply").exists()
            || git_dir.join("REBASE_HEAD").exists()
        {
            return ConflictState::Rebasing;
        }

        if git_dir.join("MERGE_HEAD").exists() {
            return ConflictState::Merging;
        }

        if git_dir.join("CHERRY_PICK_HEAD").exists() {
            return ConflictState::CherryPicking;
        }

        if git_dir.join("REVERT_HEAD").exists() {
            return ConflictState::Reverting;
        }

        ConflictState::None
    }

    /// List repo-relative paths of all files with unresolved merge conflicts.
    ///
    /// Re-reads the index from disk to pick up conflict markers left by CLI
    /// operations (merge, rebase, cherry-pick, revert), then uses the
    /// `libgit2` conflict iterator to enumerate entries with multiple stages.
    pub fn conflicted_files(&self) -> Result<Vec<String>, GitError> {
        let mut index = self.inner().index()?;
        // Force re-read from disk so we see conflicts written by git CLI
        index.read(true)?;
        let conflicts = index.conflicts()?;

        let mut paths = Vec::new();
        for entry in conflicts {
            let entry = entry?;
            // Pick whichever side is present to extract the path
            let path = entry
                .our
                .as_ref()
                .or(entry.their.as_ref())
                .or(entry.ancestor.as_ref())
                .and_then(|e| std::str::from_utf8(&e.path).ok())
                .map(String::from);

            if let Some(p) = path
                && !paths.contains(&p)
            {
                paths.push(p);
            }
        }

        Ok(paths)
    }

    /// Return the full conflict status: operation state, file list, and
    /// whether the user can continue the operation.
    ///
    /// `can_continue` is `true` when an operation is in progress but no
    /// conflicted files remain (i.e. the user has resolved all conflicts).
    pub fn conflict_status(&self) -> Result<ConflictStatus, GitError> {
        let state = self.detect_conflict_state();
        let conflicted_files = self.conflicted_files()?;
        let can_continue = state != ConflictState::None && conflicted_files.is_empty();

        Ok(ConflictStatus {
            state,
            conflicted_files,
            can_continue,
        })
    }

    // -----------------------------------------------------------------
    // Abort / continue helpers — delegate to the git CLI
    // -----------------------------------------------------------------

    /// Abort an in-progress merge, restoring the pre-merge state.
    pub fn abort_merge(&self) -> Result<crate::cli::GitCliResult, GitError> {
        self.git_cmd(&["merge", "--abort"])
    }

    /// Continue a merge after all conflicts have been resolved.
    ///
    /// The index must have no unresolved conflicts; otherwise git will refuse.
    pub fn continue_merge(&self) -> Result<crate::cli::GitCliResult, GitError> {
        self.git_cmd(&["merge", "--continue"])
    }

    /// Abort an in-progress rebase, restoring HEAD to the pre-rebase state.
    pub fn abort_rebase(&self) -> Result<crate::cli::GitCliResult, GitError> {
        self.git_cmd(&["rebase", "--abort"])
    }

    /// Continue a rebase after all conflicts have been resolved.
    pub fn continue_rebase(&self) -> Result<crate::cli::GitCliResult, GitError> {
        self.git_cmd(&["rebase", "--continue"])
    }

    /// Abort an in-progress cherry-pick.
    pub fn abort_cherry_pick(&self) -> Result<crate::cli::GitCliResult, GitError> {
        self.git_cmd(&["cherry-pick", "--abort"])
    }

    /// Continue a cherry-pick after all conflicts have been resolved.
    pub fn continue_cherry_pick(&self) -> Result<crate::cli::GitCliResult, GitError> {
        self.git_cmd(&["cherry-pick", "--continue"])
    }

    /// Abort an in-progress revert.
    pub fn abort_revert(&self) -> Result<crate::cli::GitCliResult, GitError> {
        self.git_cmd(&["revert", "--abort"])
    }

    /// Continue a revert after all conflicts have been resolved.
    pub fn continue_revert(&self) -> Result<crate::cli::GitCliResult, GitError> {
        self.git_cmd(&["revert", "--continue"])
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

    /// Create a minimal git repo with one commit on `file.txt`.
    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        git_repo
            .config()
            .unwrap()
            .set_str("user.name", "Test")
            .unwrap();
        git_repo
            .config()
            .unwrap()
            .set_str("user.email", "test@test.com")
            .unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        fs::write(dir.path().join("file.txt"), "line1\n").unwrap();
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

    /// Create a merge conflict in the test repo.
    ///
    /// 1. Create `conflict-branch`, modify `file.txt`, commit
    /// 2. Switch back to master, modify `file.txt` differently, commit
    /// 3. Attempt merge — this will fail with a conflict
    fn create_merge_conflict(dir: &tempfile::TempDir, repo: &Repository) {
        // Create and switch to a branch
        repo.create_branch("conflict-branch").unwrap();
        repo.checkout_branch("conflict-branch").unwrap();

        // Commit a change on the branch
        fs::write(dir.path().join("file.txt"), "branch version\n").unwrap();
        repo.stage_files(&["file.txt".to_string()]).unwrap();
        repo.create_commit("Branch commit", "Test", "test@test.com")
            .unwrap();

        // Switch back to master and make a conflicting change
        repo.checkout_branch("master").unwrap();
        fs::write(dir.path().join("file.txt"), "master version\n").unwrap();
        repo.stage_files(&["file.txt".to_string()]).unwrap();
        repo.create_commit("Master commit", "Test", "test@test.com")
            .unwrap();

        // Attempt merge — should fail with conflict
        let result = repo.merge_branch("conflict-branch").unwrap();
        assert!(
            !result.success,
            "merge should fail due to conflict, stderr: {}",
            result.stderr
        );
    }

    #[test]
    fn test_detect_no_conflict() {
        let (_dir, repo) = create_test_repo();
        let state = repo.detect_conflict_state();
        assert_eq!(state, ConflictState::None);
    }

    #[test]
    fn test_detect_merging() {
        let (dir, repo) = create_test_repo();
        create_merge_conflict(&dir, &repo);
        let state = repo.detect_conflict_state();
        assert_eq!(state, ConflictState::Merging);
    }

    #[test]
    fn test_conflicted_files_list() {
        let (dir, repo) = create_test_repo();
        create_merge_conflict(&dir, &repo);
        let files = repo.conflicted_files().unwrap();
        assert!(
            files.contains(&"file.txt".to_string()),
            "expected file.txt in conflicted files, got: {:?}",
            files
        );
    }

    #[test]
    fn test_conflict_status_combined() {
        let (dir, repo) = create_test_repo();
        create_merge_conflict(&dir, &repo);
        let status = repo.conflict_status().unwrap();
        assert_eq!(status.state, ConflictState::Merging);
        assert!(!status.conflicted_files.is_empty());
        // There are still unresolved conflicts, so can_continue should be false
        assert!(!status.can_continue);
    }

    #[test]
    fn test_abort_merge_clears_state() {
        let (dir, repo) = create_test_repo();
        create_merge_conflict(&dir, &repo);

        // Confirm we are in a merge state
        assert_eq!(repo.detect_conflict_state(), ConflictState::Merging);

        // Abort the merge
        let result = repo.abort_merge().unwrap();
        assert!(result.success, "abort should succeed: {}", result.stderr);

        // State should be clear now
        assert_eq!(repo.detect_conflict_state(), ConflictState::None);
    }

    #[test]
    fn test_no_conflict_status() {
        let (_dir, repo) = create_test_repo();
        let status = repo.conflict_status().unwrap();
        assert_eq!(status.state, ConflictState::None);
        assert!(status.conflicted_files.is_empty());
        assert!(!status.can_continue);
    }

    #[test]
    fn test_continue_merge_after_resolution() {
        let (dir, repo) = create_test_repo();
        create_merge_conflict(&dir, &repo);

        // Confirm merge in progress with unresolved conflicts
        let status = repo.conflict_status().unwrap();
        assert_eq!(status.state, ConflictState::Merging);
        assert!(!status.can_continue);

        // Resolve the conflict by writing resolved content and staging
        fs::write(dir.path().join("file.txt"), "resolved content\n").unwrap();
        repo.stage_files(&["file.txt".to_string()]).unwrap();

        // After staging, can_continue should be true
        let status = repo.conflict_status().unwrap();
        assert_eq!(status.state, ConflictState::Merging);
        assert!(
            status.can_continue,
            "expected can_continue after resolving conflicts"
        );

        // Continue the merge — set GIT_EDITOR to avoid "Terminal is dumb" in CI
        unsafe { std::env::set_var("GIT_EDITOR", "true") };
        let result = repo.continue_merge().unwrap();
        assert!(
            result.success,
            "continue_merge should succeed: {}",
            result.stderr
        );

        // State should be clear
        assert_eq!(repo.detect_conflict_state(), ConflictState::None);
    }

    #[test]
    fn test_detect_cherry_pick_state() {
        let (_dir, repo) = create_test_repo();
        let git_dir = repo.inner().path().to_path_buf();

        fs::write(git_dir.join("CHERRY_PICK_HEAD"), "abc123").unwrap();

        assert_eq!(repo.detect_conflict_state(), ConflictState::CherryPicking);
    }

    #[test]
    fn test_detect_revert_state() {
        let (_dir, repo) = create_test_repo();
        let git_dir = repo.inner().path().to_path_buf();

        fs::write(git_dir.join("REVERT_HEAD"), "abc123").unwrap();

        assert_eq!(repo.detect_conflict_state(), ConflictState::Reverting);
    }

    #[test]
    fn test_detect_rebase_state() {
        let (_dir, repo) = create_test_repo();
        let git_dir = repo.inner().path().to_path_buf();

        fs::create_dir(git_dir.join("rebase-merge")).unwrap();

        assert_eq!(repo.detect_conflict_state(), ConflictState::Rebasing);
    }

    #[test]
    fn test_conflict_state_priority() {
        let (_dir, repo) = create_test_repo();
        let git_dir = repo.inner().path().to_path_buf();

        // Create both MERGE_HEAD and rebase-merge — rebase should take priority
        fs::write(git_dir.join("MERGE_HEAD"), "abc123").unwrap();
        fs::create_dir(git_dir.join("rebase-merge")).unwrap();

        assert_eq!(
            repo.detect_conflict_state(),
            ConflictState::Rebasing,
            "rebase should take priority over merge"
        );
    }
}
