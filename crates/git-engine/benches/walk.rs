//! Benchmarks for `walk_commits` offset + limit performance.
//!
//! Run against a real repository by setting `BEARDGIT_BENCH_REPO` to an
//! absolute path before invoking `cargo bench`. Without the env var, the
//! benches are skipped (so CI and casual `cargo bench` runs don't fail
//! just because no fixture exists).
//!
//! Example:
//! ```
//! BEARDGIT_BENCH_REPO=/path/to/large-repo cargo bench -p git-engine
//! ```

use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use git_engine::Repository;

fn bench_walk_offset(c: &mut Criterion) {
    let repo_path = match std::env::var("BEARDGIT_BENCH_REPO") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            eprintln!("BEARDGIT_BENCH_REPO unset; skipping walk benches");
            return;
        }
    };
    let repo = Repository::open(repo_path).expect("open bench repo");

    c.bench_function("walk first 200", |b| {
        b.iter(|| repo.walk_commits(0, 200).unwrap())
    });
    c.bench_function("walk offset 10000 limit 200", |b| {
        b.iter(|| repo.walk_commits(10_000, 200).unwrap())
    });
}

criterion_group!(benches, bench_walk_offset);
criterion_main!(benches);
