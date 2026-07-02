//! Branch cleanup: default-branch resolution, merged/gone classification,
//! and batch deletion.
//!
//! Powers the "Clean up branches" flow (spec 11): list local branches that
//! are safe to prune — those whose upstream was deleted on the remote
//! (`upstream_gone`) or whose tip is already merged into the default branch —
//! and delete a selection in one shot with per-branch error capture.
//!
//! Reads use `git2`; the actual deletion reuses [`Repository::delete_branch`]
//! (which shells out to `git branch -d`/`-D`) so the merge-protection
//! semantics match the CLI exactly.

use std::collections::HashSet;

use serde::Serialize;

use crate::error::GitError;
use crate::repository::Repository;

/// A single local branch that is a candidate for cleanup, with the metadata
/// the UI shows as a "should I really delete this?" signal.
#[derive(Debug, Clone, Serialize)]
pub struct BranchCleanupCandidate {
    /// Short branch name.
    pub name: String,
    /// Full SHA-1 OID of the branch tip.
    pub tip_oid: String,
    /// Unix timestamp (seconds) of the tip commit — the "last activity" date.
    pub last_commit_time: i64,
    /// Commits on this branch that are not reachable from the cleanup target
    /// (the default branch). `0` for fully-merged branches; `> 0` for a
    /// squash-merged branch whose original commits never landed verbatim.
    pub ahead: usize,
    /// Whether this branch's configured upstream is gone (see
    /// [`crate::repository::BranchInfo::upstream_gone`]).
    pub upstream_gone: bool,
    /// Whether the tip is an ancestor of the cleanup target (fully merged).
    /// A candidate that is not merged needs a force (`-D`) delete.
    pub merged: bool,
}

/// The two cleanup groups plus the resolved target branch they were computed
/// against.
#[derive(Debug, Clone, Serialize)]
pub struct BranchCleanupList {
    /// The branch name candidates were classified against (the default branch,
    /// or HEAD as a fallback).
    pub target: String,
    /// Branches whose upstream is gone. Pre-checked in the UI. A gone branch
    /// that is not also `merged` requires a force delete.
    pub gone: Vec<BranchCleanupCandidate>,
    /// Branches fully merged into `target` (but whose upstream is *not* gone).
    /// Unchecked by default in the UI.
    pub merged: Vec<BranchCleanupCandidate>,
}

/// One failed deletion in a batch: the branch name and git's refusal reason.
#[derive(Debug, Clone, Serialize)]
pub struct BranchDeleteFailure {
    pub name: String,
    pub reason: String,
}

/// Result of a batch delete: names that were removed, and per-branch failures
/// for the ones git refused. A single refusal never aborts the rest.
#[derive(Debug, Clone, Serialize, Default)]
pub struct BatchDeleteResult {
    pub deleted: Vec<String>,
    pub failed: Vec<BranchDeleteFailure>,
}

/// Shared context for cleanup classification: the resolved target, its tip
/// OID, and the set of branches that must never be listed as candidates.
struct CleanupContext {
    target: String,
    target_oid: Option<git2::Oid>,
    /// Branches to exclude: current HEAD, the target itself, and every branch
    /// checked out in any (linked) worktree.
    excluded: HashSet<String>,
}

impl Repository {
    /// Resolve the repository's default branch name.
    ///
    /// Resolution order:
    /// 1. `origin/HEAD`'s symbolic target, if its local branch exists.
    /// 2. A conventional `main` / `master` that exists locally.
    /// 3. The current HEAD branch (fallback).
    ///
    /// Returns `None` only for a detached/empty repo with no local branches.
    pub fn default_branch(&self) -> Option<String> {
        let repo = self.inner();

        // 1. origin/HEAD → refs/remotes/origin/<name>
        if let Ok(reference) = repo.find_reference("refs/remotes/origin/HEAD")
            && let Some(target) = reference.symbolic_target()
            && let Some(short) = target.strip_prefix("refs/remotes/origin/")
            && repo.find_branch(short, git2::BranchType::Local).is_ok()
        {
            return Some(short.to_string());
        }

        // 2. Conventional names.
        for candidate in ["main", "master"] {
            if repo.find_branch(candidate, git2::BranchType::Local).is_ok() {
                return Some(candidate.to_string());
            }
        }

        // 3. Fall back to the current branch.
        repo.head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
    }

