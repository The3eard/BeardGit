//! Git bisect operations via system git CLI.
//!
//! Provides functions to drive the `git bisect` workflow: start/stop sessions,
//! mark commits as good/bad/skip, query session state, and run automated
//! bisect with a test command.

use std::path::Path;
use std::process::Command;

use tracing::instrument;

/// Current state of a bisect session.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BisectState {
    /// Whether a bisect is in progress.
    pub active: bool,
    /// The current commit being tested (if any).
    pub current_commit: Option<String>,
    /// Number of steps remaining (approximate).
    pub steps_remaining: Option<usize>,
    /// Good commits marked so far.
    pub good_commits: Vec<String>,
    /// Bad commits marked so far.
    pub bad_commits: Vec<String>,
}

/// Start a bisect session, optionally providing the initial bad and good commits.
#[instrument(fields(repo = %repo_path.display()))]
pub fn bisect_start(
    repo_path: &Path,
    bad: Option<&str>,
    good: Option<&str>,
) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path).arg("bisect").arg("start");
    if let Some(b) = bad {
        cmd.arg(b);
    }
    if let Some(g) = good {
        cmd.arg(g);
    }
    run_git(cmd)
}

/// Mark a commit (or current HEAD) as good.
#[instrument(fields(repo = %repo_path.display()))]
pub fn bisect_good(repo_path: &Path, commit: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path).arg("bisect").arg("good");
    if let Some(c) = commit {
        cmd.arg(c);
    }
    run_git(cmd)
}

/// Mark a commit (or current HEAD) as bad.
#[instrument(fields(repo = %repo_path.display()))]
pub fn bisect_bad(repo_path: &Path, commit: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path).arg("bisect").arg("bad");
    if let Some(c) = commit {
        cmd.arg(c);
    }
    run_git(cmd)
}

/// Skip the current commit (untestable).
#[instrument(fields(repo = %repo_path.display()))]
pub fn bisect_skip(repo_path: &Path) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path).args(["bisect", "skip"]);
    run_git(cmd)
}

/// Reset (end) the bisect session and return to the original HEAD.
#[instrument(fields(repo = %repo_path.display()))]
pub fn bisect_reset(repo_path: &Path) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path).args(["bisect", "reset"]);
    run_git(cmd)
}

/// Query the current bisect state by checking `BISECT_START` in the
/// repository's git directory, plus the bisect log.
///
/// The git directory is resolved through `git2` rather than joined as
/// `<repo>/.git`. In a linked worktree (and in a submodule) `.git` is a
/// *file* pointing elsewhere, so the naive join names a path that never
/// exists — which reported `active: false` with a bisect actually running,
/// and the bisect UI simply never appeared. `Repository::path()` is the
/// per-worktree git dir, which is where `BISECT_START` lives; `commondir()`
/// is the shared one and would be wrong, since bisect state is per-worktree.
pub fn bisect_state(repo_path: &Path) -> Result<BisectState, String> {
    let git_dir = git2::Repository::open(repo_path)
        .map_err(|e| format!("could not open repository: {}", e.message()))?
        .path()
        .to_path_buf();
    let bisect_start_file = git_dir.join("BISECT_START");
    if !bisect_start_file.exists() {
        return Ok(BisectState {
            active: false,
            current_commit: None,
            steps_remaining: None,
            good_commits: vec![],
            bad_commits: vec![],
        });
    }

    // Get current HEAD (short SHA)
    let mut head_cmd = Command::new("git");
    head_cmd
        .current_dir(repo_path)
        .args(["rev-parse", "--short", "HEAD"]);
    let head = run_git(head_cmd)?;

    // Parse the bisect log for marked commits. Propagate a failure instead of
    // defaulting to empty: `BISECT_START` exists, so a session *is* running,
    // and empty good/bad lists render as "no commits marked yet" — which is a
    // different claim from "the log could not be read".
    let mut log_cmd = Command::new("git");
    log_cmd.current_dir(repo_path).args(["bisect", "log"]);
    let log_output = run_git(log_cmd)?;

    let mut good = vec![];
    let mut bad = vec![];
    for line in log_output.lines() {
        if let Some(rest) = line.strip_prefix("# good: [")
            && let Some(oid) = rest.split(']').next()
        {
            good.push(oid.to_string());
        } else if let Some(rest) = line.strip_prefix("# bad: [")
            && let Some(oid) = rest.split(']').next()
        {
            bad.push(oid.to_string());
        }
    }

    Ok(BisectState {
        active: true,
        current_commit: Some(head.trim().to_string()),
        steps_remaining: None,
        good_commits: good,
        bad_commits: bad,
    })
}

