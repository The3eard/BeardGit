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
use git_engine::Repository;
use git_engine::test_support::create_synthetic_repo;
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

criterion_group!(benches, bench_walk_offset);
criterion_main!(benches);
