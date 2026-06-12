//! Test fixtures for building small in-memory git repositories.
//!
//! Enabled via the `test-support` feature; hidden otherwise so the
//! helper surface doesn't leak into release builds.

use std::path::{Path, PathBuf};
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

/// Configure a repo-local identity (`user.name` / `user.email`) so subsequent
/// `git` invocations can produce commits without picking up global settings.
fn configure_identity(repo: &git2::Repository) {
    let mut config = repo.config().expect("config");
    config.set_str("user.name", "Test User").expect("user.name");
    config
        .set_str("user.email", "test@example.com")
        .expect("user.email");
}

/// Create an initial (empty-tree) commit on `HEAD` and return its oid.
fn initial_commit(repo: &git2::Repository) -> git2::Oid {
    let sig = git2::Signature::now("Test User", "test@example.com").expect("sig");
    let tree_id = {
        let mut index = repo.index().expect("index");
        index.write_tree().expect("write_tree")
    };
    let tree = repo.find_tree(tree_id).expect("find_tree");
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .expect("initial commit")
}

/// Build a repository with a single initial commit, then write + stage the
/// supplied `(path, content)` pairs.
///
/// Invariant: `HEAD` is the empty-tree initial commit, the index carries all
/// requested files in the `INDEX_NEW` status, and the working tree matches the
/// index. No second commit is recorded so callers can test "staged changes
/// pending" flows.
pub fn create_repo_with_staged_changes(files: &[(&str, &str)]) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_path_buf();
    let repo = git2::Repository::init(&path).expect("init");
    configure_identity(&repo);

    // Initial commit so HEAD exists.
    initial_commit(&repo);

    // Write files to the working tree and stage them.
    let mut index = repo.index().expect("index");
    for (rel_path, contents) in files {
        let full = path.join(rel_path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("mkdir parent");
        }
        std::fs::write(&full, contents).expect("write file");
        index.add_path(Path::new(rel_path)).expect("add_path");
    }
    index.write().expect("index.write");

    drop(index);
    drop(repo);

    (dir, path)
}

/// Build a repository that is mid-merge with a conflict on `path`.
///
/// Steps:
/// 1. Initial commit on `main` writes `base` to `path`.
/// 2. Branch `ours-branch` overwrites `path` with `ours` and commits.
/// 3. Branch `theirs-branch` (forked from the base) overwrites `path` with
///    `theirs` and commits.
/// 4. While on `ours-branch`, merge `theirs-branch`. The merge is expected to
///    fail with a conflict.
///
/// Invariant on return: `HEAD` is `ours-branch`, the index contains conflict
/// entries for `path` (stages 1/2/3), and the working-tree file has the
/// standard conflict markers.
pub fn create_repo_with_conflict(
    base: &str,
    ours: &str,
    theirs: &str,
    path: &str,
) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let repo_path = dir.path().to_path_buf();
    let repo = git2::Repository::init(&repo_path).expect("init");
    configure_identity(&repo);

    let sig = git2::Signature::now("Test User", "test@example.com").expect("sig");

    // Base commit.
    std::fs::write(repo_path.join(path), base).expect("write base");
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(path)).expect("add base");
    index.write().expect("write index");
    let tree_id = index.write_tree().expect("write_tree");
    let tree = repo.find_tree(tree_id).expect("find_tree");
    let base_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .expect("commit base");
    drop(index);
    drop(tree);

    let base_commit = repo.find_commit(base_oid).expect("find base commit");

    // Create branch `ours-branch` at base and commit `ours` on it.
    repo.branch("ours-branch", &base_commit, false)
        .expect("branch ours");
    repo.set_head("refs/heads/ours-branch")
        .expect("set HEAD ours");
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .expect("checkout ours");

    std::fs::write(repo_path.join(path), ours).expect("write ours");
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(path)).expect("add ours");
    index.write().expect("write ours index");
    let tree_id = index.write_tree().expect("write_tree ours");
    let tree = repo.find_tree(tree_id).expect("find_tree ours");
    let ours_parent = repo
        .head()
        .expect("head")
        .peel_to_commit()
        .expect("head commit");
    repo.commit(Some("HEAD"), &sig, &sig, "ours", &tree, &[&ours_parent])
        .expect("commit ours");
    drop(ours_parent);
    drop(index);
    drop(tree);

    // Create branch `theirs-branch` at base and commit `theirs` on it.
    repo.branch("theirs-branch", &base_commit, false)
        .expect("branch theirs");
    repo.set_head("refs/heads/theirs-branch")
        .expect("set HEAD theirs");
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .expect("checkout theirs");

    std::fs::write(repo_path.join(path), theirs).expect("write theirs");
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(path)).expect("add theirs");
    index.write().expect("write theirs index");
    let tree_id = index.write_tree().expect("write_tree theirs");
    let tree = repo.find_tree(tree_id).expect("find_tree theirs");
    let theirs_parent = repo
        .head()
        .expect("head")
        .peel_to_commit()
        .expect("head commit");
    repo.commit(Some("HEAD"), &sig, &sig, "theirs", &tree, &[&theirs_parent])
        .expect("commit theirs");
    drop(theirs_parent);
    drop(index);
    drop(tree);

    // Switch back to `ours-branch` and attempt the merge via the CLI so the
    // working tree ends up with standard conflict markers and the merge
    // metadata (`MERGE_HEAD`, `MERGE_MSG`) is present.
    drop(base_commit);
    drop(repo);
    run_git(&repo_path, &["checkout", "ours-branch"]);
    // `--no-commit` so the merge is paused at the conflict; it returns
    // non-zero — that's expected.
    let _ = std::process::Command::new("git")
        .args(["merge", "--no-commit", "--no-ff", "theirs-branch"])
        .current_dir(&repo_path)
        .output()
        .expect("spawn merge");

    (dir, repo_path)
}

