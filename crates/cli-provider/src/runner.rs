//! Shared CLI subprocess execution helpers.
//!
//! Both `GitHubCli` and `GitLabCli` share the same process-spawning logic —
//! they only differ in which binary path they invoke. These free functions
//! take the binary path as a parameter so both provider structs can reuse
//! them without a common base type.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

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
