//! Worktree management — list, create, and remove git worktrees.
//!
//! Extends [`Repository`] with methods that shell out to `git worktree`
//! subcommands. All operations use the [`git_cmd`](Repository::git_cmd)
//! wrapper defined in `cli.rs`.

use crate::error::GitError;
use crate::repository::Repository;
use serde::Serialize;

/// Information about a single git worktree.
#[derive(Debug, Clone, Serialize)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree directory.
    pub path: String,
    /// Branch checked out in this worktree, if any.
    /// `None` for detached HEAD worktrees.
    pub branch: Option<String>,
    /// HEAD commit OID for this worktree.
    pub head_oid: String,
    /// `true` when this is the main worktree (the original clone directory).
    pub is_main: bool,
    /// `true` when the worktree is locked (cannot be removed without `--force`).
    pub is_locked: bool,
}

impl Repository {
    /// List all worktrees for this repository, including the main worktree.
    ///
    /// Returns a [`WorktreeInfo`] for each worktree. The first element is
    /// always the main worktree (`is_main = true`).
    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>, GitError> {
        let result = self.git_cmd(&["worktree", "list", "--porcelain"])?;
        if result.success {
            Ok(parse_worktree_list(&result.stdout))
        } else {
            Err(GitError::RepoNotFound(result.stderr))
        }
    }

    /// Create a new linked worktree at `path` checking out `branch`.
    ///
    /// When `create_branch` is `true`, a new branch is created with `-b`
    /// (equivalent to `git worktree add -b <branch> <path>`). When `false`,
    /// an existing branch is checked out into the new worktree
    /// (equivalent to `git worktree add <path> <branch>`).
    pub fn create_worktree(
        &self,
        path: &str,
        branch: &str,
        create_branch: bool,
    ) -> Result<(), GitError> {
        let result = if create_branch {
            self.git_cmd(&["worktree", "add", "-b", branch, path])
        } else {
            self.git_cmd(&["worktree", "add", path, branch])
        }?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::RepoNotFound(result.stderr))
        }
    }

    /// Remove a linked worktree at `path`.
    ///
    /// Set `force` to `true` to remove a worktree that has uncommitted changes
    /// or is locked (equivalent to `git worktree remove --force <path>`).
    pub fn remove_worktree(&self, path: &str, force: bool) -> Result<(), GitError> {
        let mut args = vec!["worktree", "remove", path];
        if force {
            args.push("--force");
        }
        let result = self.git_cmd(&args)?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::RepoNotFound(result.stderr))
        }
    }
}

