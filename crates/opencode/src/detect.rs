//! Binary detection, version parsing, and repo artifact scanning.

use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider::AiError;

/// Find the `opencode` binary on PATH.
pub fn detect_binary() -> Option<PathBuf> {
    which::which("opencode").ok()
}

/// Run `opencode --version` and extract the version string.
pub fn parse_version(binary: &PathBuf) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    extract_version_from_output(&stdout)
}

/// Extract a semver-like version from the `opencode --version` output.
///
/// Expected format: "opencode <version>" or just "<version>" on a line.
fn extract_version_from_output(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        for token in trimmed.split_whitespace() {
            if token.chars().next().is_some_and(|c| c.is_ascii_digit()) && token.contains('.') {
                return Some(token.to_string());
            }
        }
    }
    None
}

/// Check if OpenCode has artifacts in the given repo.
///
/// Looks for `.opencode/` directory in the repo root.
pub fn detect_in_repo(repo_path: &Path) -> bool {
    repo_path.join(".opencode").is_dir()
}

/// Run the binary and return the version, or an error if not found/parseable.
pub fn version(binary: &Path) -> Result<String, AiError> {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|e| AiError::CommandBuild(format!("failed to run --version: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    extract_version_from_output(&stdout)
        .ok_or_else(|| AiError::Parse(format!("no version found in output: {stdout}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_in_repo_with_opencode_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".opencode")).unwrap();
        assert!(detect_in_repo(dir.path()));
    }

    #[test]
    fn detect_in_repo_empty_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!detect_in_repo(dir.path()));
    }

    #[test]
    fn extract_version_standard_format() {
        let v = extract_version_from_output("opencode 0.1.42").unwrap();
        assert_eq!(v, "0.1.42");
    }

    #[test]
    fn extract_version_version_only() {
        let v = extract_version_from_output("0.1.42\n").unwrap();
        assert_eq!(v, "0.1.42");
    }

    #[test]
    fn extract_version_multiline() {
        let v = extract_version_from_output("OpenCode CLI\nVersion: 0.1.42\n").unwrap();
        assert_eq!(v, "0.1.42");
    }

    #[test]
    fn extract_version_no_version_returns_none() {
        let result = extract_version_from_output("no version here");
        assert!(result.is_none());
    }
}
