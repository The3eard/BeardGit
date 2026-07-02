//! Property test: the incremental "simple advance" prepend
//! ([`GraphLayout::try_prepend_simple_advance`]) must produce a layout that is
//! structurally identical to a full [`GraphLayout::compute`] rebuild — the
//! correctness gate for the graph-refresh fast path (spec 04, Phase C).
//!
//! For each of three topologies (linear, merge-heavy, multi-branch) we build a
//! synthetic repo, then apply a sequence of git mutations. After every mutation
//! we replay the exact runtime detection (one branch ref moved, its old tip was
//! the layout's row-0 commit, the new commits form a single-parent chain) and,
//! whenever it fires, assert the incremental layout equals a full rebuild.
//! Mutations that don't match the shape (merges, new branches, moves off the
//! tip) are expected to fall back and are simply not asserted — proving the
//! detector doesn't over-fire.

use std::collections::BTreeMap;

use git_engine::Repository;
use git_engine::test_support::{create_repo_with_n_commits, create_synthetic_repo};
use git2::{Repository as G2Repo, Signature, Time};
use graph_builder::{Dag, GraphCommit, GraphLayout, structural_diff};

/// Upper bound on how far the fast path will walk before giving up.
const CAP: usize = 512;

/// Monotonic author time so the newest commit is unambiguously row 0 (avoids
/// same-second ties with the synthetic-repo setup commits, which use `now()`
/// ~1.7e9). Starting well above that keeps every mutation strictly newest.
fn sig_at(secs: i64) -> Signature<'static> {
    Signature::new("Test User", "test@example.com", &Time::new(secs, 0)).unwrap()
}

/// Full-rebuild ground truth: walk every ref and lay it out exactly as the
/// production `build_fresh_layout` does for the default options.
fn full_layout(path: &std::path::Path) -> GraphLayout {
    let repo = Repository::open(path).unwrap();
    let commits = repo.walk_commits(0, 100_000).unwrap();
    let gcs: Vec<GraphCommit> = commits.into_iter().map(to_gc).collect();
    GraphLayout::compute(Dag::build(gcs))
}

fn to_gc(c: git_engine::CommitInfo) -> GraphCommit {
    GraphCommit {
        oid: c.oid,
        parents: c.parents,
        timestamp: c.timestamp,
        refs: c.refs,
        summary: c.summary,
        author: c.author,
        email: c.email,
    }
}

/// `references()` name → target OID, skipping symbolic refs (e.g. HEAD) — the
/// same shape the runtime and `mutation_events::Snapshot` compare on.
fn ref_snapshot(path: &std::path::Path) -> BTreeMap<String, String> {
    let repo = G2Repo::open(path).unwrap();
    let mut map = BTreeMap::new();
    for r in repo.references().unwrap().flatten() {
        if let (Some(name), Some(oid)) = (r.name(), r.target()) {
            map.insert(name.to_string(), oid.to_string());
        }
    }
    map
}

/// Exactly one ref changed OID (none added/removed) → `(name, old, new)`.
fn single_ref_move(
    old: &BTreeMap<String, String>,
    new: &BTreeMap<String, String>,
) -> Option<(String, String, String)> {
    if old.len() != new.len() {
        return None;
    }
    let mut moved = None;
    for (name, new_oid) in new {
        match old.get(name) {
            None => return None, // added
            Some(old_oid) if old_oid == new_oid => {}
            Some(old_oid) => {
                if moved.is_some() {
                    return None; // more than one moved
                }
                moved = Some((name.clone(), old_oid.clone(), new_oid.clone()));
            }
        }
    }
    moved
}

fn is_branch(name: &str) -> bool {
    name.starts_with("refs/heads/") || name.starts_with("refs/remotes/")
}

/// Commit an (empty-tree) commit onto the current branch via HEAD.
fn commit_on_head(path: &std::path::Path, secs: i64, msg: &str) {
    let repo = G2Repo::open(path).unwrap();
    let sig = sig_at(secs);
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = parent.tree().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent])
        .unwrap();
}

/// Advance a non-HEAD local branch by one commit, leaving HEAD put.
fn commit_on_branch(path: &std::path::Path, branch: &str, secs: i64, msg: &str) {
    let repo = G2Repo::open(path).unwrap();
    let sig = sig_at(secs);
    let full = format!("refs/heads/{branch}");
    let tip = repo
        .find_reference(&full)
        .unwrap()
        .peel_to_commit()
        .unwrap();
    let tree = tip.tree().unwrap();
    let oid = repo.commit(None, &sig, &sig, msg, &tree, &[&tip]).unwrap();
    repo.reference(&full, oid, true, "advance").unwrap();
}

/// Create a new branch at HEAD (adds a ref; nothing moves forward).
fn create_branch(path: &std::path::Path, name: &str) {
    let repo = G2Repo::open(path).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let _ = repo.branch(name, &head, true);
}

