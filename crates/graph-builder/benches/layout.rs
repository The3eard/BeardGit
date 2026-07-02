//! Benchmarks for the graph layout + viewport pipeline.
//!
//! Generates a synthetic repository via `git-engine`'s test-support helper,
//! walks it into a flat `GraphCommit` list once, then benchmarks
//! `GraphLayout::compute` (DAG build + lane assignment) and
//! `GraphLayout::viewport` (virtual-scroll slicing) over that data. No external
//! fixture is required.

use std::hint::black_box;

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
    walk_synthetic(SYNTHETIC_COMMITS, SYNTHETIC_BRANCHES)
}

fn walk_synthetic(total: usize, branches: usize) -> Vec<GraphCommit> {
    let (_dir, path) = create_synthetic_repo(total, branches);
    let repo = Repository::open(&path).expect("open synthetic repo");
    let commits = repo
        .walk_commits(0, total * 2)
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
    // Warm the lazily-built row index so the loop measures the query, not the
    // one-time build.
    let _ = layout.viewport(0, 200);
    for &offset in &[0usize, 2_000, 4_000] {
        c.bench_function(&format!("viewport offset {offset} limit 200"), |b| {
            b.iter(|| layout.viewport(offset, 200))
        });
    }
}

/// Before/after for the viewport row index, on a **dense-merge 20K layout**
/// (thousands of lane segments and merge curves). Both variants clone the same
/// visible node slice; they differ only in how they gather overlapping segments
/// and curves:
/// - "linear-scan" replays the pre-index `viewport()`: a full O(total segments +
///   curves) scan of both arrays every query.
/// - "indexed" is the shipped `viewport()`: O(window buckets + overlap) via the
///   row-bucket index.
fn bench_viewport_scan_vs_indexed(c: &mut Criterion) {
    // ~20K commits with several thousand feature-branch merges → dense curves.
    let layout = GraphLayout::compute(Dag::build(walk_synthetic(20_000, 4_000)));
    let _ = layout.viewport(0, 200); // build the index once
    eprintln!(
        "viewport bench layout: {} nodes, {} segments, {} curves",
        layout.nodes.len(),
        layout.lane_segments.len(),
        layout.merge_curves.len()
    );

    for &offset in &[0usize, 8_000, 16_000] {
        let (start, end) = (offset, offset + 200);
        // "Before": the pre-index viewport — node slice clone + full linear scan.
        c.bench_function(&format!("viewport linear-scan @ {offset} limit 200"), |b| {
            b.iter(|| {
                let n = layout.nodes.len();
                let s = start.min(n);
                let e = end.min(n);
                let nodes = layout.nodes[s..e].to_vec();
                let segs: Vec<_> = layout
                    .lane_segments
                    .iter()
                    .filter(|seg| seg.start_row < e && seg.end_row >= s)
                    .cloned()
                    .collect();
                let curves: Vec<_> = layout
                    .merge_curves
                    .iter()
                    .filter(|cu| {
                        let mn = cu.from_row.min(cu.to_row);
                        let mx = cu.from_row.max(cu.to_row);
                        mn < e && mx >= s
                    })
                    .cloned()
                    .collect();
                black_box((nodes, segs, curves))
            })
        });
        // "After": indexed viewport query.
        c.bench_function(&format!("viewport indexed @ {offset} limit 200"), |b| {
            b.iter(|| black_box(layout.viewport(offset, 200)))
        });
    }
}

criterion_group!(
    benches,
    bench_layout_compute,
    bench_incremental_advance,
    bench_viewport_slice,
    bench_viewport_scan_vs_indexed
);
criterion_main!(benches);
