//! Codex worktree discovery and cleanup.
//!
//! Thin wrappers over the shared directory-scan helpers in
//! [`ai_provider_common`], parameterized by [`crate::SPEC`]. Codex worktrees
//! are BeardGit-spawned plain directories under
//! `<repo>/.beardgit/ai-worktrees/codex/<slug>/`, each optionally carrying a
//! `.beardgit/ai-session` marker. (The shared implementation and its tests
//! live in `ai-provider-common`.)

use std::path::Path;

use ai_provider::{AiError, AiWorktree};

use crate::SPEC;

/// List all Codex worktrees spawned by BeardGit for `repo_path`.
pub fn list_worktrees(repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
    ai_provider_common::list_worktrees(&SPEC, repo_path)
}

/// Remove the given worktree directory recursively (idempotent).
pub fn cleanup_worktree(worktree: &AiWorktree) -> Result<(), AiError> {
    ai_provider_common::cleanup_worktree(worktree)
}
