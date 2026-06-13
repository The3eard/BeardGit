//! Shared helper to compute (or reuse) the per-repo [`GraphLayout`].
//!
//! Encapsulates the "check persistent cache → fall back to live walk + build"
//! pipeline used by both [`super::repository::open_repo`] and
//! [`super::project::switch_project`], keeping those command handlers focused
//! on state plumbing instead of layout logistics.
//!
//! Cache semantics mirror the spec in
//! `docs/superpowers/specs/2026-04-20-persistent-graph-layout-cache.md`:
//! - Identity = `(repo_path, HEAD oid, sorted refs, layout variant,
//!   state fingerprint)` hashed to SHA-256, where the fingerprint is the
//!   capped reachable-commit count plus the HEAD tree OID (guards against
//!   reachability changes that move no ref — shallow deepening, grafts).
//! - A mismatch, a corrupt file, a schema-version bump, or any load error is
//!   treated as a silent miss — never a user-visible failure.
//! - On a miss the live layout is always written back so the next open hits.

use std::path::Path;

use git_engine::{CommitWalkOptions, Repository};
use graph_builder::{Dag, GraphCommit, GraphLayout};
use storage::layout_cache::{
    LayoutCacheEntry, SCHEMA_VERSION, compute_cache_key, load_layout_cache, save_layout_cache,
};
use tracing::warn;

/// Options that shape how a repo's [`GraphLayout`] is computed.
///
/// Carried alongside the layout in [`crate::state::ProjectSlot`] so viewport
/// commands can tell whether the cached layout matches the mode the frontend
/// is asking for. `Default` is the classic full-graph view.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphLayoutOptions {
    /// Follow only the first parent of each commit: merges collapse onto the
    /// mainline and commits reachable solely through second parents are
    /// excluded from the walk.
    pub first_parent: bool,
    /// Show only the history reachable from this branch tip (local `main` or
    /// remote `origin/main`) instead of from every ref. Composes with
    /// `first_parent` for a clean single-branch mainline view.
    pub branch: Option<String>,
    /// Lane ceiling override, already normalized via
    /// [`GraphLayoutOptions::normalize_max_lanes`]: clamped to
    /// `MIN_LANE_CEILING..=MAX_LANE_CEILING` and `None` when equal to
    /// [`graph_builder::DEFAULT_MAX_LANES`] (so an explicit default shares
    /// the default cache slot).
    pub max_lanes: Option<u8>,
}

/// Lowest lane ceiling the frontend may request.
pub const MIN_LANE_CEILING: u8 = 4;
/// Highest lane ceiling the frontend may request.
pub const MAX_LANE_CEILING: u8 = 16;

impl GraphLayoutOptions {
    /// Stable discriminator string used in the persistent cache key and the
    /// per-variant cache file name. Empty for the default option set so
    /// pre-existing cache entries stay valid.
    pub fn variant(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.first_parent {
            parts.push("fp=1".to_string());
        }
        if let Some(branch) = &self.branch {
            parts.push(format!("branch={branch}"));
        }
        if let Some(lanes) = self.max_lanes {
            parts.push(format!("lanes={lanes}"));
        }
        parts.join(";")
    }

    /// Clamp a raw frontend-supplied lane ceiling into
    /// `MIN_LANE_CEILING..=MAX_LANE_CEILING` and collapse the default value
    /// to `None` so it can't create a duplicate cache slot.
    pub fn normalize_max_lanes(raw: Option<u8>) -> Option<u8> {
        raw.map(|n| n.clamp(MIN_LANE_CEILING, MAX_LANE_CEILING))
            .filter(|&n| usize::from(n) != graph_builder::DEFAULT_MAX_LANES)
    }

    /// Effective lane ceiling for the layout computation.
    pub(crate) fn lane_ceiling(&self) -> usize {
        self.max_lanes
            .map(usize::from)
            .unwrap_or(graph_builder::DEFAULT_MAX_LANES)
    }

