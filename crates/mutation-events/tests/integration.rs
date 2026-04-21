//! Integration tests that exercise `Snapshot` + `MutationFlags` against
//! a real git2 repository, covering every flag the TS refresh matrix
//! cares about.

use mutation_events::{MutationFlags, Snapshot};

fn init_repo(tmp: &tempfile::TempDir) -> git2::Repository {
    let repo = git2::Repository::init(tmp.path()).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test").unwrap();
    cfg.set_str("user.email", "test@example.org").unwrap();
    repo
}

fn commit(repo: &git2::Repository, msg: &str) -> git2::Oid {
    let mut idx = repo.index().unwrap();
    idx.add_all(["*"], git2::IndexAddOption::DEFAULT, None).unwrap();
    idx.write().unwrap();
    let tree_id = idx.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let parent = repo.head().ok().and_then(|r| r.target()).and_then(|oid| repo.find_commit(oid).ok());
    let parents: Vec<&git2::Commit> = parent.as_ref().map(|c| vec![c]).unwrap_or_default();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents).unwrap()
}

#[test]
fn commit_flips_head_and_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = init_repo(&tmp);
    std::fs::write(tmp.path().join("seed.txt"), "seed\n").unwrap();
    commit(&repo, "seed");
    let before = Snapshot::capture(tmp.path()).unwrap();

    std::fs::write(tmp.path().join("new.txt"), "new\n").unwrap();
    commit(&repo, "new");

    let after = Snapshot::capture(tmp.path()).unwrap();
    let flags: MutationFlags = before.diff(&after);
    assert!(flags.head_changed);
    assert!(flags.refs_changed);
}

#[test]
fn branch_create_flips_refs_only() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = init_repo(&tmp);
    std::fs::write(tmp.path().join("x.txt"), "x\n").unwrap();
    commit(&repo, "seed");
    let before = Snapshot::capture(tmp.path()).unwrap();

    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("feature", &head_commit, false).unwrap();

    let after = Snapshot::capture(tmp.path()).unwrap();
    let flags = before.diff(&after);
    assert!(flags.refs_changed);
    assert!(!flags.head_changed);
}

#[test]
fn remote_add_flips_remotes_changed() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = init_repo(&tmp);
    std::fs::write(tmp.path().join("r.txt"), "r\n").unwrap();
    commit(&repo, "seed");
    let before = Snapshot::capture(tmp.path()).unwrap();

    repo.remote("origin", "https://example.org/x.git").unwrap();

    let after = Snapshot::capture(tmp.path()).unwrap();
    let flags = before.diff(&after);
    assert!(flags.remotes_changed);
    assert!(!flags.refs_changed);
}
