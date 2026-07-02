//! Benchmarks for the graph layout + viewport pipeline.
//!
//! Generates a synthetic repository via `git-engine`'s test-support helper,
//! walks it into a flat `GraphCommit` list once, then benchmarks
//! `GraphLayout::compute` (DAG build + lane assignment) and
//! `GraphLayout::viewport` (virtual-scroll slicing) over that data. No external
//! fixture is required.

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use git_engine::Repository;
use git_engine::test_support::create_synthetic_repo;
use graph_builder::{Dag, GraphCommit, GraphLayout};

/// Commits in the generated synthetic repo. Graph layout is pure CPU, so this
/// is comfortably fast while still exercising multi-lane merge topology.
const SYNTHETIC_COMMITS: usize = 5_000;
/// Feature-branch merges spread across the synthetic history.
const SYNTHETIC_BRANCHES: usize = 50;

/// Build the synthetic repo and walk it into a flat `GraphCommit` list. The
/// temp dir is dropped once the commits are in memory.
fn synthetic_commits() -> Vec<GraphCommit> {
    let (_dir, path) = create_synthetic_repo(SYNTHETIC_COMMITS, SYNTHETIC_BRANCHES);
    let repo = Repository::open(&path).expect("open synthetic repo");
    let commits = repo
        .walk_commits(0, SYNTHETIC_COMMITS * 2)
        .expect("walk synthetic repo");
    commits
        .into_iter()
        .map(|c| GraphCommit {
            oid: c.oid,
            parents: c.parents,
            timestamp: c.timestamp,
            refs: c.refs,
            summary: c.summary,
            author: c.author,
            email: c.email,
        })
        .collect()
}

fn bench_layout_compute(c: &mut Criterion) {
    let commits = synthetic_commits();
    c.bench_function("layout compute 5000", |b| {
        b.iter_batched(
            || commits.clone(),
            |commits| GraphLayout::compute(Dag::build(commits)),
            BatchSize::SmallInput,
        )
    });
}

fn bench_incremental_advance(c: &mut Criterion) {
    // Cost of the graph-refresh fast path: splice one new commit onto an
    // already-built 5000-commit layout, versus rebuilding the whole layout
    // (`bench_layout_compute` above, which is itself only the compute half —
    // the full refresh also re-walks the repo). One new commit is the common
    // "plain commit" case.
    let commits = synthetic_commits();
    let base = GraphLayout::compute(Dag::build(commits));
    let tip = base.nodes[0].oid.clone();
    let new_commits = vec![GraphCommit {
        oid: "newtip0000000000000000000000000000000000".to_string(),
        parents: vec![tip],
        timestamp: base.nodes[0].timestamp + 1,
        refs: Vec::new(),
        summary: "new commit".to_string(),
        author: String::new(),
        email: String::new(),
    }];
    c.bench_function("incremental advance +1 over 5000", |b| {
        b.iter(|| {
            base.try_prepend_simple_advance(&new_commits, Vec::new())
                .expect("simple advance applies")
        })
    });
}

fn bench_viewport_slice(c: &mut Criterion) {
    let commits = synthetic_commits();
    let layout = GraphLayout::compute(Dag::build(commits));
    for &offset in &[0usize, 2_000, 4_000] {
        c.bench_function(&format!("viewport offset {offset} limit 200"), |b| {
            b.iter(|| layout.viewport(offset, 200))
        });
    }
}

criterion_group!(
    benches,
    bench_layout_compute,
    bench_incremental_advance,
    bench_viewport_slice
);
criterion_main!(benches);
