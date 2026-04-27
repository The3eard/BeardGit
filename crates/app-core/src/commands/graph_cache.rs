//! Shared helper to compute (or reuse) the per-repo [`GraphLayout`].
//!
//! Encapsulates the "check persistent cache → fall back to live walk + build"
//! pipeline used by both [`super::repository::open_repo`] and
//! [`super::project::switch_project`], keeping those command handlers focused
//! on state plumbing instead of layout logistics.
//!
//! Cache semantics mirror the spec in
//! `docs/superpowers/specs/2026-04-20-persistent-graph-layout-cache.md`:
//! - Identity = `(repo_path, HEAD oid, sorted refs)` hashed to SHA-256.
//! - A mismatch, a corrupt file, a schema-version bump, or any load error is
//!   treated as a silent miss — never a user-visible failure.
//! - On a miss the live layout is always written back so the next open hits.

use std::path::Path;

use git_engine::Repository;
use graph_builder::{Dag, GraphCommit, GraphLayout};
use storage::layout_cache::{
    LayoutCacheEntry, SCHEMA_VERSION, compute_cache_key, load_layout_cache, save_layout_cache,
};
use tracing::warn;

/// Gather the cache-key material (HEAD OID + sorted `(ref_name, oid)` pairs)
/// for the currently-open repo.
fn cache_material(repo: &Repository) -> Result<(String, Vec<(String, String)>), String> {
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
    Ok((head_oid, pairs))
}

/// Walk the repo and build a fresh [`GraphLayout`] with no cache interaction.
fn build_fresh_layout(repo: &Repository) -> Result<GraphLayout, String> {
    let commits = repo.walk_commits(0, 50_000).map_err(|e| e.to_string())?;
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
    let dag = Dag::build(&graph_commits);
    Ok(GraphLayout::compute(&dag))
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
) -> Result<(GraphLayout, bool), String> {
    let (head_oid, refs) = cache_material(repo)?;
    let fresh_key = compute_cache_key(repo_path, &head_oid, &refs);

    // Try the on-disk cache. Any error is treated as a miss.
    if let Ok(Some(entry)) = load_layout_cache(config_dir, repo_path)
        && entry.cache_key == fresh_key
    {
        return Ok((entry.layout, true));
    }

    // Miss (or stale): compute and persist a fresh entry.
    let layout = build_fresh_layout(repo)?;

    let entry = LayoutCacheEntry {
        schema_version: SCHEMA_VERSION,
        cache_key: fresh_key,
        repo_path: repo_path.to_string(),
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

        let (l1, hit1) = load_or_build_layout(&repo, path_str, tmp_cfg.path()).unwrap();
        let (l2, hit2) = load_or_build_layout(&repo, path_str, tmp_cfg.path()).unwrap();

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
            let (_l, hit) = load_or_build_layout(&repo, path_str, tmp_cfg.path()).unwrap();
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
        let (layout, hit) = load_or_build_layout(&repo, path_str, tmp_cfg.path()).unwrap();
        assert!(!hit, "adding a commit should invalidate the cache");
        assert_eq!(layout.nodes.len(), 6);
    }

    #[test]
    fn corrupt_cache_falls_through_silently() {
        let (_tmp_repo, repo_path) = create_repo_with_n_commits(3);
        let tmp_cfg = tempfile::tempdir().unwrap();
        let path_str = repo_path.to_str().unwrap();

        // Plant garbage at the exact cache path.
        let cache_path = layout_cache_path(tmp_cfg.path(), path_str);
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        std::fs::write(&cache_path, b"not-json-at-all").unwrap();

        let repo = Repository::open(&repo_path).unwrap();
        let (layout, hit) = load_or_build_layout(&repo, path_str, tmp_cfg.path()).unwrap();
        assert!(!hit, "corrupt file should be a silent miss");
        assert_eq!(layout.nodes.len(), 3);

        // And the miss path should have overwritten the garbage with a fresh entry.
        let (_layout2, hit2) = load_or_build_layout(&repo, path_str, tmp_cfg.path()).unwrap();
        assert!(hit2, "third call should hit the freshly-written cache");
    }
}