/// Parse the output of `git worktree list --porcelain` into a list of [`WorktreeInfo`].
///
/// The porcelain format emits one block per worktree, separated by blank lines:
///
/// ```text
/// worktree /path/to/main
/// HEAD abc123def456
/// branch refs/heads/main
///
/// worktree /path/to/linked
/// HEAD def789abc012
/// branch refs/heads/feature
///
/// worktree /path/to/detached
/// HEAD 000aaa111bbb
/// detached
///
/// worktree /path/to/locked
/// HEAD 222ccc333ddd
/// branch refs/heads/topic
/// locked
/// ```
///
/// The first block is always the main worktree. Detached HEAD worktrees emit
/// `detached` instead of a `branch` line (branch stays `None`). Locked
/// worktrees have an extra `locked` line.
fn parse_worktree_list(output: &str) -> Vec<WorktreeInfo> {
    let mut worktrees = Vec::new();
    let mut current_path = String::new();
    let mut current_head = String::new();
    let mut current_branch: Option<String> = None;
    let mut is_locked = false;
    let mut is_first = true;

    for line in output.lines() {
        if line.is_empty() {
            if !current_path.is_empty() {
                worktrees.push(WorktreeInfo {
                    path: current_path.clone(),
                    branch: current_branch.take(),
                    head_oid: current_head.clone(),
                    is_main: is_first,
                    is_locked,
                });
                is_first = false;
                current_path.clear();
                current_head.clear();
                is_locked = false;
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = path.to_string();
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current_head = head.to_string();
        } else if let Some(branch) = line.strip_prefix("branch ") {
            // Normalise "refs/heads/main" → "main"
            current_branch = Some(
                branch
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch)
                    .to_string(),
            );
        } else if line == "locked" || line.starts_with("locked ") {
            is_locked = true;
        }
        // "detached" line → branch stays None
    }

    // Handle the last block when there is no trailing blank line.
    if !current_path.is_empty() {
        worktrees.push(WorktreeInfo {
            path: current_path,
            branch: current_branch,
            head_oid: current_head,
            is_main: is_first,
            is_locked,
        });
    }

    worktrees
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Porcelain parser unit tests ─────────────────────────────────────────

    #[test]
    fn test_parse_worktree_list_single() {
        let output = "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/main\n\n";
        let wts = parse_worktree_list(output);
        assert_eq!(wts.len(), 1);
        assert_eq!(wts[0].path, "/home/user/repo");
        assert_eq!(wts[0].branch, Some("main".to_string()));
        assert_eq!(wts[0].head_oid, "abc123");
        assert!(wts[0].is_main);
        assert!(!wts[0].is_locked);
    }

    #[test]
    fn test_parse_worktree_list_multiple() {
        let output = concat!(
            "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/main\n\n",
            "worktree /home/user/repo-feat\nHEAD def456\nbranch refs/heads/feature\n\n"
        );
        let wts = parse_worktree_list(output);
        assert_eq!(wts.len(), 2);
        assert!(wts[0].is_main);
        assert!(!wts[1].is_main);
        assert_eq!(wts[1].branch, Some("feature".to_string()));
    }

    #[test]
    fn test_parse_worktree_detached() {
        let output = concat!(
            "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/main\n\n",
            "worktree /home/user/detached\nHEAD def456\ndetached\n\n"
        );
        let wts = parse_worktree_list(output);
        assert_eq!(wts.len(), 2);
        assert!(wts[1].branch.is_none());
    }

    #[test]
    fn test_parse_worktree_locked() {
        let output = "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/main\nlocked\n\n";
        let wts = parse_worktree_list(output);
        assert_eq!(wts.len(), 1);
        assert!(wts[0].is_locked);
    }

    #[test]
    fn test_parse_worktree_no_trailing_newline() {
        let output = "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/main";
        let wts = parse_worktree_list(output);
        assert_eq!(wts.len(), 1);
        assert_eq!(wts[0].branch, Some("main".to_string()));
    }

    #[test]
    fn test_parse_empty_output() {
        let wts = parse_worktree_list("");
        assert!(wts.is_empty());
    }

    // ── Integration tests with real git repos ───────────────────────────────

    fn create_test_repo() -> (tempfile::TempDir, git2::Repository) {
        let tmp = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(tmp.path()).unwrap();
        {
            let mut cfg = git_repo.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@test.com").unwrap();
        }
        // Need at least one commit for worktree operations.
        std::fs::write(tmp.path().join("file.txt"), "hello").unwrap();
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
        (tmp, git_repo)
    }

    #[test]
    fn test_list_worktrees_default() {
        let (tmp, _git_repo) = create_test_repo();
        let repo = Repository::open(tmp.path()).unwrap();
        let wts = repo.list_worktrees().unwrap();
        assert_eq!(wts.len(), 1);
        assert!(wts[0].is_main);
        assert!(!wts[0].head_oid.is_empty());
    }

    #[test]
    fn test_create_and_remove_worktree() {
        let (tmp, _git_repo) = create_test_repo();
        let repo = Repository::open(tmp.path()).unwrap();

        // Create a linked worktree on a new branch.
        let wt_path = tmp.path().join("worktree-feat");
        repo.create_worktree(wt_path.to_str().unwrap(), "feature-branch", true)
            .unwrap();

        let wts = repo.list_worktrees().unwrap();
        assert_eq!(wts.len(), 2);
        assert_eq!(wts[1].branch, Some("feature-branch".to_string()));

        // Remove the linked worktree.
        repo.remove_worktree(wt_path.to_str().unwrap(), false)
            .unwrap();
        let wts = repo.list_worktrees().unwrap();
        assert_eq!(wts.len(), 1);
    }
}
