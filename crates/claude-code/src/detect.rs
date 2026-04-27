//! Binary detection, version parsing, and repo artifact scanning.

use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider::AiError;

/// Find the `claude` binary on PATH.
pub fn detect_binary() -> Option<PathBuf> {
    which::which("claude").ok()
}

/// Run `claude --version` and extract the version string.
pub fn version(binary: &Path) -> Result<String, AiError> {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|e| AiError::CommandBuild(format!("failed to run --version: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version(&stdout)
}

/// Extract a semver-like version from the `claude --version` output.
///
/// Expected format: "claude <version>" or just "<version>" on a line.
fn parse_version(output: &str) -> Result<String, AiError> {
    for line in output.lines() {
        let trimmed = line.trim();
        for token in trimmed.split_whitespace() {
            if token.chars().next().is_some_and(|c| c.is_ascii_digit()) && token.contains('.') {
                return Ok(token.to_string());
            }
        }
    }
    Err(AiError::Parse(format!(
        "no version found in output: {output}"
    )))
}

/// Check if Claude Code has artifacts in the given repo.
///
/// Looks for `.claude/` directory or `CLAUDE.md` in the repo root.
pub fn detect_in_repo(repo_path: &Path) -> bool {
    repo_path.join(".claude").is_dir() || repo_path.join("CLAUDE.md").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_standard_format() {
        let version = parse_version("claude 2.1.104").unwrap();
        assert_eq!(version, "2.1.104");
    }

    #[test]
    fn parse_version_version_only() {
        let version = parse_version("2.1.104\n").unwrap();
        assert_eq!(version, "2.1.104");
    }

    #[test]
    fn parse_version_multiline() {
        let version = parse_version("Claude Code CLI\nVersion: 2.1.104\n").unwrap();
        assert_eq!(version, "2.1.104");
    }

    #[test]
    fn parse_version_no_version() {
        let err = parse_version("no version here").unwrap_err();
        assert!(matches!(err, AiError::Parse(_)));
    }

    #[test]
    fn detect_in_repo_with_claude_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        assert!(detect_in_repo(dir.path()));
    }

    #[test]
    fn detect_in_repo_with_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# Instructions").unwrap();
        assert!(detect_in_repo(dir.path()));
    }

    #[test]
    fn detect_in_repo_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!detect_in_repo(dir.path()));
    }
}
