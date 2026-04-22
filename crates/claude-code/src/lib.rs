//! Claude Code AI provider implementation.
//!
//! Implements [`ai_provider::AiProvider`] for the Claude Code CLI tool.
//! Handles binary detection, headless command building, session/worktree
//! introspection, config discovery, and commit attribution.

pub mod attribution;
pub mod commands;
pub mod config;
pub mod conversations;
pub mod detect;
pub mod sessions;
pub mod worktrees;

use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider::{
    AiBackgroundRunInput, AiConfigFile, AiConversation, AiError, AiProvider, AiProviderKind,
    AiSession, AiWorktree, AttributionPattern, ExecuteOptions,
};

/// AI provider for the Claude Code CLI.
pub struct ClaudeCodeProvider;

impl ClaudeCodeProvider {
    /// Create a new `ClaudeCodeProvider` instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AiProvider for ClaudeCodeProvider {
    fn provider_kind(&self) -> AiProviderKind {
        AiProviderKind::ClaudeCode
    }

    fn binary_name(&self) -> &str {
        "claude"
    }

    fn detect_binary(&self) -> Option<PathBuf> {
        detect::detect_binary()
    }

    fn version(&self) -> Result<String, AiError> {
        let binary = self
            .detect_binary()
            .ok_or_else(|| AiError::BinaryNotFound("claude".into()))?;
        detect::version(&binary)
    }

    fn detect_in_repo(&self, repo_path: &Path) -> bool {
        detect::detect_in_repo(repo_path)
    }

    fn build_execute_command(
        &self,
        prompt: &str,
        cwd: &Path,
        options: &ExecuteOptions,
    ) -> Result<Command, AiError> {
        commands::build_execute_command(self, prompt, cwd, options)
    }

    fn build_interactive_cmd(&self, cwd: &Path) -> Result<Command, AiError> {
        commands::build_interactive_cmd(self, cwd)
    }

    fn launch_background(&self, input: AiBackgroundRunInput) -> Result<Command, AiError> {
        commands::build_background_command(self, &input)
    }

    fn background_uses_stdin_prompt(&self) -> bool {
        true
    }

    fn build_worktree_cmd(&self, cwd: &Path, name: Option<&str>) -> Option<Command> {
        commands::build_worktree_cmd(self, cwd, name)
    }

    fn build_resume_session_cmd(&self, session_id: &str, cwd: &Path) -> Option<Command> {
        let binary = self.detect_binary()?;
        let mut cmd = Command::new(binary);
        cmd.current_dir(cwd);
        cmd.arg("--resume").arg(session_id);
        Some(cmd)
    }

    fn list_sessions(&self, repo_path: &Path) -> Result<Vec<AiSession>, AiError> {
        sessions::list_sessions(repo_path)
    }

    fn list_conversations(&self, repo_path: &Path) -> Result<Vec<AiConversation>, AiError> {
        conversations::list_conversations(repo_path)
    }

    fn is_session_active(&self, session: &AiSession) -> bool {
        sessions::is_session_active(session)
    }

    fn list_worktrees(&self, repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
        worktrees::list_worktrees(repo_path)
    }

    fn cleanup_worktree(&self, worktree: &AiWorktree) -> Result<(), AiError> {
        worktrees::cleanup_worktree(worktree)
    }

    fn config_files(&self, repo_path: &Path) -> Vec<AiConfigFile> {
        config::config_files(repo_path)
    }

    fn instruction_files(&self, repo_path: &Path) -> Vec<PathBuf> {
        config::instruction_files(repo_path)
    }

    fn attribution_patterns(&self) -> Vec<AttributionPattern> {
        attribution::patterns()
    }

    fn is_ai_authored(&self, message: &str, author: &str) -> bool {
        attribution::is_ai_authored(message, author)
    }
}
