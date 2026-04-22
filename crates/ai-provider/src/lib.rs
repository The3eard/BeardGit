//! AI provider trait and shared types for AI coding tool integration.
//!
//! This crate defines the [`AiProvider`] trait — the contract for all AI tool
//! backends (Claude Code, Codex, OpenCode). It is intentionally Tauri-free
//! and async-free so it can be tested and reused independently.
//!
//! # Design Principle: Command-Building
//!
//! The trait builds [`std::process::Command`] objects but never executes them.
//! Execution is handled by `app-core` via `TaskManager` (headless) or
//! `TerminalManager` (interactive). Detection methods (`detect_binary`,
//! `version`) do perform blocking filesystem I/O — they are called
//! infrequently (startup, tab switch) and are fast local operations.

pub mod error;
pub mod types;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

pub use error::AiError;
pub use types::*;

use std::path::{Path, PathBuf};
use std::process::Command;

/// Trait defining the interface for AI coding tool integrations.
///
/// The trait is **comprehensive**: it covers the union of capabilities across
/// all target providers (Claude Code, Codex, OpenCode). Each method has a
/// default implementation that returns empty/None/`NotSupported`. Concrete
/// providers only override what they actually support.
pub trait AiProvider: Send + Sync {
    // ─── 1. Identity (required) ───

    /// Which AI tool this provider represents.
    fn provider_kind(&self) -> AiProviderKind;

    /// CLI binary name (e.g., `"claude"`, `"codex"`, `"opencode"`).
    fn binary_name(&self) -> &str;

    // ─── 2. Detection ───

    /// Locate the binary on PATH or at a known bundled location.
    fn detect_binary(&self) -> Option<PathBuf>;

    /// Run `--version` and parse the version string from stdout.
    fn version(&self) -> Result<String, AiError>;

    /// Whether the binary is installed on this machine.
    fn is_installed(&self) -> bool {
        self.detect_binary().is_some()
    }

    /// Whether this tool has artifacts in the given repo (`.claude/`, `CLAUDE.md`, etc.).
    fn detect_in_repo(&self, repo_path: &Path) -> bool;

    // ─── 3. Headless Execution (core primitive) ───

    /// Build a [`Command`] for non-interactive, single-shot execution.
    ///
    /// The caller (`app-core`) is responsible for spawning the process and
    /// streaming its output through `TaskManager`.
    fn build_execute_command(
        &self,
        prompt: &str,
        cwd: &Path,
        options: &ExecuteOptions,
    ) -> Result<Command, AiError>;

    // ─── 4. Specialized Actions ───

    /// Build a command to generate a commit message from a diff.
    fn build_commit_message_cmd(&self, diff: &str, cwd: &Path) -> Result<Command, AiError> {
        self.build_execute_command(
            &format!(
                "Generate a concise git commit message for this diff. \
                 Use the conventional commits format: type(scope): description. \
                 Output ONLY the commit message, no explanations.\n\n{diff}"
            ),
            cwd,
            &ExecuteOptions::default(),
        )
    }

    /// Build a command to review code changes.
    fn build_review_cmd(&self, diff: &str, cwd: &Path) -> Result<Command, AiError> {
        self.build_execute_command(
            &format!(
                "Review this code diff. Report bugs, security issues, \
                 performance problems, and style concerns. Be concise.\n\n{diff}"
            ),
            cwd,
            &ExecuteOptions::default(),
        )
    }

    /// Build a command to analyze code and answer a question about it.
    fn build_analysis_cmd(
        &self,
        content: &str,
        question: &str,
        cwd: &Path,
    ) -> Result<Command, AiError> {
        self.build_execute_command(
            &format!("{question}\n\n{content}"),
            cwd,
            &ExecuteOptions::default(),
        )
    }

    /// Build a command to generate a PR/MR description.
    fn build_pr_description_cmd(&self, diff: &str, cwd: &Path) -> Result<Command, AiError> {
        self.build_execute_command(
            &format!(
                "Generate a pull request description for this diff. \
                 Include a summary section and a list of key changes. \
                 Use markdown formatting.\n\n{diff}"
            ),
            cwd,
            &ExecuteOptions::default(),
        )
    }