    /// View of these options as git-engine walk options.
    pub(crate) fn walk_options(&self) -> CommitWalkOptions<'_> {
        CommitWalkOptions {
            first_parent: self.first_parent,
            branch: self.branch.as_deref(),
        }
    }
}

/// Material identifying the repo state for cache-key purposes.
struct CacheMaterial {
    /// HEAD OID (empty string for a bare/empty repo).
    head_oid: String,
    /// `(ref_name, oid)` pairs for every resolvable ref (sorted inside
    /// `compute_cache_key`).
    refs: Vec<(String, String)>,
    /// State fingerprint — see [`state_fingerprint`].
    fingerprint: String,
}

/// Gather the cache-key material (HEAD OID + sorted `(ref_name, oid)` pairs
/// + repo-state fingerprint) for the currently-open repo.
fn cache_material(repo: &Repository) -> Result<CacheMaterial, String> {
    let inner = repo.inner();
    // HEAD OID — falls back to an empty string for a bare/empty repo so the
    // key still changes when the first commit is made.
    let head_oid = inner
        .head()
        .ok()
        .and_then(|h| h.target())
        .map(|o| o.to_string())
        .unwrap_or_default();

    let mut pairs: Vec<(String, String)> = Vec::new();
    let refs = inner.references().map_err(|e| e.to_string())?;
    for r in refs.flatten() {
        let name = r.name().unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let target_oid = if let Some(oid) = r.target() {
            oid
        } else if let Ok(resolved) = r.resolve() {
            match resolved.target() {
                Some(oid) => oid,
                None => continue,
            }
        } else {
            continue;
        };
        pairs.push((name, target_oid.to_string()));
    }
    let fingerprint = state_fingerprint(inner);
    Ok(CacheMaterial {
        head_oid,
        refs: pairs,
        fingerprint,
    })
}

/// Cheap fingerprint of repo state that `(HEAD oid, refs)` alone can miss.
///
/// Two components:
/// - `count` — number of commits reachable from all refs, capped at
///   [`MAX_INITIAL_LAYOUT_COMMITS`] + 1 (states differing only beyond the
///   layout cap produce identical layouts, so counting further would cost
///   time without adding discrimination). This catches reachability changes
///   that leave every ref OID untouched — e.g. a shallow clone being
///   deepened, grafts, or odb surgery — which previously produced a false
///   cache hit.
/// - `tree` — the HEAD commit's tree OID, a belt-and-braces guard for
///   exotic flows (e.g. `reset --soft` round-trips combined with object
///   replacement) where HEAD's OID survives but its content identity must
///   be re-checked.
///
/// Best-effort: any walk error degrades to `count=0`, which still yields a
/// stable (if conservative) key.
fn state_fingerprint(inner: &git2::Repository) -> String {
    let count = (|| -> Result<usize, git2::Error> {
        let mut revwalk = inner.revwalk()?;
        for reference in inner.references()?.flatten() {
            if let Some(oid) = reference.target() {
                let _ = revwalk.push(oid);
            }
        }
        Ok(revwalk.take(MAX_INITIAL_LAYOUT_COMMITS + 1).count())
    })()
    .unwrap_or(0);

    let tree_oid = inner
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| inner.find_commit(oid).ok())
        .map(|c| c.tree_id().to_string())
        .unwrap_or_default();

    format!("count={count};tree={tree_oid}")
}

/// Maximum number of commits walked when building the in-memory layout
/// for the active project's `ProjectSlot`.
///
/// Trade-off:
/// - Higher cap → more history visible without scrolling beyond, but
///   `open_repo` (cache miss) and `refresh_graph_layout` (after every
///   ref change) become proportionally slower and pin more RAM.
/// - Lower cap → instant first paint and a smaller layout, at the cost
///   of older history needing `load_graph_chunk` on demand from the
///   frontend (which is wired to call it on scroll past the cached
///   range).
///
/// 20 000 is enough to cover the working set most users actually scroll
/// through (a few thousand commits at most), while keeping the walk
/// well under a second on a typical mechanical-disk repo and the
/// resulting `GraphLayout` under ~50 MB serialized. Repos that need
/// older commits than this still get them via `load_graph_chunk` —
/// they just don't pay the cost up front.
const MAX_INITIAL_LAYOUT_COMMITS: usize = 20_000;

