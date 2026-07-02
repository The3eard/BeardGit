//! Integration tests that exercise `Snapshot` + `MutationFlags` against
//! a real git2 repository, covering every flag the TS refresh matrix
//! cares about.

use std::path::Path;

use git_engine::test_support::{create_repo_with_branches, create_repo_with_n_commits};
use mutation_events::{MutationFlags, Snapshot};

/// Run `git` with `args` inside `repo_path` and panic on failure.
/// Used by stash/worktree tests where shelling out is simpler than the
/// equivalent git2 API dance.
fn run_git(repo_path: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .status()
        .expect("spawn git");
    assert!(status.success(), "git {args:?} failed: {status:?}");
}

#[test]
fn commit_flips_head_and_refs() {
    let (_tmp, path) = create_repo_with_n_commits(1);
    let before = Snapshot::capture(&path).unwrap();

    // Create a second commit on the same repo via git-engine's high-level
    // helpers so we don't re-implement signing/index/tree wiring here.
    let repo = git_engine::Repository::open(&path).unwrap();
    std::fs::write(path.join("new.txt"), "new\n").unwrap();
    repo.stage_files(&["new.txt".to_string()]).unwrap();
    repo.create_commit("new").unwrap();

    let after = Snapshot::capture(&path).unwrap();
    let flags: MutationFlags = before.diff(&after);
    assert!(flags.head_changed);
    assert!(flags.refs_changed);
}

#[test]
fn branch_create_flips_refs_only() {
    let (_tmp, path) = create_repo_with_n_commits(1);
    let before = Snapshot::capture(&path).unwrap();

    let repo = git2::Repository::open(&path).unwrap();
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("feature", &head_commit, false).unwrap();

    let after = Snapshot::capture(&path).unwrap();
    let flags = before.diff(&after);
    assert!(flags.refs_changed);
    assert!(!flags.head_changed);
}

#[test]
fn batch_branch_delete_is_one_refs_diff() {
    // Deleting N branches must collapse into a single snapshot diff — the
    // basis for the `delete_branches` command wrapping the whole batch in one
    // `MutationGuard` and emitting one `project-mutated` event, not N.
    let (_tmp, path) = create_repo_with_branches(&["b1", "b2", "b3"]);
    let repo = git_engine::Repository::open(&path).unwrap();

    let before = Snapshot::capture(&path).unwrap();

    let names = vec!["b1".to_string(), "b2".to_string(), "b3".to_string()];
    let result = repo.delete_branches(&names, &names); // force: fixture branches are unmerged
    assert_eq!(result.deleted.len(), 3);
    assert!(result.failed.is_empty());

    let after = Snapshot::capture(&path).unwrap();
    let flags: MutationFlags = before.diff(&after);
    assert!(
        flags.refs_changed,
        "a batch of branch deletions flips refs_changed exactly once"
    );
    assert!(
        !flags.head_changed,
        "deleting non-HEAD branches leaves HEAD"
    );
}

#[test]
fn remote_add_flips_remotes_changed() {
    let (_tmp, path) = create_repo_with_n_commits(1);
    let before = Snapshot::capture(&path).unwrap();

    let repo = git2::Repository::open(&path).unwrap();
    repo.remote("origin", "https://example.org/x.git").unwrap();

    let after = Snapshot::capture(&path).unwrap();
    let flags = before.diff(&after);
    assert!(flags.remotes_changed);
    assert!(!flags.refs_changed);
}

#[test]
fn stash_flips_stashes_changed() {
    let (_tmp, path) = create_repo_with_n_commits(1);

    // Seed a tracked file + commit so stash has something to compare against.
    std::fs::write(path.join("tracked.txt"), "base\n").unwrap();
    run_git(&path, &["add", "tracked.txt"]);
    run_git(&path, &["commit", "-m", "track"]);

    // Dirty the working tree so `git stash push` has content to save.
    std::fs::write(path.join("tracked.txt"), "dirty\n").unwrap();

    let before = Snapshot::capture(&path).unwrap();
    run_git(&path, &["stash", "push", "-m", "test-stash"]);
    let after = Snapshot::capture(&path).unwrap();

    let flags = before.diff(&after);
    assert!(
        flags.stashes_changed,
        "stashes_changed should flip on stash push"
    );
}

#[test]
fn worktree_add_flips_worktrees_changed() {
    let (_tmp, path) = create_repo_with_n_commits(1);

    // `git worktree add` needs a sibling directory that doesn't already
    // exist — use the tempdir's parent.
    let worktree_path = path.parent().unwrap().join("wt-linked");

    let before = Snapshot::capture(&path).unwrap();
    run_git(
        &path,
        &[
            "worktree",
            "add",
            "-b",
            "wt-branch",
            worktree_path.to_str().unwrap(),
        ],
    );
    let after = Snapshot::capture(&path).unwrap();

    let flags = before.diff(&after);
    assert!(
        flags.worktrees_changed,
        "worktrees_changed should flip after `git worktree add`"
    );

    // Clean up the linked worktree so the tempdir drop doesn't race with a
    // dangling admin file.
    let _ = std::process::Command::new("git")
        .args([
            "worktree",
            "remove",
            "--force",
            worktree_path.to_str().unwrap(),
        ])
        .current_dir(&path)
        .status();
}