/// Build a repository with a single initial commit and `n` stash entries on
/// top. Each stash modifies a distinct tracked file so the stash content is
/// non-empty and can be round-tripped through `git stash pop`.
///
/// Invariant on return: the working tree is clean (every stash was created
/// with `git stash push`), `git stash list` has exactly `n` entries, and the
/// initial commit remains the only entry on `HEAD`.
pub fn create_repo_with_stash(n: usize) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let repo_path = dir.path().to_path_buf();
    let repo = git2::Repository::init(&repo_path).expect("init");
    configure_identity(&repo);

    // Seed `n` tracked files, one per intended stash entry.
    let sig = git2::Signature::now("Test User", "test@example.com").expect("sig");
    let mut index = repo.index().expect("index");
    for i in 0..n.max(1) {
        let rel = format!("f{i}.txt");
        std::fs::write(repo_path.join(&rel), format!("base-{i}\n")).expect("write base file");
        index.add_path(Path::new(&rel)).expect("add base file");
    }
    index.write().expect("index.write");
    let tree_id = index.write_tree().expect("write_tree");
    let tree = repo.find_tree(tree_id).expect("find_tree");
    repo.commit(Some("HEAD"), &sig, &sig, "seed", &tree, &[])
        .expect("seed commit");
    drop(index);
    drop(tree);
    drop(repo);

    // Now create N stashes, one at a time, modifying a different file each
    // time so the working tree is never clean when we call `git stash push`.
    for i in 0..n {
        let rel = format!("f{i}.txt");
        std::fs::write(repo_path.join(&rel), format!("stash-{i}\n")).expect("write modified file");
        run_git(
            &repo_path,
            &["stash", "push", "-m", &format!("stash-{i}"), &rel],
        );
    }

    (dir, repo_path)
}

/// Build a repository with one base commit and `branches.len()` additional
/// local branches, each pointing at an independent commit that touches a
/// branch-specific file. The first branch is created on top of the base
/// commit, subsequent branches are also created on top of the base commit
/// (they are peers, not a chain).
///
/// Invariant: every name in `branches` resolves to a distinct commit whose
/// parent is the base commit. `HEAD` is left on the repository's default
/// branch (unchanged — we never check out the new branches).
pub fn create_repo_with_branches(branches: &[&str]) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let repo_path = dir.path().to_path_buf();
    let repo = git2::Repository::init(&repo_path).expect("init");
    configure_identity(&repo);

    let sig = git2::Signature::now("Test User", "test@example.com").expect("sig");

    // Base commit.
    std::fs::write(repo_path.join("README"), "base\n").expect("write base");
    let mut index = repo.index().expect("index");
    index.add_path(Path::new("README")).expect("add README");
    index.write().expect("write index");
    let tree_id = index.write_tree().expect("write_tree");
    let tree = repo.find_tree(tree_id).expect("find_tree");
    let base_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .expect("commit base");
    drop(index);
    drop(tree);

    let base_commit = repo.find_commit(base_oid).expect("find base commit");

    for (i, branch_name) in branches.iter().enumerate() {
        // Build a new tree that adds a branch-specific file on top of the
        // base tree. We do this by staging the new file into the index,
        // writing the tree, then reverting the index back so the next
        // iteration starts from the same baseline.
        let rel = format!("branch-{i}.txt");
        std::fs::write(repo_path.join(&rel), format!("branch {i}\n")).expect("write branch file");

        let mut index = repo.index().expect("index");
        index.add_path(Path::new(&rel)).expect("add branch file");
        let branch_tree_id = index.write_tree().expect("write branch tree");
        let branch_tree = repo.find_tree(branch_tree_id).expect("find branch tree");

        let commit_oid = repo
            .commit(
                None, // don't move HEAD — we only want the branch to move.
                &sig,
                &sig,
                &format!("branch {i} commit"),
                &branch_tree,
                &[&base_commit],
            )
            .expect("commit branch");
        let commit = repo.find_commit(commit_oid).expect("find branch commit");
        repo.branch(branch_name, &commit, false)
            .expect("create branch");

        // Remove the branch file from the index and from disk so the next
        // iteration doesn't inherit it.
        index
            .remove_path(Path::new(&rel))
            .expect("remove from index");
        index.write().expect("rewrite index");
        std::fs::remove_file(repo_path.join(&rel)).expect("rm branch file");
        drop(index);
        drop(branch_tree);
        drop(commit);
    }

    drop(base_commit);
    drop(repo);

    (dir, repo_path)
}

