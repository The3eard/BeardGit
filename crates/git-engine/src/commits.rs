//! Commit walking and history retrieval.
//!
//! Extends [`Repository`] with methods to walk the full commit graph or to
//! retrieve individual commits by OID. Filtering by branch, author, message,
//! or SHA prefix is provided via [`Repository::walk_commits_filtered`].

use std::collections::HashMap;

use serde::Serialize;

use crate::{error::GitError, repository::Repository};

/// Serialisable snapshot of a single git commit.
#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    /// Full SHA-1 OID as a hex string.
    pub oid: String,
    /// First line of the commit message.
    pub summary: String,
    /// Remainder of the commit message after the summary line.
    pub body: String,
    /// Author display name.
    pub author: String,
    /// Author e-mail address.
    pub email: String,
    /// Unix timestamp (seconds since epoch) of the author date.
    pub timestamp: i64,
    /// OIDs of parent commits (empty for root commits, two entries for merges).
    pub parents: Vec<String>,
    /// Short ref names (branches, tags) that point directly at this commit.
    pub refs: Vec<String>,
}

/// Extract a [`CommitInfo`] from a libgit2 commit and its OID.
fn commit_to_info(
    commit: &git2::Commit,
    oid: git2::Oid,
    ref_map: &std::collections::HashMap<String, Vec<String>>,
) -> CommitInfo {
    let summary = commit.summary().unwrap_or("").to_owned();
    let body = commit.body().unwrap_or("").to_owned();
    let author = commit.author().name().unwrap_or("").to_owned();
    let email = commit.author().email().unwrap_or("").to_owned();
    let timestamp = commit.time().seconds();
    let parents: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();
    let refs = ref_map.get(&oid.to_string()).cloned().unwrap_or_default();
    CommitInfo {
        oid: oid.to_string(),
        summary,
        body,
        author,
        email,
        timestamp,
        parents,
        refs,
    }
}

/// Build a map from OID (as string) → list of short ref names pointing to it.
fn build_ref_map(repo: &git2::Repository) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    if let Ok(references) = repo.references() {
        for reference in references.flatten() {
            // Resolve symbolic refs to their target
            let target_oid = if let Some(oid) = reference.target() {
                oid
            } else if let Ok(resolved) = reference.resolve() {
                if let Some(oid) = resolved.target() {
                    oid
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let shorthand = reference
                .shorthand()
                .unwrap_or_else(|| reference.name().unwrap_or("unknown"))
                .to_owned();

            map.entry(target_oid.to_string())
                .or_default()
                .push(shorthand);
        }
    }

    map
}

/// Return the short refs that point at `target` without materialising the
/// full repository-wide ref map. O(refs) but with a single pass and no map
/// allocation per OID — used by [`Repository::get_commit`] which only needs
/// the refs of a single commit.
fn refs_for_oid(repo: &git2::Repository, target: git2::Oid) -> Vec<String> {
    let Ok(references) = repo.references() else {
        return Vec::new();
    };

    let mut hits = Vec::new();
    for reference in references.flatten() {
        let oid = match reference.target() {
            Some(oid) => oid,
            None => match reference.resolve().ok().and_then(|r| r.target()) {
                Some(oid) => oid,
                None => continue,
            },
        };
        if oid != target {
            continue;
        }
        hits.push(
            reference
                .shorthand()
                .unwrap_or_else(|| reference.name().unwrap_or("unknown"))
                .to_owned(),
        );
    }
    hits
}

/// Result of [`Repository::simple_advance_commits`]: `Some((new_commits,
/// old_tip_refs))` when the range is a clean linear advance (new commits
/// newest-first, plus the refs now on the old tip), or `None` when it isn't and
/// the caller should fall back to a full rebuild.
pub type SimpleAdvance = Option<(Vec<CommitInfo>, Vec<String>)>;

/// Options controlling how [`Repository::walk_commits_with_options`] traverses
/// history.
///
/// `Default` reproduces the behaviour of [`Repository::walk_commits`]: all
/// refs are pushed as starting points and every parent edge is followed.
#[derive(Debug, Clone, Copy, Default)]
pub struct CommitWalkOptions<'a> {
    /// Follow only the first parent of each commit (like
    /// `git log --first-parent`). Commits reachable solely through second
    /// parents of merges are excluded from the walk. Note that the returned
    /// [`CommitInfo::parents`] still lists *all* parents of a merge — edge
    /// simplification for graph rendering happens in `graph-builder`.
    pub first_parent: bool,
    /// Walk only the history reachable from this branch tip instead of from
    /// every ref. Accepts local (`main`) and remote (`origin/main`) branch
    /// names, resolved like [`Repository::branch_commits`]. Errors if the
    /// branch does not exist. Composes with `first_parent` for a clean
    /// "mainline of one branch" view.
    pub branch: Option<&'a str>,
}

impl Repository {
    /// Walk all commits reachable from any ref, skipping `offset` and returning at most `max_count`.
    ///
    /// Commits are returned in topological + time order (newest first). The first
    /// `offset` commits encountered during the walk are discarded before
    /// collecting up to `max_count` results, enabling pagination/windowing over
    /// the commit graph.
    pub fn walk_commits(
        &self,
        offset: usize,
        max_count: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        self.walk_commits_with_options(offset, max_count, CommitWalkOptions::default())
    }

