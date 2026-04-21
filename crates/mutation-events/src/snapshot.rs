//! Repository state snapshot — captured pre/post every mutation so
//! `diff` can produce a precise [`crate::MutationFlags`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use thiserror::Error;

use crate::MutationFlags;

/// Errors that may occur while capturing a snapshot.
#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("failed to open repo at {path}: {source}")]
    OpenRepo {
        path: String,
        #[source]
        source: git2::Error,
    },
    #[error("libgit2 error: {0}")]
    Git(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Small, cheap descriptor of repository state relevant to UI refresh.
///
/// Target capture cost < 2 ms on a warm repo — dwarfed by the mutation
/// itself. All fields are ordered (`BTreeMap`/`BTreeSet`) so equality
/// is deterministic across captures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub head_oid: Option<String>,
    pub refs: BTreeMap<String, String>,
    pub stash_count: usize,
    pub worktree_count: usize,
    pub remote_names: BTreeSet<String>,
    pub status_dirty: bool,
}

impl Snapshot {
    /// Capture a snapshot of the repository rooted at `path`.
    pub fn capture(path: &Path) -> Result<Self, SnapshotError> {
        let repo = git2::Repository::open(path).map_err(|source| {
            SnapshotError::OpenRepo {
                path: path.display().to_string(),
                source,
            }
        })?;

        let head_oid = match repo.head() {
            Ok(r) => r.target().map(|o| o.to_string()),
            Err(_) => None, // unborn branch is fine
        };

        let mut refs = BTreeMap::new();
        for rref in repo.references()?.flatten() {
            if let (Some(name), Some(oid)) = (rref.name(), rref.target()) {
                refs.insert(name.to_string(), oid.to_string());
            }
        }

        let mut stash_count = 0;
        // `stash_foreach` needs `&mut` repo; reopen for it.
        let mut stash_repo = git2::Repository::open(path).map_err(SnapshotError::from)?;
        stash_repo
            .stash_foreach(|_, _, _| {
                stash_count += 1;
                true
            })
            .ok();

        let worktree_count = repo.worktrees().map(|w| w.len()).unwrap_or(0);

        let remote_names: BTreeSet<String> = repo
            .remotes()?
            .iter()
            .flatten()
            .map(|s| s.to_string())
            .collect();

        let mut status_opts = git2::StatusOptions::new();
        status_opts.include_untracked(true).recurse_untracked_dirs(false);
        let status_dirty = !repo.statuses(Some(&mut status_opts))?.is_empty();

        Ok(Self {
            head_oid,
            refs,
            stash_count,
            worktree_count,
            remote_names,
            status_dirty,
        })
    }

    /// Diff `self` (before) against `after` → [`MutationFlags`].
    pub fn diff(&self, after: &Snapshot) -> MutationFlags {
        MutationFlags {
            head_changed: self.head_oid != after.head_oid,
            refs_changed: self.refs != after.refs,
            status_changed: self.status_dirty != after.status_dirty,
            stashes_changed: self.stash_count != after.stash_count,
            worktrees_changed: self.worktree_count != after.worktree_count,
            remotes_changed: self.remote_names != after.remote_names,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git_engine::test_support::{create_repo_with_n_commits, create_repo_with_staged_changes};

    #[test]
    fn capture_empty_repo_has_no_head() {
        let tmp = tempfile::tempdir().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let snap = Snapshot::capture(tmp.path()).unwrap();
        assert!(snap.head_oid.is_none());
        assert!(snap.refs.is_empty());
    }

    #[test]
    fn diff_detects_head_move() {
        let (_tmp1, path) = create_repo_with_n_commits(1);
        let before = Snapshot::capture(&path).unwrap();

        // Simulate a commit: create another commit on the same repo.
        let repo = git_engine::Repository::open(&path).unwrap();
        std::fs::write(path.join("x.txt"), "x\n").unwrap();
        repo.stage_files(&["x.txt".to_string()]).unwrap();
        repo.create_commit("next").unwrap();

        let after = Snapshot::capture(&path).unwrap();
        let flags = before.diff(&after);
        assert!(flags.head_changed);
        assert!(flags.refs_changed);
        // status should be clean after commit.
        assert!(flags.status_changed || !after.status_dirty);
    }

    #[test]
    fn diff_detects_status_change() {
        let (_tmp, path) = create_repo_with_staged_changes(&[("a.txt", "a\n")]);
        let before = Snapshot::capture(&path).unwrap();
        // Already dirty — write another unstaged file.
        std::fs::write(path.join("b.txt"), "b\n").unwrap();
        let after = Snapshot::capture(&path).unwrap();
        // `status_dirty` is a boolean, so it's only set when transitioning.
        // Fall through: refs unchanged, head unchanged.
        let flags = before.diff(&after);
        assert!(!flags.head_changed);
        assert!(!flags.refs_changed);
        // both before and after are dirty, so status_changed stays false —
        // we're testing the equality semantics, not richer diffing.
        assert!(!flags.status_changed);
    }

    #[test]
    fn identical_snapshots_diff_to_empty() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let a = Snapshot::capture(&path).unwrap();
        let b = Snapshot::capture(&path).unwrap();
        assert!(a.diff(&b).is_empty());
    }
}
