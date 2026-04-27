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

/// Query the current bisect state by checking `.git/BISECT_START` and the bisect log.
pub fn bisect_state(repo_path: &Path) -> Result<BisectState, String> {
    let bisect_start_file = repo_path.join(".git").join("BISECT_START");
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

    // Parse the bisect log for marked commits
    let mut log_cmd = Command::new("git");
    log_cmd.current_dir(repo_path).args(["bisect", "log"]);
    let log_output = run_git(log_cmd).unwrap_or_default();

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
#[instrument(fields(repo = %repo_path.display()))]
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

    #[test]
    fn bisect_state_inactive_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        let state = bisect_state(tmp.path()).unwrap();
        assert!(!state.active);
        assert!(state.current_commit.is_none());
        assert!(state.good_commits.is_empty());
        assert!(state.bad_commits.is_empty());
    }
}