/// Merge `branch` into the current branch via HEAD (a 2-parent commit).
fn merge_into_head(path: &std::path::Path, branch: &str, secs: i64, msg: &str) -> bool {
    let repo = G2Repo::open(path).unwrap();
    let full = format!("refs/heads/{branch}");
    let Ok(other_ref) = repo.find_reference(&full) else {
        return false;
    };
    let other = other_ref.peel_to_commit().unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let sig = sig_at(secs);
    let tree = head.tree().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&head, &other])
        .unwrap();
    true
}

/// Names of local branches other than the one HEAD points at.
fn other_local_branches(path: &std::path::Path) -> Vec<String> {
    let repo = G2Repo::open(path).unwrap();
    let head_name = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(str::to_string));
    repo.branches(Some(git2::BranchType::Local))
        .unwrap()
        .flatten()
        .filter_map(|(b, _)| b.name().ok().flatten().map(str::to_string))
        .filter(|n| Some(n.as_str()) != head_name.as_deref())
        .collect()
}

/// Drive one topology through `steps` mutations, asserting incremental == full
/// on every detected simple advance. Returns how many times the fast path fired.
fn run_topology(path: &std::path::Path, steps: usize) -> usize {
    let mut prev = full_layout(path);
    let mut prev_refs = ref_snapshot(path);
    let mut engaged = 0usize;
    // Deterministic LCG so failures reproduce.
    let mut state: u64 = 0x9E3779B97F4A7C15 ^ (path.as_os_str().len() as u64);
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 33) as u32
    };
    let mut secs: i64 = 2_000_000_000;

    for i in 0..steps {
        secs += 1;
        // The first few steps are always plain HEAD commits so the fast path is
        // guaranteed to be exercised; the rest are a weighted random mix.
        let choice = if i < 5 { 3 } else { next() % 10 };
        match choice {
            0 => create_branch(path, &format!("topic-{i}")),
            1 => {
                let others = other_local_branches(path);
                if let Some(b) = others.first() {
                    if !merge_into_head(path, b, secs, &format!("merge {b} @ {i}")) {
                        commit_on_head(path, secs, &format!("c{i}"));
                    }
                } else {
                    commit_on_head(path, secs, &format!("c{i}"));
                }
            }
            2 => {
                let others = other_local_branches(path);
                if let Some(b) = others.first() {
                    commit_on_branch(path, b, secs, &format!("branch c{i}"));
                } else {
                    commit_on_head(path, secs, &format!("c{i}"));
                }
            }
            _ => commit_on_head(path, secs, &format!("c{i}")),
        }

        let full_after = full_layout(path);
        let new_refs = ref_snapshot(path);

        if let Some((name, old_oid, new_oid)) = single_ref_move(&prev_refs, &new_refs) {
            let row0 = &prev.nodes[0];
            if is_branch(&name) && old_oid == row0.oid && row0.lane == 0 {
                let repo = Repository::open(path).unwrap();
                if let Some((commits, old_tip_refs)) = repo
                    .simple_advance_commits(&old_oid, &new_oid, CAP)
                    .unwrap()
                {
                    let ts_ok = commits.iter().all(|c| c.timestamp >= row0.timestamp);
                    if ts_ok {
                        let gcs: Vec<GraphCommit> = commits.into_iter().map(to_gc).collect();
                        let inc = prev
                            .try_prepend_simple_advance(&gcs, old_tip_refs)
                            .expect("a detected simple advance must apply");
                        assert!(
                            structural_diff(&inc, &full_after).is_none(),
                            "step {i}: incremental != full rebuild: {:?}",
                            structural_diff(&inc, &full_after)
                        );
                        engaged += 1;
                    }
                }
            }
        }

        prev = full_after;
        prev_refs = new_refs;
    }

    engaged
}

#[test]
fn incremental_prepend_matches_full_rebuild_linear() {
    let (dir, path) = create_repo_with_n_commits(40);
    let engaged = run_topology(&path, 25);
    assert!(engaged >= 3, "fast path should engage on a linear repo");
    drop(dir);
}

#[test]
fn incremental_prepend_matches_full_rebuild_merge_heavy() {
    // Dense merges: many feature branches folded into the mainline.
    let (dir, path) = create_synthetic_repo(120, 20);
    let engaged = run_topology(&path, 25);
    assert!(
        engaged >= 3,
        "fast path should engage on a merge-heavy repo"
    );
    drop(dir);
}

#[test]
fn incremental_prepend_matches_full_rebuild_multi_branch() {
    // Fewer merges, more standing branches (wider multi-lane graph).
    let (dir, path) = create_synthetic_repo(200, 6);
    // Add extra standing branches so several lanes persist across the walk.
    {
        let repo = G2Repo::open(&path).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        for k in 0..4 {
            let _ = repo.branch(&format!("standing-{k}"), &head, true);
        }
    }
    let engaged = run_topology(&path, 30);
    assert!(
        engaged >= 3,
        "fast path should engage on a multi-branch repo"
    );
    drop(dir);
}
