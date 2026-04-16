//! Binary detection, version parsing, and repo artifact scanning for Codex.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Find the `codex` binary on PATH.
pub fn detect_binary() -> Option<PathBuf> {
    which::which("codex").ok()
}

/// Run `codex --version` and extract the version string.
pub fn parse_version(binary: &PathBuf) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    extract_version(&stdout)
}

/// Extract a semver-like version from the `codex --version` output.
///
/// Expected format: "codex <version>" or just "<version>" on a line.
fn extract_version(output: &str) -> Option<String> {
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

/// Check if Codex has artifacts in the given repo.
///
/// Looks for a `.codex/` directory in the repo root.
pub fn detect_in_repo(repo_path: &Path) -> bool {
    repo_path.join(".codex").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_in_repo_returns_false_for_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!detect_in_repo(dir.path()));
    }

    #[test]
    fn detect_in_repo_returns_true_when_codex_dir_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".codex")).unwrap();
        assert!(detect_in_repo(dir.path()));
    }

    #[test]
    fn extract_version_standard_format() {
        let version = extract_version("codex 0.1.2").unwrap();
        assert_eq!(version, "0.1.2");
    }

    #[test]
    fn extract_version_version_only() {
        let version = extract_version("0.1.2\n").unwrap();
        assert_eq!(version, "0.1.2");
    }

    #[test]
    fn extract_version_no_version() {
        assert!(extract_version("no version here").is_none());
    }
}
