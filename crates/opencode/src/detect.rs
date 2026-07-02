//! Binary detection, version parsing, and repo artifact scanning for OpenCode.
//!
//! Thin wrappers over [`ai_provider_common`], parameterized by [`crate::SPEC`].

use std::path::{Path, PathBuf};

use ai_provider::AiError;

use crate::SPEC;

/// Find the `opencode` binary on PATH.
pub fn detect_binary() -> Option<PathBuf> {
    ai_provider_common::detect_binary(&SPEC)
}

/// Run `opencode --version` and return the parsed version, or an error if the
/// process failed to spawn / no version token was present.
pub fn version(binary: &Path) -> Result<String, AiError> {
    ai_provider_common::version(binary)
}

/// Check if OpenCode has artifacts (a `.opencode/` directory) in the given repo.
pub fn detect_in_repo(repo_path: &Path) -> bool {
    ai_provider_common::detect_in_repo(&SPEC, repo_path)
}
