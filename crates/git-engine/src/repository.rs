//! Repository discovery and high-level status inspection.
//!
//! [`Repository`] is the central type of this crate. Open a repository with
//! [`Repository::open`] and then use its methods (defined across multiple
//! modules) to perform git operations.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::GitError;

/// Metadata about a single git branch (local or remote).
#[derive(Debug, Clone, Serialize)]
pub struct BranchInfo {
    /// Short branch name (e.g. `main`, `origin/main`).
    pub name: String,
    /// Whether this branch is the current HEAD.
    pub is_head: bool,
    /// Whether this is a remote-tracking branch.
    pub is_remote: bool,
    /// Full SHA-1 OID of the branch tip as a hex string.
    pub oid: String,
}

/// Starship-style git status counters for display in the title bar.
#[derive(Debug, Clone, Serialize)]
pub struct StatusSummary {
    /// Commits ahead of upstream.
    pub ahead: usize,
    /// Commits behind upstream.
    pub behind: usize,
    /// Staged file count.
    pub staged: usize,
    /// Modified (unstaged) file count.
    pub unstaged: usize,
    /// Untracked file count.
    pub untracked: usize,
    /// Conflicted file count.
    pub conflicted: usize,
    /// Stash entry count.
    pub stash_count: usize,
}

/// High-level summary of a repository's current state.
#[derive(Debug, Clone, Serialize)]
pub struct RepoStatus {
    /// Absolute path to the working directory.
    pub path: String,
    /// Short name of the current branch, or `None` if HEAD is detached or the repo is empty.
    pub head_branch: Option<String>,
    /// OID of the current HEAD commit as a hex string, or `None` for an empty repo.
    pub head_oid: Option<String>,
    /// `true` when the repository has no commits.
    pub is_empty: bool,
    /// Total number of local and remote branches.
    pub branch_count: usize,
}

/// An open git repository backed by `libgit2`.
///
/// Methods are spread across multiple modules (`commits`, `staging`,
/// `operations`, `diff`, `cli`) via `impl Repository` blocks.
pub struct Repository {
    repo: git2::Repository,
    path: PathBuf,
    /// Cached full tag list. Populated on first access; invalidated on create/delete.
    pub(crate) tag_cache: std::sync::Mutex<Option<Vec<crate::cli::TagInfo>>>,
}

impl std::fmt::Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Repository")
            .field("path", &self.path)
            .finish()
    }
}

