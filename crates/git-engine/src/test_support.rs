//! Test fixtures for building small in-memory git repositories.
//!
//! Enabled via the `test-support` feature; hidden otherwise so the
//! helper surface doesn't leak into release builds.

use std::path::PathBuf;
use tempfile::TempDir;

/// Build a throwaway repository with `n` linear commits and return its
/// `TempDir` (so the caller can keep it alive) plus the repo path.
///
/// The repo has a single branch (`main` via `HEAD`), linear history, and
/// trivial author/committer identities. Intended for exercising pagination,
/// layout caching, and other engine-level tests.
pub fn create_repo_with_n_commits(n: usize) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_path_buf();
    let repo = git2::Repository::init(&path).expect("init");

    {
        let mut config = repo.config().expect("config");
        config.set_str("user.name", "Test User").expect("user.name");
        config
            .set_str("user.email", "test@example.com")
            .expect("user.email");
    }

    let sig = git2::Signature::now("Test User", "test@example.com").expect("sig");

    let mut parent_commit: Option<git2::Oid> = None;

    for i in 0..n {
        let tree_id = {
            let mut index = repo.index().expect("index");
            index.write_tree().expect("write_tree")
        };
        let tree = repo.find_tree(tree_id).expect("find_tree");

        let parents_vec: Vec<git2::Commit> = parent_commit
            .iter()
            .filter_map(|&oid| repo.find_commit(oid).ok())
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents_vec.iter().collect();

        let msg = format!("Commit {}", i + 1);
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, &msg, &tree, &parent_refs)
            .expect("commit");
        parent_commit = Some(oid);
    }

    // Drop the `git2::Repository` handle before returning so callers (and
    // subsequent `git2::Repository::open` calls) don't race on the lockfile.
    drop(repo);

    (dir, path)
}