    /// Like [`Repository::walk_commits`] but parameterised by
    /// [`CommitWalkOptions`] (first-parent simplification, branch scoping).
    pub fn walk_commits_with_options(
        &self,
        offset: usize,
        max_count: usize,
        options: CommitWalkOptions<'_>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.inner();
        let ref_map = build_ref_map(repo);

        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        if options.first_parent {
            revwalk.simplify_first_parent()?;
        }

        if let Some(branch) = options.branch {
            // Branch-scoped: walk only from this branch's tip. Try local,
            // then remote — same resolution as `branch_commits`.
            let reference = repo
                .find_reference(&format!("refs/heads/{branch}"))
                .or_else(|_| repo.find_reference(&format!("refs/remotes/{branch}")))?;
            let oid = reference
                .target()
                .or_else(|| reference.resolve().ok().and_then(|r| r.target()))
                .ok_or_else(|| {
                    GitError::Git(git2::Error::from_str("Symbolic ref without target"))
                })?;
            revwalk.push(oid)?;
        } else {
            // Push all heads, remotes, and tags as starting points.
            if let Ok(refs) = repo.references() {
                for reference in refs.flatten() {
                    if let Some(oid) = reference.target() {
                        // Only push commit-ish objects; ignore failures silently.
                        let _ = revwalk.push(oid);
                    }
                }
            }
        }

        let mut commits = Vec::new();

        for (i, oid_result) in revwalk.enumerate() {
            if i < offset {
                continue;
            }
            if commits.len() >= max_count {
                break;
            }

            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_info(&commit, oid, &ref_map));
        }