/// Walk the repo and build a fresh [`GraphLayout`] with no cache interaction.
fn build_fresh_layout(
    repo: &Repository,
    options: &GraphLayoutOptions,
) -> Result<GraphLayout, String> {
    let commits = repo
        .walk_commits_with_options(0, MAX_INITIAL_LAYOUT_COMMITS, options.walk_options())
        .map_err(|e| e.to_string())?;
    let graph_commits: Vec<GraphCommit> = commits
        .iter()
        .map(|c| GraphCommit {
            oid: c.oid.clone(),
            parents: c.parents.clone(),
            timestamp: c.timestamp,
            refs: c.refs.clone(),
            summary: c.summary.clone(),
            author: c.author.clone(),
            email: c.email.clone(),
        })
        .collect();
    let dag = if options.first_parent {
        Dag::build_first_parent(graph_commits)
    } else {
        Dag::build(graph_commits)
    };
    Ok(GraphLayout::compute_with_max_lanes(
        dag,
        options.lane_ceiling(),
    ))
}

/// Build the graph layout for a repo, consulting the persistent cache first.
///
/// Returns the layout plus a `was_cached` flag so callers can log or
/// instrument cache hits. On any cache failure (miss, corrupt, mismatch)
/// falls back to a live walk + compute and writes a fresh cache entry.
///
/// The write is best-effort: any error is logged at `warn!` and discarded so
/// an unwritable config dir can't block repo opens. On a hit the returned
/// [`GraphLayout`] is the exact layout that was on disk — callers can trust
/// it to be byte-identical to what the live path would have produced the
/// last time HEAD + refs matched.
pub(crate) fn load_or_build_layout(
    repo: &Repository,
    repo_path: &str,
    config_dir: &Path,
    options: &GraphLayoutOptions,
) -> Result<(GraphLayout, bool), String> {
    let CacheMaterial {
        head_oid,
        refs,
        fingerprint,
    } = cache_material(repo)?;
    let variant = options.variant();
    // The key mixes in the state fingerprint (commit count + HEAD tree) so
    // reachability changes that don't move any ref still miss. The file
    // path, however, uses only the mode `variant` — the fingerprint changes
    // with every repo state and must not multiply cache files.
    let key_variant = if variant.is_empty() {
        fingerprint
    } else {
        format!("{variant};{fingerprint}")
    };
    let fresh_key = compute_cache_key(repo_path, &head_oid, &refs, &key_variant);

    // Try the on-disk cache. Any error is treated as a miss.
    if let Ok(Some(entry)) = load_layout_cache(config_dir, repo_path, &variant)
        && entry.cache_key == fresh_key
    {
        return Ok((entry.layout, true));
    }

    // Miss (or stale): compute and persist a fresh entry.
    let layout = build_fresh_layout(repo, options)?;

    let entry = LayoutCacheEntry {
        schema_version: SCHEMA_VERSION,
        cache_key: fresh_key,
        repo_path: repo_path.to_string(),
        variant,
        head_oid,
        generated_at: chrono::Utc::now().to_rfc3339(),
        layout: layout.clone(),
    };
    persist_entry_async(config_dir.to_path_buf(), entry);

    Ok((layout, false))
}