    /// Build the shared classification context for `into` (or the default
    /// branch when `None`).
    fn cleanup_context(&self, into: Option<&str>) -> Result<CleanupContext, GitError> {
        let repo = self.inner();
        let target = match into {
            Some(t) => t.to_string(),
            None => self
                .default_branch()
                .ok_or_else(|| GitError::CliError("no default branch to compare against".into()))?,
        };

        let target_oid = repo
            .find_branch(&target, git2::BranchType::Local)
            .ok()
            .and_then(|b| b.get().target());

        let mut excluded = HashSet::new();
        excluded.insert(target.clone());
        if let Ok(head) = repo.head()
            && let Some(name) = head.shorthand()
        {
            excluded.insert(name.to_string());
        }
        // Branches checked out in ANY worktree — git refuses to delete these,
        // so exclude them up front rather than failing mid-batch.
        if let Ok(worktrees) = self.list_worktrees() {
            for wt in worktrees {
                if let Some(branch) = wt.branch {
                    excluded.insert(branch);
                }
            }
        }

        Ok(CleanupContext {
            target,
            target_oid,
            excluded,
        })
    }

    /// Whether `tip` is an ancestor of (or equal to) `target_oid` — i.e. the
    /// branch is fully merged into the target.
    ///
    /// git2's `graph_descendant_of(a, b)` is true when `a` descends from `b`
    /// and is *false* for equal OIDs, so equality is handled explicitly.
    fn tip_merged_into(&self, tip: git2::Oid, target_oid: git2::Oid) -> bool {
        tip == target_oid
            || self
                .inner()
                .graph_descendant_of(target_oid, tip)
                .unwrap_or(false)
    }

    /// List local branches whose tip is an ancestor of `into` (fully merged),
    /// excluding the current branch, `into` itself, and any branch checked out
    /// in a worktree. `into` defaults to the repository's default branch.
    ///
    /// This is the spec's core primitive; the richer [`Self::cleanup_candidates`]
    /// drives the UI.
    pub fn list_merged_branches(&self, into: Option<&str>) -> Result<Vec<String>, GitError> {
        let ctx = self.cleanup_context(into)?;
        let Some(target_oid) = ctx.target_oid else {
            return Ok(Vec::new());
        };

        let mut merged = Vec::new();
        for item in self.inner().branches(Some(git2::BranchType::Local))? {
            let (branch, _) = item?;
            let Some(name) = branch.name()?.map(str::to_owned) else {
                continue;
            };
            if ctx.excluded.contains(&name) {
                continue;
            }
            let Some(tip) = branch.get().target() else {
                continue;
            };
            if self.tip_merged_into(tip, target_oid) {
                merged.push(name);
            }
        }
        Ok(merged)
    }

    /// Classify local branches into cleanup groups (gone / merged) against
    /// `into` (or the default branch when `None`), with per-branch metadata.
    ///
    /// A gone branch is always placed in `gone` (even if also merged), because
    /// "upstream deleted" is the stronger cleanup signal and the UI pre-checks
    /// that group. A branch that is neither gone nor merged is not a candidate.
    pub fn cleanup_candidates(&self, into: Option<&str>) -> Result<BranchCleanupList, GitError> {
        let repo = self.inner();
        let ctx = self.cleanup_context(into)?;
        let config = repo.config().ok();

        let mut gone = Vec::new();
        let mut merged = Vec::new();

        for item in repo.branches(Some(git2::BranchType::Local))? {
            let (branch, _) = item?;
            let Some(name) = branch.name()?.map(str::to_owned) else {
                continue;
            };
            if ctx.excluded.contains(&name) {
                continue;
            }
            let Some(tip) = branch.get().target() else {
                continue;
            };

            let upstream_gone =
                branch.upstream().is_err() && branch_upstream_configured(config.as_ref(), &name);
            let merged_flag = ctx.target_oid.is_some_and(|t| self.tip_merged_into(tip, t));

            // Only gone or merged branches are cleanup candidates.
            if !upstream_gone && !merged_flag {
                continue;
            }

            let ahead = ctx
                .target_oid
                .and_then(|t| repo.graph_ahead_behind(tip, t).ok())
                .map(|(a, _)| a)
                .unwrap_or(0);
            let last_commit_time = repo
                .find_commit(tip)
                .map(|c| c.time().seconds())
                .unwrap_or(0);

            let candidate = BranchCleanupCandidate {
                name,
                tip_oid: tip.to_string(),
                last_commit_time,
                ahead,
                upstream_gone,
                merged: merged_flag,
            };

            if upstream_gone {
                gone.push(candidate);
            } else {
                merged.push(candidate);
            }
        }

        // Most-recently-active first — the top rows are the ones the user most
        // likely still cares about, so they read them before hitting delete.
        gone.sort_by_key(|c| std::cmp::Reverse(c.last_commit_time));
        merged.sort_by_key(|c| std::cmp::Reverse(c.last_commit_time));

        Ok(BranchCleanupList {
            target: ctx.target,
            gone,
            merged,
        })
    }