impl Repository {
    /// Open a repository by discovering it from the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let path = path.as_ref();
        let repo = git2::Repository::discover(path)
            .map_err(|_| GitError::RepoNotFound(path.to_string_lossy().to_string()))?;
        let repo_path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
        Ok(Self {
            repo,
            path: repo_path,
            tag_cache: std::sync::Mutex::new(None),
        })
    }

    /// Return high-level status information about the repository.
    pub fn status(&self) -> Result<RepoStatus, GitError> {
        let is_empty = self.repo.is_empty()?;

        let (head_branch, head_oid) = if is_empty {
            (None, None)
        } else {
            match self.repo.head() {
                Ok(head_ref) => {
                    let branch = head_ref.shorthand().map(|s| s.to_owned());
                    let oid = head_ref.target().map(|id| id.to_string());
                    (branch, oid)
                }
                Err(_) => (None, None),
            }
        };

        let branch_count = self.repo.branches(None)?.filter_map(|b| b.ok()).count();

        Ok(RepoStatus {
            path: self.path.to_string_lossy().to_string(),
            head_branch,
            head_oid,
            is_empty,
            branch_count,
        })
    }

    /// List all local and remote branches.
    pub fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let head_oid = self.repo.head().ok().and_then(|h| h.target());

        let mut branches = Vec::new();

        for item in self.repo.branches(None)? {
            let (branch, branch_type) = item?;

            let name = match branch.name()? {
                Some(n) => n.to_owned(),
                None => continue,
            };

            let is_remote = branch_type == git2::BranchType::Remote;

            let oid = match branch.get().target() {
                Some(id) => id.to_string(),
                None => continue,
            };

            let is_head =
                head_oid.is_some_and(|h| (branch.get().target() == Some(h)) && !is_remote);

            branches.push(BranchInfo {
                name,
                is_head,
                is_remote,
                oid,
            });
        }

        Ok(branches)
    }

    /// Starship-style status summary: ahead/behind remote, staged, unstaged, stash count.
    pub fn status_summary(&self) -> Result<StatusSummary, GitError> {
        let mut staged = 0usize;
        let mut unstaged = 0usize;
        let mut untracked = 0usize;
        let mut conflicted = 0usize;

        let statuses = self.repo.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .include_ignored(false),
        ))?;

        for entry in statuses.iter() {
            let s = entry.status();
            if s.intersects(
                git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
                    | git2::Status::INDEX_TYPECHANGE,
            ) {
                staged += 1;
            }
            if s.intersects(
                git2::Status::WT_MODIFIED
                    | git2::Status::WT_DELETED
                    | git2::Status::WT_RENAMED
                    | git2::Status::WT_TYPECHANGE,
            ) {
                unstaged += 1;
            }
            if s.contains(git2::Status::WT_NEW) {
                untracked += 1;
            }
            if s.contains(git2::Status::CONFLICTED) {
                conflicted += 1;
            }
        }

        // Ahead/behind upstream
        let (ahead, behind) = self
            .repo
            .head()
            .ok()
            .and_then(|head| {
                let local_oid = head.target()?;
                let branch_name = head.shorthand()?.to_string();
                let upstream_name = format!("origin/{}", branch_name);
                let upstream_ref = self
                    .repo
                    .find_reference(&format!("refs/remotes/{}", upstream_name))
                    .ok()?;
                let upstream_oid = upstream_ref.target()?;
                self.repo.graph_ahead_behind(local_oid, upstream_oid).ok()
            })
            .unwrap_or((0, 0));

        // Stash count via CLI (stash_foreach requires &mut self)
        let stash_count = {
            let mut cmd = std::process::Command::new("git");
            cmd.args(["stash", "list"]).current_dir(&self.path);

            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }

            cmd.output()
                .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
                .unwrap_or(0)
        };

        Ok(StatusSummary {
            ahead,
            behind,
            staged,
            unstaged,
            untracked,
            conflicted,
            stash_count,
        })
    }

    /// Access the underlying `git2::Repository`.
    pub fn inner(&self) -> &git2::Repository {
        &self.repo
    }

    /// Return the repository's working-directory path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether this is a linked worktree (not the main worktree).
    ///
    /// Linked worktrees are created with `git worktree add` and share
    /// the object database with the main repository. Their `.git`
    /// location lives under `<main>/.git/worktrees/<name>/` rather
    /// than at `<repo>/.git/`.
    pub fn is_worktree(&self) -> bool {
        self.repo.is_worktree()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Create a minimal git repo with one commit and return the path.
    fn create_repo_with_commit(dir: &tempfile::TempDir) -> PathBuf {
        let path = dir.path().to_path_buf();
        let repo = git2::Repository::init(&path).expect("init repo");

        // Configure identity so git2 can create a commit.
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        path
    }

    #[test]
    fn test_open_valid_repo() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_commit(&dir);

        let repo = Repository::open(&path).expect("should open valid repo");
        let status = repo.status().expect("should get status");

        assert!(!status.is_empty);
        assert!(status.head_branch.is_some());
        assert!(status.head_oid.is_some());
        // At least one branch (the default branch)
        assert!(status.branch_count >= 1);
    }

    #[test]
    fn test_open_invalid_path() {
        let result = Repository::open("/tmp/this_path_does_not_exist_at_all_xyz");
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::RepoNotFound(_) => {}
            other => panic!("expected RepoNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_branches() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_commit(&dir);

        let repo = Repository::open(&path).expect("should open repo");
        let branches = repo.branches().expect("should list branches");

        assert!(!branches.is_empty(), "should have at least one branch");

        // Exactly one branch should be HEAD
        let head_branches: Vec<_> = branches.iter().filter(|b| b.is_head).collect();
        assert_eq!(head_branches.len(), 1, "exactly one HEAD branch");

        // No remote branches in a fresh local repo
        let remote_branches: Vec<_> = branches.iter().filter(|b| b.is_remote).collect();
        assert!(remote_branches.is_empty(), "no remote branches expected");
    }

    #[test]
    fn test_status_returns_repo_info() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_commit(&dir);

        let repo = Repository::open(&path).unwrap();
        let status = repo.status().unwrap();

        assert!(!status.path.is_empty(), "path should be set");
        assert!(status.head_branch.is_some(), "head_branch should be Some");
        assert!(status.head_oid.is_some(), "head_oid should be Some");
        assert!(!status.is_empty, "is_empty should be false");
    }

    #[test]
    fn test_status_empty_repo() {
        let dir = tempfile::TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let repo = Repository::open(dir.path()).unwrap();
        let status = repo.status().unwrap();

        assert!(status.is_empty, "is_empty should be true");
        assert!(status.head_branch.is_none(), "head_branch should be None");
    }

    #[test]
    fn test_status_summary_clean() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_commit(&dir);

        let repo = Repository::open(&path).unwrap();
        let summary = repo.status_summary().unwrap();

        assert_eq!(summary.staged, 0);
        assert_eq!(summary.unstaged, 0);
        assert_eq!(summary.untracked, 0);
        assert_eq!(summary.conflicted, 0);
        assert_eq!(summary.ahead, 0);
        assert_eq!(summary.behind, 0);
        assert_eq!(summary.stash_count, 0);
    }

    #[test]
    fn test_status_summary_with_changes() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_commit(&dir);

        // Create a committed file first so we can modify it
        let git_repo = git2::Repository::open(&path).unwrap();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        std::fs::write(path.join("tracked.txt"), "content\n").unwrap();
        {
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("tracked.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let head = git_repo.head().unwrap().peel_to_commit().unwrap();
            git_repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    "Add tracked file",
                    &tree,
                    &[&head],
                )
                .unwrap();
        }

        // Now modify the tracked file (unstaged change)
        std::fs::write(path.join("tracked.txt"), "modified\n").unwrap();
        // Create an untracked file
        std::fs::write(path.join("untracked.txt"), "new\n").unwrap();

        let repo = Repository::open(&path).unwrap();
        let summary = repo.status_summary().unwrap();

        assert_eq!(summary.unstaged, 1, "one unstaged modification");
        assert_eq!(summary.untracked, 1, "one untracked file");
        assert_eq!(summary.staged, 0, "nothing staged");
        assert_eq!(summary.conflicted, 0, "no conflicts");
    }

    #[test]
    fn test_path_returns_workdir() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_commit(&dir);

        let repo = Repository::open(&path).unwrap();
        // Canonicalize both to handle symlinks (e.g. /tmp -> /private/tmp on macOS)
        let expected = std::fs::canonicalize(&path).unwrap();
        let actual = std::fs::canonicalize(repo.path()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_is_worktree_main_repo_returns_false() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_commit(&dir);

        let repo = Repository::open(&path).unwrap();
        assert!(!repo.is_worktree(), "main working tree should report false");
    }

    #[test]
    fn test_is_worktree_linked_worktree_returns_true() {
        let main_dir = tempfile::TempDir::new().unwrap();
        let main_path = create_repo_with_commit(&main_dir);

        // Create a linked worktree. `git2` requires a branch ref for
        // the new worktree to check out; the test repo's default
        // branch commit works as the base.
        let main_repo = git2::Repository::open(&main_path).unwrap();
        let wt_dir = tempfile::TempDir::new().unwrap();
        let wt_path = wt_dir.path().join("wt");
        // `add` picks a ref implicitly when `reference` is None only
        // if libgit2 can derive one; for robustness we create a
        // dedicated branch.
        let head_commit = main_repo.head().unwrap().peel_to_commit().unwrap();
        main_repo
            .branch("wt-branch", &head_commit, false)
            .unwrap();
        let wt_ref = main_repo.find_reference("refs/heads/wt-branch").unwrap();
        let mut opts = git2::WorktreeAddOptions::new();
        opts.reference(Some(&wt_ref));
        main_repo
            .worktree("wt", &wt_path, Some(&opts))
            .expect("create linked worktree");

        let wt_repo = Repository::open(&wt_path).unwrap();
        assert!(wt_repo.is_worktree(), "linked worktree should report true");
    }
}