/// Build a repository whose `HEAD` history contains a merge commit whose
/// second parent (the merged "feature branch") is no longer referenced by
/// any ref:
///
/// ```text
/// m   "merge feature"  (HEAD)  parents = [c2, f1]
/// ├── c2  "Commit 2"
/// │    └── c1  "Commit 1" (root)
/// └── f1  "feature work"  parent = c2  (no branch ref — reachable only via m)
/// ```
///
/// Ideal for first-parent walk tests (a first-parent walk must skip `f1`)
/// and layout tests (the default layout needs 2 lanes, the first-parent
/// layout collapses to 1).
pub fn create_repo_with_merged_branch() -> (TempDir, PathBuf) {
    let (dir, path) = create_repo_with_n_commits(2);
    {
        let repo = git2::Repository::open(&path).expect("open");
        let sig = git2::Signature::now("Test User", "test@example.com").expect("sig");

        let head_oid = repo.head().expect("head").target().expect("head oid");
        let c2 = repo.find_commit(head_oid).expect("find c2");
        let tree = c2.tree().expect("tree");

        // Feature commit off c2, intentionally not referenced by any branch.
        let f1_oid = repo
            .commit(None, &sig, &sig, "feature work", &tree, &[&c2])
            .expect("commit f1");
        let f1 = repo.find_commit(f1_oid).expect("find f1");

        // Merge commit on HEAD: c2 is the first parent, f1 the second.
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "merge feature",
            &tree,
            &[&c2, &f1],
        )
        .expect("commit merge");
    }
    (dir, path)
}

/// Helper: shell out to `git` in `repo_path` and panic on failure.
fn run_git(repo_path: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .status()
        .expect("spawn git");
    if !status.success() {
        panic!("git {args:?} failed with status {status:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_repo_with_n_commits_has_n_commits() {
        let (_tmp, path) = create_repo_with_n_commits(3);
        let repo = git2::Repository::open(&path).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        // Walk parents, count.
        let mut count = 1;
        let mut cur = head;
        while let Ok(parent) = cur.parent(0) {
            count += 1;
            cur = parent;
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn staged_changes_fixture_has_staged_files() {
        let (_tmp, path) =
            create_repo_with_staged_changes(&[("a.txt", "hello\n"), ("sub/b.txt", "world\n")]);

        let repo = git2::Repository::open(&path).unwrap();
        let statuses = repo.statuses(None).unwrap();
        let paths: Vec<String> = statuses
            .iter()
            .filter(|e| e.status().contains(git2::Status::INDEX_NEW))
            .map(|e| e.path().unwrap_or("").to_string())
            .collect();
        assert!(paths.iter().any(|p| p == "a.txt"));
        assert!(paths.iter().any(|p| p == "sub/b.txt"));
    }

    #[test]
    fn conflict_fixture_has_conflict_on_path() {
        let (_tmp, path) =
            create_repo_with_conflict("base\n", "ours\n", "theirs\n", "conflicted.txt");
        let repo = git2::Repository::open(&path).unwrap();
        let index = repo.index().unwrap();
        assert!(index.has_conflicts(), "index should carry a conflict");

        // Working-tree file should include conflict markers.
        let content = std::fs::read_to_string(path.join("conflicted.txt")).unwrap();
        assert!(content.contains("<<<<<<<"));
        assert!(content.contains(">>>>>>>"));
    }

    #[test]
    fn stash_fixture_has_n_entries() {
        let (_tmp, path) = create_repo_with_stash(2);
        let output = std::process::Command::new("git")
            .args(["stash", "list"])
            .current_dir(&path)
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&output.stdout);
        assert_eq!(s.lines().count(), 2);
    }

    #[test]
    fn branches_fixture_creates_all_branches() {
        let (_tmp, path) = create_repo_with_branches(&["feat-a", "feat-b", "feat-c"]);
        let repo = git2::Repository::open(&path).unwrap();
        for name in ["feat-a", "feat-b", "feat-c"] {
            let branch = repo
                .find_branch(name, git2::BranchType::Local)
                .unwrap_or_else(|e| panic!("branch {name} missing: {e}"));
            // Each branch tip should be distinct from HEAD.
            let head_oid = repo.head().unwrap().target().unwrap();
            assert_ne!(branch.get().target().unwrap(), head_oid);
        }
    }
}
