//! Binary detection, version parsing, and repo artifact scanning for Codex.
//!
//! Thin wrappers over [`ai_provider_common`], parameterized by [`crate::SPEC`].

use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider_common::parse_version_token;

use crate::SPEC;

/// Find the `codex` binary on PATH.
pub fn detect_binary() -> Option<PathBuf> {
    ai_provider_common::detect_binary(&SPEC)
}

/// Run `codex --version` and extract the version string, if any.
pub fn parse_version(binary: &Path) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    parse_version_token(&String::from_utf8_lossy(&output.stdout))
}

/// Check if Codex has artifacts (a `.codex/` directory) in the given repo.
pub fn detect_in_repo(repo_path: &Path) -> bool {
    ai_provider_common::detect_in_repo(&SPEC, repo_path)
}
