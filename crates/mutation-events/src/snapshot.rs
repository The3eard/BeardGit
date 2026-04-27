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
    /// Per-file status fingerprint: `(path, libgit2 Status bitflags)`.
    ///
    /// `status_dirty` alone is insufficient for the staging flow — both
    /// before and after `git add` have `dirty=true`, so the boolean
    /// doesn't flip and the diff misses index movement. Storing the
    /// full entry set lets `diff` detect per-file changes (staging,
    /// unstaging, discarding) even when the overall dirty flag is
    /// unchanged.
    pub status_entries: BTreeSet<(String, u32)>,
}

impl Snapshot {
    /// Capture a snapshot of the repository rooted at `path`.
    pub fn capture(path: &Path) -> Result<Self, SnapshotError> {
        let repo = git2::Repository::open(path).map_err(|source| SnapshotError::OpenRepo {
            path: path.display().to_string(),
            source,
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
        stash_repo.stash_foreach(|_, _, _| {
            stash_count += 1;
            true
        })?;

        let worktree_count = repo.worktrees()?.len();

        let remote_names: BTreeSet<String> = repo
            .remotes()?
            .iter()
            .flatten()
            .map(|s| s.to_string())
            .collect();

        let mut status_opts = git2::StatusOptions::new();
        status_opts
            .include_untracked(true)
            .recurse_untracked_dirs(false);
        let statuses = repo.statuses(Some(&mut status_opts))?;
        let status_dirty = !statuses.is_empty();
        let status_entries: BTreeSet<(String, u32)> = statuses
            .iter()
            .map(|entry| {
                (
                    entry.path().unwrap_or_default().to_string(),
                    entry.status().bits(),
                )
            })
            .collect();

        Ok(Self {
            head_oid,
            refs,
            stash_count,
            worktree_count,
            remote_names,
            status_dirty,
            status_entries,
        })
    }

    /// Diff `self` (before) against `after` → [`MutationFlags`].
    ///
    /// `status_changed` flips whenever the per-file status fingerprint
    /// changed — this catches `git add` / `git reset` / `git restore`
    /// against files that leave the overall "worktree is dirty" boolean
    /// unchanged, which would otherwise suppress the refresh.
    pub fn diff(&self, after: &Snapshot) -> MutationFlags {
        MutationFlags {
            head_changed: self.head_oid != after.head_oid,
            refs_changed: self.refs != after.refs,
            status_changed: self.status_entries != after.status_entries,
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
    fn diff_detects_per_file_status_change_even_when_still_dirty() {
        let (_tmp, path) = create_repo_with_staged_changes(&[("a.txt", "a\n")]);
        let before = Snapshot::capture(&path).unwrap();
        // Already dirty — add another untracked file.
        std::fs::write(path.join("b.txt"), "b\n").unwrap();
        let after = Snapshot::capture(&path).unwrap();
        let flags = before.diff(&after);
        assert!(!flags.head_changed);
        assert!(!flags.refs_changed);
        // Both snapshots are dirty, but the per-file fingerprint added
        // `b.txt` → status_changed must flip so the UI refreshes.
        assert!(flags.status_changed);
    }

    #[test]
    fn diff_detects_staging_transition() {
        // Covers the regression: `git add a.txt` on a repo that was
        // already dirty keeps `status_dirty = true` on both sides, but
        // must still report `status_changed` because the per-file
        // Status bitflags flip (WT_NEW → INDEX_NEW).
        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        // Seed with a commit so there's a HEAD.
        {
            let mut cfg = repo.config().unwrap();
            cfg.set_str("user.name", "t").unwrap();
            cfg.set_str("user.email", "t@t").unwrap();
            std::fs::write(tmp.path().join("seed.txt"), "seed\n").unwrap();
            let mut idx = repo.index().unwrap();
            idx.add_all(["*"], git2::IndexAddOption::DEFAULT, None)
                .unwrap();
            idx.write().unwrap();
            let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "seed", &tree, &[])
                .unwrap();
        }
        // Dirty worktree: two untracked files, neither staged.
        std::fs::write(tmp.path().join("a.txt"), "a\n").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "b\n").unwrap();
        let before = Snapshot::capture(tmp.path()).unwrap();
        assert!(before.status_dirty);
        // Stage a.txt only; b.txt stays unstaged.
        let mut idx = repo.index().unwrap();
        idx.add_path(std::path::Path::new("a.txt")).unwrap();
        idx.write().unwrap();
        let after = Snapshot::capture(tmp.path()).unwrap();
        assert!(after.status_dirty);
        let flags = before.diff(&after);
        assert!(
            flags.status_changed,
            "staging must flip status_changed even when dirty flag is stable"
        );
    }

    #[test]
    fn identical_snapshots_diff_to_empty() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let a = Snapshot::capture(&path).unwrap();
        let b = Snapshot::capture(&path).unwrap();
        assert!(a.diff(&b).is_empty());
    }
}