/// Return the raw bisect log output.
pub fn bisect_log(repo_path: &Path) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path).args(["bisect", "log"]);
    run_git(cmd)
}

/// Run an automated bisect with a test command.
///
/// The test command is split on whitespace and passed to `git bisect run`.
// `skip_all`: see `cmd::bisect::run_auto` — the test command is
// user-typed and can carry inline secrets.
#[instrument(skip_all, fields(repo = %repo_path.display()))]
pub fn bisect_run(repo_path: &Path, test_command: &str) -> Result<String, String> {
    let parts: Vec<&str> = test_command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty test command".into());
    }
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path).arg("bisect").arg("run");
    cmd.args(&parts);
    run_git(cmd)
}

/// Execute a git command and return its stdout on success, or an error string.
///
/// Bisect commands sometimes output useful information to stdout even when
/// the exit code is non-zero, so we return stdout when stderr is empty.
fn run_git(mut cmd: Command) -> Result<String, String> {
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Bisect often outputs useful info to stdout even on "failure"
        if !stdout.is_empty() && stderr.is_empty() {
            Ok(stdout)
        } else {
            Err(if stderr.is_empty() { stdout } else { stderr })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `git` in `dir`, asserting success.
    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// A repo with `n` commits on `main`, identity pinned so the fixture
    /// doesn't inherit the machine's `~/.gitconfig`.
    fn repo_with_commits(dir: &Path, n: usize) {
        git(dir, &["init", "-q", "-b", "main", "."]);
        git(dir, &["config", "user.email", "test@example.com"]);
        git(dir, &["config", "user.name", "Test"]);
        git(dir, &["config", "commit.gpgsign", "false"]);
        for i in 0..n {
            std::fs::write(dir.join("f.txt"), format!("{i}\n")).unwrap();
            git(dir, &["add", "-A"]);
            git(dir, &["commit", "-qm", &format!("c{i}")]);
        }
    }

    #[test]
    fn bisect_state_inactive_when_no_bisect_running() {
        let tmp = tempfile::tempdir().unwrap();
        repo_with_commits(tmp.path(), 1);
        let state = bisect_state(tmp.path()).unwrap();
        assert!(!state.active);
        assert!(state.current_commit.is_none());
        assert!(state.good_commits.is_empty());
        assert!(state.bad_commits.is_empty());
    }

    #[test]
    fn bisect_state_active_in_the_main_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        repo_with_commits(tmp.path(), 4);
        git(tmp.path(), &["bisect", "start", "HEAD", "HEAD~2"]);

        let state = bisect_state(tmp.path()).unwrap();
        assert!(state.active);
        assert!(state.current_commit.is_some());
    }

    /// The reason this module can't build the path itself. In a linked
    /// worktree `<wt>/.git` is a *file* pointing at
    /// `<main>/.git/worktrees/<name>/`, and that is where `BISECT_START`
    /// lives — bisect state is per-worktree, so `commondir()` is the wrong
    /// answer here even though it looks like the right one.
    #[test]
    fn bisect_state_active_inside_a_linked_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let main = tmp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        repo_with_commits(&main, 4);

        let wt = tmp.path().join("wt");
        git(
            &main,
            &["worktree", "add", "-q", wt.to_str().unwrap(), "-b", "side"],
        );
        git(&wt, &["bisect", "start", "HEAD", "HEAD~2"]);

        assert!(wt.join(".git").is_file(), "fixture: .git should be a file");
        let state = bisect_state(&wt).unwrap();
        assert!(
            state.active,
            "a bisect running in this worktree has to be visible from it"
        );

        // And the main worktree, which has no bisect of its own, still reads
        // as inactive — the state must not leak across worktrees.
        assert!(!bisect_state(&main).unwrap().active);
    }
}
