//! Benchmarks for `walk_commits` offset + limit performance.
//!
//! By default the benches run against a **synthetic** repository generated in a
//! temp dir (a few thousand commits with merge topology) so `cargo bench`
//! produces meaningful numbers with no external fixture. To benchmark a
//! real-world repository instead, set `BEARDGIT_BENCH_REPO` to its absolute
//! path before invoking `cargo bench`:
//!
//! ```
//! BEARDGIT_BENCH_REPO=/path/to/large-repo cargo bench -p git-engine
//! ```

use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use git_engine::test_support::create_synthetic_repo;
use git_engine::{CommitWalkOptions, Repository};
use tempfile::TempDir;

/// Commits in the generated synthetic repo. Kept modest so bench runs (and the
/// generation step) stay fast while still large enough to be meaningful.
const SYNTHETIC_COMMITS: usize = 5_000;
/// Feature-branch merges spread across the synthetic history.
const SYNTHETIC_BRANCHES: usize = 50;

/// Resolve the repository to benchmark. Returns an optional `TempDir` guard
/// that the caller must keep alive for the synthetic path (dropping it deletes
/// the repo).
fn bench_repo() -> (Option<TempDir>, PathBuf) {
    match std::env::var("BEARDGIT_BENCH_REPO") {
        Ok(p) => (None, PathBuf::from(p)),
        Err(_) => {
            let (dir, path) = create_synthetic_repo(SYNTHETIC_COMMITS, SYNTHETIC_BRANCHES);
            (Some(dir), path)
        }
    }
}

fn bench_walk_offset(c: &mut Criterion) {
    let (_guard, repo_path) = bench_repo();
    let repo = Repository::open(&repo_path).expect("open bench repo");

    for &offset in &[0usize, 2_000, 4_000] {
        c.bench_function(&format!("walk offset {offset} limit 200"), |b| {
            b.iter(|| repo.walk_commits(offset, 200).unwrap())
        });
    }
}

/// Deep-scroll pagination: offset walk (O(offset)) vs anchored walk (O(limit))
/// at the same positions, in the anchored-eligible mode (first-parent over a
/// single branch tip). The offset walk re-enumerates and discards the first
/// `offset` commits; the anchored walk starts at the anchor, so its cost is flat
/// with depth. Reported side by side so the O(offset) → O(limit) win is visible.
fn bench_anchored_vs_offset(c: &mut Criterion) {
    let (_guard, repo_path) = bench_repo();
    let repo = Repository::open(&repo_path).expect("open bench repo");

    // First-parent scoped to the current branch is anchored-eligible (a single
    // linear tip). Fall back to the default mode's branch if HEAD is detached.
    let head_branch = git2::Repository::open(&repo_path)
        .ok()
        .and_then(|g| g.head().ok().and_then(|h| h.shorthand().map(str::to_owned)))
        .unwrap_or_else(|| "main".to_string());
    let opts = CommitWalkOptions {
        first_parent: true,
        branch: Some(&head_branch),
    };
    assert!(
        repo.supports_anchored_pagination(opts).unwrap_or(false),
        "bench mode must be anchored-eligible"
    );

    // Resolve the anchor OID at each offset from the full first-parent walk.
    let full = repo
        .walk_commits_with_options(0, usize::MAX, opts)
        .expect("full first-parent walk");

    for &offset in &[0usize, 2_000, 4_000] {
        if offset >= full.len() {
            continue;
        }
        c.bench_function(&format!("chunk offset walk @ {offset} limit 200"), |b| {
            b.iter(|| repo.walk_commits_with_options(offset, 200, opts).unwrap())
        });
        let anchor = full[offset.saturating_sub(1)].oid.clone();
        c.bench_function(&format!("chunk anchored walk @ {offset} limit 200"), |b| {
            b.iter(|| {
                repo.walk_commits_after_with_options(&anchor, 200, opts)
                    .unwrap()
            })
        });
    }
}

criterion_group!(benches, bench_walk_offset, bench_anchored_vs_offset);
criterion_main!(benches);