/// Write a freshly-computed [`LayoutCacheEntry`] to disk without blocking the
/// caller.
///
/// When a Tokio runtime is available we hand the serialize + write off to a
/// dedicated `spawn_blocking` worker so the critical path (which is itself
/// typically running inside a `spawn_blocking` closure) returns the layout
/// immediately. In non-async contexts (notably unit tests) we fall back to a
/// synchronous write so tests remain deterministic without needing to pump
/// the runtime.
fn persist_entry_async(config_dir: std::path::PathBuf, entry: LayoutCacheEntry) {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn_blocking(move || {
                if let Err(e) = save_layout_cache(&config_dir, &entry) {
                    warn!(error = %e, "failed to persist graph layout cache");
                }
            });
        }
        Err(_) => {
            if let Err(e) = save_layout_cache(&config_dir, &entry) {
                warn!(error = %e, "failed to persist graph layout cache");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // The tests below run as plain `#[test]` functions, so no Tokio runtime is
    // active. `persist_entry_async` detects that and falls back to a
    // synchronous write — the cache file is on disk by the time
    // `load_or_build_layout` returns. That keeps the second-call hit assertion
    // deterministic without having to pump a runtime.

    use super::*;
    use git_engine::test_support::create_repo_with_n_commits;
    use storage::layout_cache::layout_cache_path;

    #[test]
    fn second_call_hits_cache() {
        let (_tmp_repo, repo_path) = create_repo_with_n_commits(20);
        let repo = Repository::open(&repo_path).unwrap();
        let tmp_cfg = tempfile::tempdir().unwrap();
        let path_str = repo_path.to_str().unwrap();

        let opts = GraphLayoutOptions::default();
        let (l1, hit1) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &opts).unwrap();
        let (l2, hit2) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &opts).unwrap();

        assert!(!hit1, "first call should be a miss");
        assert!(hit2, "second call should be a hit");
        assert_eq!(l1.nodes.len(), l2.nodes.len());
        assert_eq!(l1.nodes.len(), 20);
    }

    #[test]
    fn cache_misses_when_head_changes() {
        let (_tmp_repo, repo_path) = create_repo_with_n_commits(5);
        let tmp_cfg = tempfile::tempdir().unwrap();
        let path_str = repo_path.to_str().unwrap();

        // Initial miss → cache gets written.
        {
            let repo = Repository::open(&repo_path).unwrap();
            let (_l, hit) = load_or_build_layout(
                &repo,
                path_str,
                tmp_cfg.path(),
                &GraphLayoutOptions::default(),
            )
            .unwrap();
            assert!(!hit);
        }

        // Make a new commit so HEAD + refs move forward.
        {
            let git_repo = git2::Repository::open(&repo_path).unwrap();
            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let parent = git_repo
                .find_commit(git_repo.head().unwrap().target().unwrap())
                .unwrap();
            let tree = git_repo
                .find_tree(git_repo.index().unwrap().write_tree().unwrap())
                .unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "extra", &tree, &[&parent])
                .unwrap();
        }

        // Reopening the repo now sees the new HEAD; cache must miss.
        let repo = Repository::open(&repo_path).unwrap();
        let (layout, hit) = load_or_build_layout(
            &repo,
            path_str,
            tmp_cfg.path(),
            &GraphLayoutOptions::default(),
        )
        .unwrap();
        assert!(!hit, "adding a commit should invalidate the cache");
        assert_eq!(layout.nodes.len(), 6);
    }

    #[test]
    fn first_parent_mode_builds_and_caches_separately() {
        let (_tmp_repo, repo_path) = git_engine::test_support::create_repo_with_merged_branch();
        let repo = Repository::open(&repo_path).unwrap();
        let tmp_cfg = tempfile::tempdir().unwrap();
        let path_str = repo_path.to_str().unwrap();

        let default_opts = GraphLayoutOptions::default();
        let fp_opts = GraphLayoutOptions {
            first_parent: true,
            ..Default::default()
        };

        // Warm both variants.
        let (full, hit_full) =
            load_or_build_layout(&repo, path_str, tmp_cfg.path(), &default_opts).unwrap();
        let (fp, hit_fp) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &fp_opts).unwrap();
        assert!(!hit_full);
        assert!(!hit_fp, "fp variant must not hit the default-variant entry");

        // Full graph: 4 commits across 2 lanes. First-parent: 3 commits, 1 lane.
        assert_eq!(full.nodes.len(), 4);
        assert_eq!(full.lane_count, 2);
        assert_eq!(fp.nodes.len(), 3);
        assert_eq!(fp.lane_count, 1);
        assert!(
            !fp.nodes.iter().any(|n| n.summary == "feature work"),
            "merged-branch commit must be absent in first-parent mode"
        );

        // Both variants now hit independently.
        let (_l, hit_full2) =
            load_or_build_layout(&repo, path_str, tmp_cfg.path(), &default_opts).unwrap();
        let (_l, hit_fp2) =
            load_or_build_layout(&repo, path_str, tmp_cfg.path(), &fp_opts).unwrap();
        assert!(hit_full2);
        assert!(hit_fp2);
    }

    #[test]
    fn normalize_max_lanes_clamps_and_collapses_default() {
        assert_eq!(GraphLayoutOptions::normalize_max_lanes(None), None);
        // The default ceiling collapses to None — no duplicate cache slot.
        assert_eq!(GraphLayoutOptions::normalize_max_lanes(Some(8)), None);
        // In-range values pass through.
        assert_eq!(GraphLayoutOptions::normalize_max_lanes(Some(12)), Some(12));
        assert_eq!(GraphLayoutOptions::normalize_max_lanes(Some(4)), Some(4));
        assert_eq!(GraphLayoutOptions::normalize_max_lanes(Some(16)), Some(16));
        // Out-of-range values clamp to 4..=16.
        assert_eq!(GraphLayoutOptions::normalize_max_lanes(Some(1)), Some(4));
        assert_eq!(GraphLayoutOptions::normalize_max_lanes(Some(99)), Some(16));
    }

    #[test]
    fn max_lanes_variant_gets_own_cache_slot() {
        let (_tmp_repo, repo_path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&repo_path).unwrap();
        let tmp_cfg = tempfile::tempdir().unwrap();
        let path_str = repo_path.to_str().unwrap();

        let wide_opts = GraphLayoutOptions {
            max_lanes: Some(16),
            ..Default::default()
        };
        let (_l, hit1) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &wide_opts).unwrap();
        assert!(!hit1);
        let (_l, hit2) = load_or_build_layout(
            &repo,
            path_str,
            tmp_cfg.path(),
            &GraphLayoutOptions::default(),
        )
        .unwrap();
        assert!(!hit2, "default options must not hit the lanes=16 entry");
        let (_l, hit3) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &wide_opts).unwrap();
        assert!(hit3);
    }

    #[test]
    fn cache_material_includes_commit_count_and_head_tree() {
        let (_tmp_repo, repo_path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&repo_path).unwrap();

        let material = cache_material(&repo).unwrap();

        let git_repo = git2::Repository::open(&repo_path).unwrap();
        let head_commit = git_repo
            .find_commit(git_repo.head().unwrap().target().unwrap())
            .unwrap();
        assert_eq!(material.head_oid, head_commit.id().to_string());
        assert_eq!(
            material.fingerprint,
            format!("count=5;tree={}", head_commit.tree_id()),
            "fingerprint must carry the reachable count and HEAD tree OID"
        );
    }

    #[test]
    fn cache_misses_when_reachability_changes_without_ref_moves() {
        // A shallow boundary changes which commits are reachable while every
        // ref OID (and HEAD) stays identical — exactly the false-hit class
        // the fingerprint guards against. Simulate it by writing
        // `.git/shallow` by hand, the same on-disk state a shallow fetch
        // leaves behind.
        let (_tmp_repo, repo_path) = create_repo_with_n_commits(5);
        let tmp_cfg = tempfile::tempdir().unwrap();
        let path_str = repo_path.to_str().unwrap();
        let opts = GraphLayoutOptions::default();

        // Warm the cache with the full 5-commit state.
        {
            let repo = Repository::open(&repo_path).unwrap();
            let (_l, hit) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &opts).unwrap();
            assert!(!hit);
            let (_l, hit2) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &opts).unwrap();
            assert!(hit2, "sanity: unchanged repo must hit");
        }

        // Mark the 3rd commit as a shallow boundary. Refs and HEAD are
        // untouched.
        let boundary = {
            let git_repo = git2::Repository::open(&repo_path).unwrap();
            let mut revwalk = git_repo.revwalk().unwrap();
            revwalk.push_head().unwrap();
            revwalk.nth(2).unwrap().unwrap()
        };
        std::fs::write(repo_path.join(".git/shallow"), format!("{boundary}\n")).unwrap();

        let repo = Repository::open(&repo_path).unwrap();
        let material_after = cache_material(&repo).unwrap();
        let (_l, hit) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &opts).unwrap();

        // If libgit2 honors the shallow file the reachable count shrinks; the
        // key must change and the cache must miss. (HEAD itself is untouched
        // either way — assert that to prove refs alone wouldn't have caught
        // this.)
        let git_repo = git2::Repository::open(&repo_path).unwrap();
        assert_eq!(
            material_after.head_oid,
            git_repo.head().unwrap().target().unwrap().to_string(),
            "HEAD must be unchanged by the shallow boundary"
        );
        let head_tree = git_repo
            .find_commit(git_repo.head().unwrap().target().unwrap())
            .unwrap()
            .tree_id();
        assert_eq!(
            material_after.fingerprint,
            format!("count=3;tree={head_tree}")
        );
        assert!(
            !hit,
            "reachability change without ref movement must invalidate the cache"
        );
    }

    #[test]
    fn branch_scoped_mode_builds_and_caches_separately() {
        let (_tmp_repo, repo_path) = git_engine::test_support::create_repo_with_merged_branch();
        let repo = Repository::open(&repo_path).unwrap();
        let tmp_cfg = tempfile::tempdir().unwrap();
        let path_str = repo_path.to_str().unwrap();

        let head_branch = {
            let git_repo = git2::Repository::open(&repo_path).unwrap();
            git_repo.head().unwrap().shorthand().unwrap().to_string()
        };
        let scoped_opts = GraphLayoutOptions {
            branch: Some(head_branch.clone()),
            ..Default::default()
        };
        let clean_opts = GraphLayoutOptions {
            first_parent: true,
            branch: Some(head_branch),
            ..Default::default()
        };

        let (scoped, hit1) =
            load_or_build_layout(&repo, path_str, tmp_cfg.path(), &scoped_opts).unwrap();
        assert!(!hit1);
        assert_eq!(scoped.nodes.len(), 4);

        // branch + first_parent composes into the clean mainline view and
        // gets its own cache slot.
        let (clean, hit2) =
            load_or_build_layout(&repo, path_str, tmp_cfg.path(), &clean_opts).unwrap();
        assert!(!hit2, "composed variant must not hit the branch-only entry");
        assert_eq!(clean.nodes.len(), 3);
        assert_eq!(clean.lane_count, 1);

        // Each variant hits its own entry afterwards.
        let (_l, hit3) =
            load_or_build_layout(&repo, path_str, tmp_cfg.path(), &scoped_opts).unwrap();
        let (_l, hit4) =
            load_or_build_layout(&repo, path_str, tmp_cfg.path(), &clean_opts).unwrap();
        assert!(hit3);
        assert!(hit4);
    }

    #[test]
    fn corrupt_cache_falls_through_silently() {
        let (_tmp_repo, repo_path) = create_repo_with_n_commits(3);
        let tmp_cfg = tempfile::tempdir().unwrap();
        let path_str = repo_path.to_str().unwrap();

        // Plant garbage at the exact cache path.
        let cache_path = layout_cache_path(tmp_cfg.path(), path_str, "");
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        std::fs::write(&cache_path, b"not-json-at-all").unwrap();

        let opts = GraphLayoutOptions::default();
        let repo = Repository::open(&repo_path).unwrap();
        let (layout, hit) = load_or_build_layout(&repo, path_str, tmp_cfg.path(), &opts).unwrap();
        assert!(!hit, "corrupt file should be a silent miss");
        assert_eq!(layout.nodes.len(), 3);

        // And the miss path should have overwritten the garbage with a fresh entry.
        let (_layout2, hit2) =
            load_or_build_layout(&repo, path_str, tmp_cfg.path(), &opts).unwrap();
        assert!(hit2, "third call should hit the freshly-written cache");
    }
}