    /// Build a command to review a PR/MR.
    fn build_pr_review_cmd(&self, diff: &str, cwd: &Path) -> Result<Command, AiError> {
        self.build_execute_command(
            &format!(
                "Review this pull request diff. Report bugs, security issues, \
                 design concerns, and suggest improvements. Be thorough.\n\n{diff}"
            ),
            cwd,
            &ExecuteOptions::default(),
        )
    }

    /// Build a [`Command`] for a **headless background run** inside a
    /// dedicated worktree.
    ///
    /// Default implementation returns [`AiError::NotSupported`]. Providers
    /// override this with their own headless flag set (e.g. Claude Code's
    /// `--print --output-format json-stream`). The caller is responsible for
    /// spawning the process, streaming stdout/stderr, and writing the prompt
    /// to stdin when the provider signals it reads prompts from stdin.
    fn launch_background(&self, input: AiBackgroundRunInput) -> Result<Command, AiError> {
        let _ = input;
        Err(AiError::NotSupported)
    }

    /// Whether [`launch_background`](Self::launch_background) should pipe the
    /// prompt on stdin (`true`) instead of passing it as a CLI flag (`false`).
    ///
    /// Default: `false` — most providers accept `--prompt` / `-p`. Claude Code
    /// overrides to `true` because `--print <prompt>` truncates long strings.
    fn background_uses_stdin_prompt(&self) -> bool {
        false
    }

    // ─── 5. Interactive Launch ───

    /// Build a [`Command`] to launch an interactive session in a terminal tab.
    fn build_interactive_cmd(&self, cwd: &Path) -> Result<Command, AiError>;

    /// Build a [`Command`] to launch with a worktree.
    /// Returns `None` if the provider does not support worktrees.
    fn build_worktree_cmd(&self, cwd: &Path, name: Option<&str>) -> Option<Command> {
        let _ = (cwd, name);
        None
    }

    /// Build a [`Command`] to resume an existing session by ID.
    ///
    /// Returns `None` if the provider does not support session resumption.
    /// Providers that support it (e.g., Claude Code with `--resume`) return
    /// a command that reconnects to the given session.
    fn build_resume_session_cmd(&self, session_id: &str, cwd: &Path) -> Option<Command> {
        let _ = (session_id, cwd);
        None
    }

    // ─── 6. Session & Worktree Introspection ───

    /// List sessions for this provider in the given repo.
    fn list_sessions(&self, repo_path: &Path) -> Result<Vec<AiSession>, AiError> {
        let _ = repo_path;
        Ok(vec![])
    }

    /// List on-disk conversation transcripts whose cwd matches `repo_path`.
    ///
    /// This is the transcript-first replacement for [`list_sessions`] — each
    /// provider overrides it to read from its own transcript store (e.g. Claude
    /// Code's `~/.claude/projects/{cwd-slug}/*.jsonl`). `list_sessions` is kept
    /// alive during the feature-flagged rollout and will be removed once the
    /// V2 UI becomes the default.
    fn list_conversations(&self, repo_path: &Path) -> Result<Vec<AiConversation>, AiError> {
        let _ = repo_path;
        Ok(vec![])
    }

    /// Check if a session process is still running.
    fn is_session_active(&self, session: &AiSession) -> bool {
        let _ = session;
        false
    }

    /// List AI-created worktrees for this repo.
    fn list_worktrees(&self, repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
        let _ = repo_path;
        Ok(vec![])
    }

    /// Remove a worktree and its branch.
    fn cleanup_worktree(&self, worktree: &AiWorktree) -> Result<(), AiError> {
        let _ = worktree;
        Err(AiError::NotSupported)
    }

    // ─── 7. Configuration & Attribution ───

    /// Discover configuration files for this provider in the repo.
    fn config_files(&self, repo_path: &Path) -> Vec<AiConfigFile>;

    /// Discover instruction files (CLAUDE.md, AGENTS.md, etc.).
    fn instruction_files(&self, repo_path: &Path) -> Vec<PathBuf>;

    /// Patterns used to detect AI-authored commits.
    fn attribution_patterns(&self) -> Vec<AttributionPattern> {
        vec![]
    }

    /// Check if a commit was authored by this AI tool.
    fn is_ai_authored(&self, message: &str, author: &str) -> bool {
        let _ = (message, author);
        false
    }
}