        Ok(commits)
    }

    /// Walk commits that follow `anchor` in the full history walk, without
    /// re-enumerating the commits before it — the basis for O(limit) deep-scroll
    /// pagination.
    ///
    /// `options.branch` is ignored (the anchor defines the tip); only
    /// `options.first_parent` selects the strategy:
    ///
    /// - **first-parent (the anchored fast path's only eligible mode):** a
    ///   first-parent walk is a linear chain, so this simply follows `parent(0)`
    ///   from the anchor for `max_count` commits. That is **O(max_count)** at any
    ///   depth — it never materialises the anchor's ancestry.
    /// - **non-first-parent:** falls back to a `TOPOLOGICAL | TIME` revwalk that
    ///   pushes only the anchor. Correct results, but a `TOPOLOGICAL` revwalk
    ///   buffers the anchor's whole reachable set before yielding, so it costs
    ///   O(commits below the anchor), not O(max_count). This branch exists only
    ///   to keep the method well-defined for every option shape; production never
    ///   reaches it (see `supports_anchored_pagination`, which requires
    ///   first-parent).
    ///
    /// # Correctness
    /// The result equals `walk_commits_with_options(pos_of(anchor) + 1,
    /// max_count, options)` **only when every commit the full walk emits after
    /// `anchor` is reachable from `anchor`** — guaranteed for a first-parent
    /// single-tip walk (a linear chain), and *not* in general (a multi-head or
    /// merge walk can interleave commits unreachable from the anchor into the
    /// tail). Callers must gate on [`Repository::supports_anchored_pagination`];
    /// the `anchored_*` tests below pin down both the equal and the divergent
    /// shapes.
    pub fn walk_commits_after_with_options(
        &self,
        anchor: &str,
        max_count: usize,
        options: CommitWalkOptions<'_>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.inner();
        let ref_map = build_ref_map(repo);
        let anchor_oid = git2::Oid::from_str(anchor)?;

        if options.first_parent {
            // Linear chain: follow first parents from the anchor. O(max_count).
            let mut commits = Vec::with_capacity(max_count);
            let mut current = repo.find_commit(anchor_oid)?;
            while commits.len() < max_count {
                let Ok(parent) = current.parent(0) else {
                    break; // reached a root
                };
                let parent_oid = parent.id();
                commits.push(commit_to_info(&parent, parent_oid, &ref_map));
                current = parent;
            }
            return Ok(commits);
        }

        // Non-first-parent fallback: reproduce the sorted walk from the anchor.
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push(anchor_oid)?;

        let mut commits = Vec::with_capacity(max_count);
        for oid_result in revwalk {
            if commits.len() >= max_count {
                break;
            }
            let oid = oid_result?;
            // The anchor is always the first commit the walk yields; drop it so
            // enumeration begins with the commit that follows it.
            if oid == anchor_oid {
                continue;
            }
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_info(&commit, oid, &ref_map));
        }

        Ok(commits)
    }

    /// Whether the anchored walk ([`Repository::walk_commits_after_with_options`])
    /// is provably equal to the offset walk for these `options` on this repo.
    ///
    /// Equality holds only when the full walk is a **strictly linear chain**, so
    /// that every commit after any anchor is reachable from it. Two independent
    /// facts force that:
    /// - **`first_parent` must be set.** Under `TOPOLOGICAL | TIME`, libgit2
    ///   *intermixes* the two sides of a merge (e.g. a merge of branches A and B
    ///   emits `[m, b2, a2, b1, a1, base]`). An anchor that lands on the A side
    ///   leaves B's commits pending in the tail — and they are not reachable from
    ///   the anchor, so a non-first-parent walk diverges even on a single-head
    ///   repo. First-parent simplification collapses the graph to one line, where
    ///   the tail is always the anchor's ancestry.
    /// - **The walk must push a single tip.** A branch-scoped walk always pushes
    ///   exactly one ref; an all-refs walk is single-tip only when every ref
    ///   resolves to the same commit. With several tips, each contributes its own
    ///   first-parent chain and sibling tips interleave into the tail.
    ///
    /// Returns `Ok(false)` (→ use the offset walk) for every other shape. This is
    /// conservative: some non-linear histories happen to match, but only the
    /// linear case is *guaranteed*, and correctness must never depend on the fast
    /// path succeeding.
    pub fn supports_anchored_pagination(
        &self,
        options: CommitWalkOptions<'_>,
    ) -> Result<bool, GitError> {
        if !options.first_parent {
            return Ok(false);
        }
        if options.branch.is_some() {
            return Ok(true); // branch-scoped: exactly one tip is pushed
        }
        // All-refs walk: single-tip only when every pushed ref target is the
        // same commit. Mirrors the push set of `walk_commits_with_options`
        // (every `reference.target()` that is `Some`; symbolic refs are skipped).
        let repo = self.inner();
        let mut tip: Option<git2::Oid> = None;
        for reference in repo.references()? {
            let reference = reference?;
            if let Some(oid) = reference.target() {
                match tip {
                    None => tip = Some(oid),
                    Some(t) if t == oid => {}
                    Some(_) => return Ok(false),
                }
            }
        }
        Ok(tip.is_some())
    }

    /// Walk commits filtered by criteria, skipping `offset` raw revwalk entries.
    ///
    /// Returns commits matching ALL filters in topological + time order
    /// (newest first). The first `offset` commits produced by the revwalk are
    /// discarded before filtering; up to `max_count` matching commits are
    /// returned afterwards. This enables windowed/paginated scans over filtered
    /// history.
    pub fn walk_commits_filtered(
        &self,
        offset: usize,
        max_count: usize,
        branch_filter: Option<&str>,
        author_filter: Option<&str>,
        message_filter: Option<&str>,
        sha_filter: Option<&str>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.inner();
        let ref_map = build_ref_map(repo);
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

        if let Some(branch) = branch_filter {
            // Push only refs matching the branch filter
            let mut found = false;
            for reference in repo.references()? {
                let reference = reference?;
                if let Some(name) = reference.shorthand()
                    && name.to_lowercase().contains(&branch.to_lowercase())
                    && let Some(target) = reference.target()
                {
                    revwalk.push(target)?;
                    found = true;
                }
            }
            if !found {
                return Ok(vec![]); // No matching branches
            }
        } else {
            // Push all refs
            revwalk.push_glob("refs/heads/*")?;
            revwalk.push_glob("refs/remotes/*")?;
            revwalk.push_glob("refs/tags/*")?;
        }

        let mut commits = Vec::with_capacity(max_count);
        for (i, oid_result) in revwalk.enumerate() {
            if i < offset {
                continue;
            }
            if commits.len() >= max_count {
                break;
            }
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;

            // Apply author filter
            if let Some(author) = author_filter {
                let sig = commit.author();
                let name = sig.name().unwrap_or("");
                if !name.to_lowercase().contains(&author.to_lowercase()) {
                    continue;
                }
            }

            // Apply message filter
            if let Some(msg) = message_filter {
                let summary = commit.summary().unwrap_or("");
                if !summary.to_lowercase().contains(&msg.to_lowercase()) {
                    continue;
                }
            }

            // Apply SHA filter
            if let Some(sha) = sha_filter
                && !oid.to_string().starts_with(&sha.to_lowercase())
            {
                continue;
            }

            commits.push(commit_to_info(&commit, oid, &ref_map));
        }

        Ok(commits)
    }

    /// Walk commits reachable from a specific branch, up to `limit`.
    ///
    /// Resolves the branch name to a ref and walks from its tip.
    /// Works for both local (`main`) and remote (`origin/main`) branches.
    pub fn branch_commits(
        &self,
        branch_name: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.inner();
        let ref_map = build_ref_map(repo);

        // Try local, then remote ref
        let reference = repo
            .find_reference(&format!("refs/heads/{branch_name}"))
            .or_else(|_| repo.find_reference(&format!("refs/remotes/{branch_name}")))?;

        let oid = reference
            .target()
            .ok_or_else(|| GitError::Git(git2::Error::from_str("Symbolic ref without target")))?;

        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push(oid)?;

        let mut commits = Vec::new();
        for oid_result in revwalk {
            if commits.len() >= limit {
                break;
            }
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_info(&commit, oid, &ref_map));
        }
        Ok(commits)
    }

    /// Collect the commits a "simple advance" added to a branch: everything on
    /// the first-parent chain from `new_tip` down to (but excluding) `old_tip`,
    /// plus the refs that now point at `old_tip`.
    ///
    /// This powers the incremental graph-refresh fast path
    /// ([`graph_builder::GraphLayout::try_prepend_simple_advance`]). It returns
    /// `Ok(None)` — meaning "not a simple advance; fall back to a full rebuild"
    /// — when:
    /// - `new_tip == old_tip` (nothing advanced),
    /// - the chain from `new_tip` to `old_tip` contains any commit without
    ///   exactly one parent (a merge or a root), or
    /// - `old_tip` isn't reached within `cap` commits (a deep jump, a rebase,
    ///   or a force-push — none representable as a linear prepend).
    ///
    /// New commits come back newest-first (`[0]` is `new_tip`) with their refs
    /// resolved by the same shared ref map the full graph walk uses, so a
    /// layout built incrementally from them matches a full rebuild's node refs.
    pub fn simple_advance_commits(
        &self,
        old_tip: &str,
        new_tip: &str,
        cap: usize,
    ) -> Result<SimpleAdvance, GitError> {
        if old_tip == new_tip {
            return Ok(None);
        }
        let repo = self.inner();
        let old_oid = git2::Oid::from_str(old_tip)?;
        let mut cur = git2::Oid::from_str(new_tip)?;
        let ref_map = build_ref_map(repo);

        let mut commits = Vec::new();
        while cur != old_oid {
            if commits.len() >= cap {
                return Ok(None); // too far away — not a linear advance
            }
            let commit = repo.find_commit(cur)?;
            if commit.parent_count() != 1 {
                return Ok(None); // merge or root in the range → not simple
            }
            commits.push(commit_to_info(&commit, cur, &ref_map));
            cur = commit.parent_id(0)?;
        }
        if commits.is_empty() {
            return Ok(None);
        }
        let old_tip_refs = ref_map.get(old_tip).cloned().unwrap_or_default();
        Ok(Some((commits, old_tip_refs)))
    }

    /// Return the merge base (best common ancestor) of two revisions, or
    /// `None` when they share no common history (unrelated roots).
    ///
    /// Both inputs are resolved with `revparse_single`, so branch names,
    /// tags, `HEAD`, and abbreviated/full SHAs all work. Read-only.
    pub fn merge_base(&self, a: &str, b: &str) -> Result<Option<String>, GitError> {
        let repo = self.inner();
        let a_oid = repo.revparse_single(a)?.peel_to_commit()?.id();
        let b_oid = repo.revparse_single(b)?.peel_to_commit()?.id();
        match repo.merge_base(a_oid, b_oid) {
            Ok(oid) => Ok(Some(oid.to_string())),
            // libgit2 returns an error when the two commits are on unrelated
            // histories; surface that as "no merge base" rather than an error.
            Err(_) => Ok(None),
        }
    }

    /// Walk the commits in `from..to` — reachable from `to` but not from
    /// `from` — newest-first, returning at most `limit`.
    ///
    /// Mirrors `git log from..to` / `git rev-list from..to`: the revwalk
    /// pushes `to` and hides `from`. Both endpoints are resolved with
    /// `revparse_single`, so branch names, tags, `HEAD`, and SHAs are
    /// accepted. When `anchor` is `Some` (the OID of the last commit already
    /// shown), the walk resumes *after* it — cheap "load more" pagination
    /// over a bounded divergence without re-sending earlier pages.
    pub fn commits_between(
        &self,
        from: &str,
        to: &str,
        limit: usize,
        anchor: Option<&str>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.inner();
        let ref_map = build_ref_map(repo);

        let from_oid = repo.revparse_single(from)?.peel_to_commit()?.id();
        let to_oid = repo.revparse_single(to)?.peel_to_commit()?.id();

        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push(to_oid)?;
        revwalk.hide(from_oid)?;

        // When no anchor is given we start collecting immediately; otherwise
        // we skip until we've passed the anchor commit.
        let mut seen_anchor = anchor.is_none();
        let mut commits = Vec::with_capacity(limit.min(1024));
        for oid_result in revwalk {
            let oid = oid_result?;
            if !seen_anchor {
                if anchor == Some(oid.to_string().as_str()) {
                    seen_anchor = true;
                }
                continue;
            }
            if commits.len() >= limit {
                break;
            }
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_info(&commit, oid, &ref_map));
        }
        Ok(commits)
    }

    /// Retrieve a single commit by its OID string.
    pub fn get_commit(&self, oid_str: &str) -> Result<CommitInfo, GitError> {
        let repo = self.inner();
        let oid = git2::Oid::from_str(oid_str)?;
        let commit = repo.find_commit(oid)?;
        // Avoid `build_ref_map` here — it walks every reference in the repo
        // even though we only need the refs touching this single OID. On
        // repos with many tags, click-to-load-commit was O(refs).
        let mut ref_map: HashMap<String, Vec<String>> = HashMap::new();
        let refs = refs_for_oid(repo, oid);
        if !refs.is_empty() {
            ref_map.insert(oid.to_string(), refs);
        }
        Ok(commit_to_info(&commit, oid, &ref_map))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::create_repo_with_n_commits;

    #[test]
    fn test_walk_commits_returns_all() {
        let (_dir, path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&path).unwrap();

        let commits = repo.walk_commits(0, 100).unwrap();
        assert_eq!(commits.len(), 5, "should return all 5 commits");
    }

    #[test]
    fn test_walk_commits_respects_offset() {
        let (_dir, path) = create_repo_with_n_commits(10);
        let repo = Repository::open(&path).unwrap();

        let all = repo.walk_commits(0, 100).unwrap();
        let skipped = repo.walk_commits(3, 100).unwrap();
        assert_eq!(all.len(), 10);
        assert_eq!(skipped.len(), 7);
        assert_eq!(
            skipped[0].oid, all[3].oid,
            "offset should skip the first 3 commits"
        );
    }

    #[test]
    fn test_walk_commits_offset_beyond_total_returns_empty() {
        let (_dir, path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&path).unwrap();

        let result = repo.walk_commits(100, 100).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_walk_commits_respects_max_count() {
        let (_dir, path) = create_repo_with_n_commits(10);
        let repo = Repository::open(&path).unwrap();

        let commits = repo.walk_commits(0, 3).unwrap();
        assert_eq!(commits.len(), 3, "should respect max_count of 3");
    }

    #[test]
    fn test_walk_commits_has_parent_info() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        let commits = repo.walk_commits(0, 100).unwrap();
        assert_eq!(commits.len(), 3);

        // Newest commit (index 0) should have one parent
        assert_eq!(commits[0].parents.len(), 1, "newest commit has 1 parent");
        // Middle commit should also have one parent
        assert_eq!(commits[1].parents.len(), 1, "middle commit has 1 parent");
        // Root commit should have no parents
        assert_eq!(commits[2].parents.len(), 0, "root commit has 0 parents");

        // Parent of commit[0] should be OID of commit[1]
        assert_eq!(commits[0].parents[0], commits[1].oid);
    }

    #[test]
    fn test_get_commit_by_oid() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        let commits = repo.walk_commits(0, 100).unwrap();
        assert!(!commits.is_empty());

        // Pick the middle commit
        let target = &commits[1];
        let fetched = repo.get_commit(&target.oid).unwrap();

        assert_eq!(fetched.oid, target.oid);
        assert_eq!(fetched.summary, target.summary);
        assert_eq!(fetched.author, target.author);
        assert_eq!(fetched.parents, target.parents);
    }

    #[test]
    fn test_walk_commits_first_parent_skips_merged_branch_commits() {
        let (_dir, path) = crate::test_support::create_repo_with_merged_branch();
        let repo = Repository::open(&path).unwrap();

        // Default walk sees the whole graph: merge + feature + 2 mainline.
        let all = repo.walk_commits(0, 100).unwrap();
        assert_eq!(all.len(), 4);
        assert!(all.iter().any(|c| c.summary == "feature work"));

        // First-parent walk follows only the mainline: merge, c2, c1.
        let fp = repo
            .walk_commits_with_options(
                0,
                100,
                CommitWalkOptions {
                    first_parent: true,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            fp.len(),
            3,
            "first-parent walk must skip the feature commit"
        );
        assert!(
            !fp.iter().any(|c| c.summary == "feature work"),
            "commit reachable only via a second parent must be excluded"
        );
        // The merge commit itself stays on the mainline and keeps both parents
        // in its metadata.
        let merge = fp.iter().find(|c| c.summary == "merge feature").unwrap();
        assert_eq!(merge.parents.len(), 2);
    }

    #[test]
    fn test_walk_commits_branch_scoped_excludes_other_branches() {
        let (_dir, path) = crate::test_support::create_repo_with_merged_branch();

        // Add a "side" branch ahead of the merge so the all-refs walk picks
        // up an extra commit that a branch-scoped walk must not see.
        let head_branch = {
            let git_repo = git2::Repository::open(&path).unwrap();
            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let head = git_repo
                .find_commit(git_repo.head().unwrap().target().unwrap())
                .unwrap();
            let tree = head.tree().unwrap();
            let side_oid = git_repo
                .commit(None, &sig, &sig, "side work", &tree, &[&head])
                .unwrap();
            let side = git_repo.find_commit(side_oid).unwrap();
            git_repo.branch("side", &side, false).unwrap();
            git_repo.head().unwrap().shorthand().unwrap().to_string()
        };

        let repo = Repository::open(&path).unwrap();

        // All-refs walk: side + merge + feature + 2 mainline = 5.
        let all = repo.walk_commits(0, 100).unwrap();
        assert_eq!(all.len(), 5);

        // Scoped to the head branch: the side commit disappears.
        let scoped = repo
            .walk_commits_with_options(
                0,
                100,
                CommitWalkOptions {
                    branch: Some(&head_branch),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(scoped.len(), 4);
        assert!(!scoped.iter().any(|c| c.summary == "side work"));

        // Scoped to "side": its own commit plus everything it inherits.
        let side_scoped = repo
            .walk_commits_with_options(
                0,
                100,
                CommitWalkOptions {
                    branch: Some("side"),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(side_scoped.len(), 5);
        assert_eq!(side_scoped[0].summary, "side work");
    }

    #[test]
    fn test_walk_commits_branch_scoped_composes_with_first_parent() {
        let (_dir, path) = crate::test_support::create_repo_with_merged_branch();
        let repo = Repository::open(&path).unwrap();
        let head_branch = {
            let git_repo = git2::Repository::open(&path).unwrap();
            git_repo.head().unwrap().shorthand().unwrap().to_string()
        };

        let clean = repo
            .walk_commits_with_options(
                0,
                100,
                CommitWalkOptions {
                    first_parent: true,
                    branch: Some(&head_branch),
                },
            )
            .unwrap();
        assert_eq!(clean.len(), 3, "mainline of the branch: m, c2, c1");
        assert!(!clean.iter().any(|c| c.summary == "feature work"));
    }

    #[test]
    fn test_walk_commits_branch_scoped_unknown_branch_errors() {
        let (_dir, path) = create_repo_with_n_commits(2);
        let repo = Repository::open(&path).unwrap();
        let result = repo.walk_commits_with_options(
            0,
            100,
            CommitWalkOptions {
                branch: Some("does-not-exist"),
                ..Default::default()
            },
        );
        assert!(result.is_err(), "unknown branch must surface an error");
    }

    #[test]
    fn test_walk_commits_first_parent_default_options_match_walk_commits() {
        let (_dir, path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&path).unwrap();

        let a = repo.walk_commits(0, 100).unwrap();
        let b = repo
            .walk_commits_with_options(0, 100, CommitWalkOptions::default())
            .unwrap();
        assert_eq!(
            a.iter().map(|c| &c.oid).collect::<Vec<_>>(),
            b.iter().map(|c| &c.oid).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn test_walk_commits_filtered_by_author() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        // Filter by existing author
        let commits = repo
            .walk_commits_filtered(0, 100, None, Some("Test User"), None, None)
            .unwrap();
        assert_eq!(commits.len(), 3, "all commits are by Test User");

        // Filter by nonexistent author
        let commits = repo
            .walk_commits_filtered(0, 100, None, Some("Nonexistent"), None, None)
            .unwrap();
        assert!(commits.is_empty(), "no commits by Nonexistent");
    }

    #[test]
    fn test_walk_commits_filtered_by_message() {
        let (_dir, path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&path).unwrap();

        // Filter by message substring matching a single commit
        let commits = repo
            .walk_commits_filtered(0, 100, None, None, Some("Commit 3"), None)
            .unwrap();
        assert_eq!(commits.len(), 1, "only one commit matches 'Commit 3'");
        assert_eq!(commits[0].summary, "Commit 3");

        // Filter by common substring matching all commits
        let commits = repo
            .walk_commits_filtered(0, 100, None, None, Some("Commit"), None)
            .unwrap();
        assert_eq!(commits.len(), 5, "all commits match 'Commit'");
    }

    #[test]
    fn test_walk_commits_filtered_by_sha() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        let all = repo.walk_commits(0, 100).unwrap();
        let target = &all[1];
        let sha_prefix = &target.oid[..8];

        let commits = repo
            .walk_commits_filtered(0, 100, None, None, None, Some(sha_prefix))
            .unwrap();
        assert_eq!(commits.len(), 1, "exactly one commit matches SHA prefix");
        assert_eq!(commits[0].oid, target.oid);
    }

    #[test]
    fn test_walk_commits_filtered_by_branch() {
        let (_dir, path) = create_repo_with_n_commits(2);
        let git_repo = git2::Repository::open(&path).unwrap();

        // Create a "feature" branch from HEAD
        let head_commit = git_repo
            .find_commit(git_repo.head().unwrap().target().unwrap())
            .unwrap();
        git_repo.branch("feature", &head_commit, false).unwrap();

        // Add one more commit on "feature"
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree = git_repo
            .find_tree(git_repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        git_repo
            .commit(
                Some("refs/heads/feature"),
                &sig,
                &sig,
                "Feature commit",
                &tree,
                &[&head_commit],
            )
            .unwrap();

        let repo = Repository::open(&path).unwrap();
        let commits = repo
            .walk_commits_filtered(0, 100, Some("feature"), None, None, None)
            .unwrap();

        assert!(
            commits.len() >= 3,
            "feature branch should include its own commit plus inherited ones"
        );
        assert!(
            commits.iter().any(|c| c.summary == "Feature commit"),
            "should contain the feature-only commit"
        );
    }

    #[test]
    fn test_walk_commits_filtered_no_match() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        let commits = repo
            .walk_commits_filtered(
                0,
                100,
                Some("nonexistent-branch"),
                Some("Nobody"),
                Some("zzz"),
                Some("0000000"),
            )
            .unwrap();
        assert!(
            commits.is_empty(),
            "no commits should match all bad filters"
        );
    }

    #[test]
    fn test_branch_commits() {
        let (_dir, path) = create_repo_with_n_commits(2);
        let git_repo = git2::Repository::open(&path).unwrap();

        // Create a "feature" branch and add a commit to it
        let head_commit = git_repo
            .find_commit(git_repo.head().unwrap().target().unwrap())
            .unwrap();
        git_repo.branch("feature", &head_commit, false).unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree = git_repo
            .find_tree(git_repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        git_repo
            .commit(
                Some("refs/heads/feature"),
                &sig,
                &sig,
                "Feature work",
                &tree,
                &[&head_commit],
            )
            .unwrap();

        let repo = Repository::open(&path).unwrap();
        let commits = repo.branch_commits("feature", 100).unwrap();

        assert_eq!(commits.len(), 3, "feature has 1 own + 2 inherited commits");
        assert_eq!(commits[0].summary, "Feature work");
    }

    #[test]
    fn test_branch_commits_nonexistent() {
        let (_dir, path) = create_repo_with_n_commits(2);
        let repo = Repository::open(&path).unwrap();

        let result = repo.branch_commits("nonexistent", 100);
        assert!(result.is_err(), "nonexistent branch should return an error");
    }

    #[test]
    fn test_simple_advance_collects_linear_range() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();
        let all = repo.walk_commits(0, 100).unwrap(); // newest-first: [c3, c2, c1]
        let new_tip = &all[0].oid;
        let old_tip = &all[2].oid;

        let (commits, _refs) = repo
            .simple_advance_commits(old_tip, new_tip, 100)
            .unwrap()
            .expect("linear range is a simple advance");
        // c3 and c2 are new relative to c1; c1 (old_tip) is excluded.
        assert_eq!(commits.len(), 2);
        assert_eq!(&commits[0].oid, new_tip);
        assert_eq!(commits[1].oid, all[1].oid);
    }

    #[test]
    fn test_simple_advance_none_when_equal_or_over_cap() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();
        let all = repo.walk_commits(0, 100).unwrap();

        // Same tip → nothing advanced.
        assert!(
            repo.simple_advance_commits(&all[0].oid, &all[0].oid, 100)
                .unwrap()
                .is_none()
        );
        // old_tip unreachable within the cap → fall back.
        assert!(
            repo.simple_advance_commits(&all[2].oid, &all[0].oid, 1)
                .unwrap()
                .is_none()
        );
    }

    // ── Anchored-walk equality.
    //
    // The anchored chunk must reproduce the offset chunk at the same position
    // for every eligible walk (see `supports_anchored_pagination`: first-parent
    // + single tip = a strictly linear chain). The eligibility gate itself is
    // covered by `supports_anchored_pagination_*` below; the divergence tests
    // pin down the excluded shapes so the gate can't silently loosen.

    /// Build a single-head repo whose head is a merge of two independent
    /// branches with interleaved commit times, forcing libgit2 to intermix the
    /// two sides under `TOPOLOGICAL | TIME`. Returns the temp dir (keep alive)
    /// and path. Full-history order is `[m, b2, a2, b1, a1, base]`.
    fn interleaved_merge_repo() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let repo = git2::Repository::init(&path).unwrap();
        let tree = {
            let mut idx = repo.index().unwrap();
            let tid = idx.write_tree().unwrap();
            repo.find_tree(tid).unwrap()
        };
        let sig = |secs: i64| git2::Signature::new("T", "t@e", &git2::Time::new(secs, 0)).unwrap();
        let base = repo
            .commit(None, &sig(100), &sig(100), "base", &tree, &[])
            .unwrap();
        let bc = repo.find_commit(base).unwrap();
        let a1 = repo
            .commit(None, &sig(110), &sig(110), "a1", &tree, &[&bc])
            .unwrap();
        let b1 = repo
            .commit(None, &sig(120), &sig(120), "b1", &tree, &[&bc])
            .unwrap();
        let a1c = repo.find_commit(a1).unwrap();
        let b1c = repo.find_commit(b1).unwrap();
        let a2 = repo
            .commit(None, &sig(130), &sig(130), "a2", &tree, &[&a1c])
            .unwrap();
        let b2 = repo
            .commit(None, &sig(140), &sig(140), "b2", &tree, &[&b1c])
            .unwrap();
        let a2c = repo.find_commit(a2).unwrap();
        let b2c = repo.find_commit(b2).unwrap();
        repo.commit(
            Some("HEAD"),
            &sig(150),
            &sig(150),
            "m",
            &tree,
            &[&a2c, &b2c],
        )
        .unwrap();
        (dir, path)
    }

    /// Assert `walk_commits_after_with_options(anchor, limit)` equals
    /// `walk_commits_with_options(offset, limit)` for *every* sequential anchor
    /// position in the given walk.
    fn assert_anchored_matches_offset_everywhere(
        repo: &Repository,
        options: CommitWalkOptions<'_>,
        limit: usize,
    ) {
        let all = repo
            .walk_commits_with_options(0, 1_000_000, options)
            .unwrap();
        assert!(all.len() >= 2, "fixture too small to exercise anchors");
        for offset in 1..all.len() {
            let anchor = &all[offset - 1].oid;
            let expected: Vec<&String> = all[offset..(offset + limit).min(all.len())]
                .iter()
                .map(|c| &c.oid)
                .collect();
            let anchored = repo
                .walk_commits_after_with_options(anchor, limit, options)
                .unwrap();
            let got: Vec<&String> = anchored.iter().map(|c| &c.oid).collect();
            assert_eq!(
                got, expected,
                "anchored chunk at offset {offset} (anchor {anchor}) diverged from offset walk"
            );
        }
    }

    /// Report the first anchor position where the *non*-first-parent anchored
    /// walk diverges from the offset walk, or `None` if it never does.
    fn first_default_divergence(repo: &Repository, limit: usize) -> Option<usize> {
        let all = repo.walk_commits(0, 1_000_000).unwrap();
        (1..all.len()).find(|&offset| {
            let anchor = &all[offset - 1].oid;
            let expected: Vec<&String> = all[offset..(offset + limit).min(all.len())]
                .iter()
                .map(|c| &c.oid)
                .collect();
            let anchored = repo
                .walk_commits_after_with_options(anchor, limit, CommitWalkOptions::default())
                .unwrap();
            let got: Vec<&String> = anchored.iter().map(|c| &c.oid).collect();
            got != expected
        })
    }

    #[test]
    fn anchored_first_parent_linear_matches_offset() {
        let (_dir, path) = create_repo_with_n_commits(40);
        let repo = Repository::open(&path).unwrap();
        assert_anchored_matches_offset_everywhere(
            &repo,
            CommitWalkOptions {
                first_parent: true,
                ..Default::default()
            },
            7,
        );
    }

    #[test]
    fn anchored_first_parent_merge_heavy_matches_offset() {
        // Single-head merge fixture; first-parent collapses it to a line.
        let (_dir, path) = crate::test_support::create_repo_with_merged_branch();
        let repo = Repository::open(&path).unwrap();
        assert_anchored_matches_offset_everywhere(
            &repo,
            CommitWalkOptions {
                first_parent: true,
                ..Default::default()
            },
            3,
        );
    }

    #[test]
    fn anchored_first_parent_branch_scoped_matches_offset() {
        // Mainline with feature branches merged back — many refs, but scoping to
        // one branch and following first parents yields a single linear chain.
        let (_dir, path) = crate::test_support::create_synthetic_repo(300, 20);
        let head_branch = {
            let git_repo = git2::Repository::open(&path).unwrap();
            git_repo.head().unwrap().shorthand().unwrap().to_string()
        };
        let repo = Repository::open(&path).unwrap();
        assert_anchored_matches_offset_everywhere(
            &repo,
            CommitWalkOptions {
                first_parent: true,
                branch: Some(&head_branch),
            },
            50,
        );
    }

    #[test]
    fn anchored_first_parent_rescues_interleaved_merge() {
        // The non-first-parent walk of this fixture intermixes the two merge
        // sides and diverges (see below); first-parent flattens it to a line, so
        // the anchored walk matches the offset walk at every anchor.
        let (_dir, path) = interleaved_merge_repo();
        let repo = Repository::open(&path).unwrap();
        assert_anchored_matches_offset_everywhere(
            &repo,
            CommitWalkOptions {
                first_parent: true,
                ..Default::default()
            },
            2,
        );
    }

    #[test]
    fn anchored_single_head_merge_non_first_parent_diverges() {
        // A single-head merge of two independent branches: libgit2 emits
        // [m, b2, a2, b1, a1, base], so an anchor on the A side leaves B's
        // commits unreachable in the tail. This is why the fast path requires
        // first-parent even for single-head repos.
        let (_dir, path) = interleaved_merge_repo();
        let repo = Repository::open(&path).unwrap();
        assert_eq!(
            first_default_divergence(&repo, 100),
            Some(2),
            "single-head merge must diverge on the non-first-parent walk"
        );
    }

    #[test]
    fn anchored_multi_head_default_walk_diverges() {
        // Multiple sibling branch tips → the default all-refs walk interleaves
        // commits unreachable from a given anchor into the tail, so the fast
        // path must exclude this case.
        let (_dir, path) =
            crate::test_support::create_repo_with_branches(&["b0", "b1", "b2", "b3", "b4", "b5"]);
        let repo = Repository::open(&path).unwrap();
        assert!(
            first_default_divergence(&repo, 10).is_some(),
            "expected a multi-head divergence justifying the offset fallback"
        );
    }

    #[test]
    fn supports_anchored_pagination_requires_first_parent() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();
        // Single-head linear repo, but a non-first-parent walk is never eligible.
        assert!(
            !repo
                .supports_anchored_pagination(CommitWalkOptions::default())
                .unwrap()
        );
        // First-parent on the same single-tip repo is eligible.
        assert!(
            repo.supports_anchored_pagination(CommitWalkOptions {
                first_parent: true,
                ..Default::default()
            })
            .unwrap()
        );
    }

    #[test]
    fn supports_anchored_pagination_branch_scoped_is_single_tip() {
        let (_dir, path) = crate::test_support::create_repo_with_branches(&["b0", "b1", "b2"]);
        let repo = Repository::open(&path).unwrap();
        // Multi-head repo: the default first-parent walk pushes several tips.
        assert!(
            !repo
                .supports_anchored_pagination(CommitWalkOptions {
                    first_parent: true,
                    ..Default::default()
                })
                .unwrap()
        );
        // Scoping to one branch pushes exactly one tip → eligible.
        assert!(
            repo.supports_anchored_pagination(CommitWalkOptions {
                first_parent: true,
                branch: Some("b0"),
            })
            .unwrap()
        );
    }

    #[test]
    fn test_simple_advance_none_across_a_merge() {
        let (_dir, path) = crate::test_support::create_repo_with_merged_branch();
        let repo = Repository::open(&path).unwrap();
        // HEAD is the merge commit "m"; walking from it hits a 2-parent commit
        // immediately, so it's never a simple advance.
        let all = repo.walk_commits(0, 100).unwrap();
        let merge = all.iter().find(|c| c.summary == "merge feature").unwrap();
        let root = all.iter().find(|c| c.parents.is_empty()).unwrap();
        assert!(
            repo.simple_advance_commits(&root.oid, &merge.oid, 100)
                .unwrap()
                .is_none(),
            "a range containing a merge must not be a simple advance"
        );
    }

    // ── Compare-view range semantics (merge_base / commits_between).
    //
    // Verified against the `git` CLI on a diverged-branch fixture so the
    // engine's answers match `git merge-base` / `git rev-list` exactly.

    /// Run `git` in `path` and return trimmed stdout, panicking on failure.
    fn git_stdout(path: &std::path::Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(path)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn merge_base_matches_git_cli() {
        let (_dir, path) = crate::test_support::create_repo_with_diverged_branches();
        let repo = Repository::open(&path).unwrap();

        let expected = git_stdout(&path, &["merge-base", "main", "feature"]);
        let got = repo.merge_base("main", "feature").unwrap();
        assert_eq!(
            got,
            Some(expected),
            "merge_base must match `git merge-base`"
        );

        // Argument order is symmetric.
        assert_eq!(
            repo.merge_base("feature", "main").unwrap(),
            repo.merge_base("main", "feature").unwrap(),
        );
    }

    #[test]
    fn merge_base_none_for_unrelated_histories() {
        let (_dir, path) = crate::test_support::create_repo_with_diverged_branches();
        let repo = Repository::open(&path).unwrap();
        let git_repo = git2::Repository::open(&path).unwrap();
        // An orphan root with no shared history with `main`.
        let sig = git2::Signature::now("T", "t@e").unwrap();
        let mut tb = git_repo.treebuilder(None).unwrap();
        let blob = git_repo.blob(b"x\n").unwrap();
        tb.insert("orphan.txt", blob, 0o100644).unwrap();
        let tree = git_repo.find_tree(tb.write().unwrap()).unwrap();
        let orphan = git_repo
            .commit(None, &sig, &sig, "orphan", &tree, &[])
            .unwrap();
        git_repo
            .branch("orphan", &git_repo.find_commit(orphan).unwrap(), true)
            .unwrap();

        assert_eq!(
            repo.merge_base("main", "orphan").unwrap(),
            None,
            "unrelated histories have no merge base"
        );
    }

    #[test]
    fn commits_between_matches_git_rev_list() {
        let (_dir, path) = crate::test_support::create_repo_with_diverged_branches();
        let repo = Repository::open(&path).unwrap();

        // main..feature — the commits `feature` adds over `main`.
        let expected: Vec<String> = git_stdout(&path, &["rev-list", "main..feature"])
            .lines()
            .map(|l| l.to_string())
            .collect();
        let got: Vec<String> = repo
            .commits_between("main", "feature", 100, None)
            .unwrap()
            .into_iter()
            .map(|c| c.oid)
            .collect();
        assert_eq!(got, expected, "count + order must match `git rev-list`");
        assert_eq!(got.len(), 2, "feature adds feat_a + feat_b over main");

        // Reverse direction — the commits `main` adds over `feature`.
        let behind: Vec<String> = git_stdout(&path, &["rev-list", "feature..main"])
            .lines()
            .map(|l| l.to_string())
            .collect();
        let behind_got: Vec<String> = repo
            .commits_between("feature", "main", 100, None)
            .unwrap()
            .into_iter()
            .map(|c| c.oid)
            .collect();
        assert_eq!(behind_got, behind);
        assert_eq!(behind_got.len(), 1, "main adds one commit over feature");
    }

    #[test]
    fn commits_between_respects_limit_and_anchor() {
        let (_dir, path) = crate::test_support::create_repo_with_diverged_branches();
        let repo = Repository::open(&path).unwrap();

        // Page 1: one commit (the newest, feat_b).
        let page1 = repo.commits_between("main", "feature", 1, None).unwrap();
        assert_eq!(page1.len(), 1);

        // Page 2: resume after the last-shown OID → the next (feat_a).
        let page2 = repo
            .commits_between("main", "feature", 1, Some(&page1[0].oid))
            .unwrap();
        assert_eq!(page2.len(), 1);
        assert_ne!(page1[0].oid, page2[0].oid);

        // The concatenation equals the un-paginated walk.
        let full = repo.commits_between("main", "feature", 100, None).unwrap();
        assert_eq!(
            vec![page1[0].oid.clone(), page2[0].oid.clone()],
            full.iter().map(|c| c.oid.clone()).collect::<Vec<_>>(),
        );
    }
}
