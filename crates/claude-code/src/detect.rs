//! Binary detection, version parsing, and repo artifact scanning.

use std::path::{Path, PathBuf};

use ai_provider::AiError;

/// Find the `claude` binary on PATH.
pub fn detect_binary() -> Option<PathBuf> {
    which::which("claude").ok()
}

/// Run `claude --version` and extract the version string.
///
/// The version-token scanner is the one detection helper genuinely shared with
/// the other providers — see [`ai_provider_common::version`].
pub fn version(binary: &Path) -> Result<String, AiError> {
    ai_provider_common::version(binary)
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