    /// Delete a batch of local branches, capturing per-branch failures instead
    /// of aborting on the first refusal.
    ///
    /// Names present in `force` are deleted with `git branch -D`; all others
    /// use the safe `git branch -d`, which refuses branches with unmerged
    /// commits. Callers wrap this in a single `MutationGuard` so the whole
    /// batch emits one `project-mutated` event.
    pub fn delete_branches(&self, names: &[String], force: &[String]) -> BatchDeleteResult {
        let force_set: HashSet<&str> = force.iter().map(String::as_str).collect();
        let mut result = BatchDeleteResult::default();
        for name in names {
            match self.delete_branch(name, force_set.contains(name.as_str())) {
                Ok(()) => result.deleted.push(name.clone()),
                Err(e) => result.failed.push(BranchDeleteFailure {
                    name: name.clone(),
                    reason: e.to_string(),
                }),
            }
        }
        result
    }
}

/// Whether a local branch has an upstream configured (`branch.<name>.merge`).
/// Mirrors the check in `repository.rs`; duplicated to keep both modules
/// self-contained rather than exporting an internal helper.
fn branch_upstream_configured(config: Option<&git2::Config>, branch_name: &str) -> bool {
    config
        .and_then(|c| c.get_string(&format!("branch.{branch_name}.merge")).ok())
        .is_some()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Build a repo laid out for cleanup classification:
    /// - default branch (`main`/`master`) at the base commit,
    /// - `merged` pointing at the base (fully merged into default),
    /// - `feature` with an extra commit (unmerged, no upstream — not a candidate),
    /// - `gone` with an extra commit + a configured-but-missing upstream.
    ///
    /// Returns the tempdir (keep alive), the path, and the resolved default
    /// branch name.
    fn setup() -> (TempDir, PathBuf, String) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        let repo = git2::Repository::init(&path).unwrap();
        {
            let mut cfg = repo.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@test.com").unwrap();
            cfg.set_str("core.autocrlf", "false").unwrap();
        }
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        // Base commit on the default branch.
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let base_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
            .unwrap();
        let base = repo.find_commit(base_oid).unwrap();
        let default_branch = repo.head().unwrap().shorthand().unwrap().to_string();

        // `merged` at base → fully merged into default.
        repo.branch("merged", &base, false).unwrap();

        // `feature` with a child commit → unmerged, no upstream config.
        let feature_oid = repo
            .commit(None, &sig, &sig, "feature work", &tree, &[&base])
            .unwrap();
        let feature = repo.find_commit(feature_oid).unwrap();
        repo.branch("feature", &feature, false).unwrap();

        // `gone` with a child commit → unmerged, and a configured-but-missing
        // upstream (origin/gone is never created).
        let gone_oid = repo
            .commit(None, &sig, &sig, "gone work", &tree, &[&base])
            .unwrap();
        let gone = repo.find_commit(gone_oid).unwrap();
        repo.branch("gone", &gone, false).unwrap();
        {
            let mut cfg = repo.config().unwrap();
            cfg.set_str("branch.gone.remote", "origin").unwrap();
            cfg.set_str("branch.gone.merge", "refs/heads/gone").unwrap();
        }

        drop(base);
        drop(feature);
        drop(gone);
        drop(tree);
        drop(repo);
        (dir, path, default_branch)
    }

    #[test]
    fn default_branch_resolves_to_head_fallback() {
        let (_dir, path, default_branch) = setup();
        let repo = Repository::open(&path).unwrap();
        assert_eq!(
            repo.default_branch().as_deref(),
            Some(default_branch.as_str())
        );
    }

    #[test]
    fn list_merged_branches_returns_only_merged() {
        let (_dir, path, _default) = setup();
        let repo = Repository::open(&path).unwrap();
        let merged = repo.list_merged_branches(None).unwrap();
        assert_eq!(merged, vec!["merged".to_string()]);
    }

    #[test]
    fn cleanup_candidates_classifies_gone_and_merged() {
        let (_dir, path, default) = setup();
        let repo = Repository::open(&path).unwrap();
        let list = repo.cleanup_candidates(None).unwrap();

        assert_eq!(list.target, default);

        // `gone` → gone group, unmerged so it needs force, ahead >= 1.
        assert_eq!(list.gone.len(), 1);
        let gone = &list.gone[0];
        assert_eq!(gone.name, "gone");
        assert!(gone.upstream_gone);
        assert!(!gone.merged, "gone branch is not merged (needs force)");
        assert!(gone.ahead >= 1);
        assert!(gone.last_commit_time > 0);

        // `merged` → merged group, ahead 0.
        assert_eq!(list.merged.len(), 1);
        let merged = &list.merged[0];
        assert_eq!(merged.name, "merged");
        assert!(merged.merged);
        assert_eq!(merged.ahead, 0);

        // `feature` (neither gone nor merged) and the default branch itself
        // are not candidates in either group.
        let all: Vec<&str> = list
            .gone
            .iter()
            .chain(list.merged.iter())
            .map(|c| c.name.as_str())
            .collect();
        assert!(!all.contains(&"feature"));
        assert!(!all.contains(&default.as_str()));
    }

    #[test]
    fn cleanup_excludes_worktree_checked_out_branch() {
        let (_dir, path, _default) = setup();
        let repo = Repository::open(&path).unwrap();

        // Check `merged` out into a linked worktree — it must then be excluded
        // from both the merged list and the candidate groups.
        let wt_path = path.parent().unwrap().join("bd-wt-merged");
        repo.create_worktree(wt_path.to_str().unwrap(), "merged", false)
            .unwrap();

        let merged = repo.list_merged_branches(None).unwrap();
        assert!(
            !merged.contains(&"merged".to_string()),
            "worktree-checked-out branch must not be listed as merged"
        );
        let list = repo.cleanup_candidates(None).unwrap();
        assert!(
            list.merged.iter().all(|c| c.name != "merged"),
            "worktree-checked-out branch must not be a candidate"
        );

        // Clean up the linked worktree admin files before tempdir drop.
        let _ = repo.remove_worktree(wt_path.to_str().unwrap(), true);
    }

    #[test]
    fn delete_branches_captures_partial_failure() {
        // One deletable branch + the current branch (which git refuses to
        // delete): the deletable one succeeds, the protected one is reported
        // as a failure — the batch does not abort.
        let (_dir, path, default) = setup();
        let repo = Repository::open(&path).unwrap();

        let result = repo.delete_branches(
            &["gone".to_string(), default.clone()],
            &["gone".to_string()], // force-delete the unmerged `gone`
        );

        assert_eq!(result.deleted, vec!["gone".to_string()]);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].name, default);
        assert!(!result.failed[0].reason.is_empty());

        // `gone` is really gone; the default branch survives.
        let names: Vec<String> = repo
            .branches()
            .unwrap()
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert!(!names.contains(&"gone".to_string()));
        assert!(names.contains(&default));
    }

    #[test]
    fn delete_branches_safe_delete_refuses_unmerged() {
        // Without force, the unmerged `gone` branch is refused (git branch -d),
        // while the fully-merged `merged` branch deletes fine.
        let (_dir, path, _default) = setup();
        let repo = Repository::open(&path).unwrap();

        let result = repo.delete_branches(
            &["merged".to_string(), "gone".to_string()],
            &[], // no force
        );

        assert_eq!(result.deleted, vec!["merged".to_string()]);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].name, "gone");
    }
}
