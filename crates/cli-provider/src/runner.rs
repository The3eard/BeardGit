//! Shared CLI subprocess execution helpers.
//!
//! Both `GitHubCli` and `GitLabCli` share the same process-spawning logic —
//! they only differ in which binary path they invoke. These free functions
//! take the binary path as a parameter so both provider structs can reuse
//! them without a common base type.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::configure_no_window;
use crate::error::CliError;

/// Check whether a path refers to a bare binary name (no directory component).
///
/// Used to distinguish `"gh"` (resolved via PATH) from `/usr/local/bin/gh`
/// (checked on disk first).
pub(crate) fn is_path_binary(path: &Path) -> bool {
    path.parent().is_none_or(|p| p.as_os_str().is_empty())
}

/// Run a CLI command and return stdout as a string.
pub(crate) fn run(binary_path: &Path, repo_path: &Path, args: &[&str]) -> Result<String, CliError> {
    if !binary_path.exists() && !is_path_binary(binary_path) {
        return Err(CliError::BinaryNotFound(binary_path.display().to_string()));
    }

    let mut cmd = Command::new(binary_path);
    cmd.args(args).current_dir(repo_path);
    configure_no_window(&mut cmd);

    let output = cmd.output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(CliError::CommandFailed(stderr))
    }
}

/// Run a CLI command and parse stdout as JSON into `T`.
pub(crate) fn run_json<T: serde::de::DeserializeOwned>(
    binary_path: &Path,
    repo_path: &Path,
    args: &[&str],
) -> Result<T, CliError> {
    let stdout = run(binary_path, repo_path, args)?;
    serde_json::from_str(&stdout).map_err(|e| CliError::JsonError(e.to_string()))
}

/// Run a CLI command with a hard wall-clock timeout.
///
/// Spawns the binary, polls for completion via `wait-timeout`, and kills
/// the child + returns [`CliError::CommandFailed`] with a "timed out"
/// message if it hasn't exited within `timeout`.
///
/// Used by the PR/MR diff fetch path (`gh api … --paginate`), which has
/// been observed to hang for many minutes on very large PRs. The
/// frontend already applies a 15 s cap on the Tauri invoke; this helper
/// gives the Rust side a slightly larger budget (20 s is the planned
/// value at the call site) so the subprocess dies cleanly instead of
/// lingering as an orphan after the frontend moves on.
pub(crate) fn run_with_timeout(
    binary_path: &Path,
    repo_path: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<String, CliError> {
    if !binary_path.exists() && !is_path_binary(binary_path) {
        return Err(CliError::BinaryNotFound(binary_path.display().to_string()));
    }

    let mut cmd = Command::new(binary_path);
    cmd.args(args)
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_no_window(&mut cmd);

    let mut child = cmd.spawn()?;

    match child.wait_timeout(timeout)? {
        Some(status) => {
            // Child exited on its own — drain its stdout/stderr pipes.
            let mut stdout = String::new();
            let mut stderr = String::new();
            if let Some(mut o) = child.stdout.take() {
                o.read_to_string(&mut stdout)?;
            }
            if let Some(mut e) = child.stderr.take() {
                e.read_to_string(&mut stderr)?;
            }
            if status.success() {
                Ok(stdout)
            } else {
                Err(CliError::CommandFailed(stderr))
            }
        }
        None => {
            // Timed out — kill + reap so we don't leave a zombie.
            let _ = child.kill();
            let _ = child.wait();
            Err(CliError::CommandFailed(format!(
                "command timed out after {}s",
                timeout.as_secs()
            )))
        }
    }
}

/// Run a CLI command with piped stdin and return stdout as a string.
pub(crate) fn run_with_stdin(
    binary_path: &Path,
    repo_path: &Path,
    args: &[&str],
    stdin_data: &str,
) -> Result<String, CliError> {
    if !binary_path.exists() && !is_path_binary(binary_path) {
        return Err(CliError::BinaryNotFound(binary_path.display().to_string()));
    }

    let mut cmd = Command::new(binary_path);
    cmd.args(args)
        .current_dir(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_no_window(&mut cmd);

    let mut child = cmd.spawn()?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(stdin_data.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(CliError::CommandFailed(stderr))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Instant;

    #[test]
    fn run_with_timeout_kills_long_running_child() {
        // `sleep 30` would otherwise block the test for half a minute;
        // we only give it 1 second and assert the helper surfaces the
        // timeout error + returns within ~5 s.
        let sleep_bin = PathBuf::from("/bin/sleep");
        let cwd = std::env::temp_dir();
        let started = Instant::now();
        let err = run_with_timeout(&sleep_bin, &cwd, &["30"], Duration::from_secs(1))
            .expect_err("sleep 30 must hit the 1s timeout");
        let elapsed = started.elapsed();

        match err {
            CliError::CommandFailed(msg) => assert!(
                msg.contains("timed out"),
                "error message should mention 'timed out', got: {msg}"
            ),
            other => panic!("expected CommandFailed, got {other:?}"),
        }
        assert!(
            elapsed < Duration::from_secs(5),
            "run_with_timeout should return promptly after killing the child (took {elapsed:?})"
        );
    }

    #[test]
    fn run_with_timeout_returns_stdout_for_fast_command() {
        // `/bin/echo hello` completes well inside the timeout — the
        // helper should capture stdout + return `Ok`.
        let echo_bin = PathBuf::from("/bin/echo");
        let cwd = std::env::temp_dir();
        let out = run_with_timeout(&echo_bin, &cwd, &["hello"], Duration::from_secs(5))
            .expect("echo must succeed");
        assert_eq!(out.trim(), "hello");
    }
}
