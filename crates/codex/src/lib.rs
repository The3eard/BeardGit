//! Codex AI provider implementation.
//!
//! Implements [`ai_provider::AiProvider`] for the Codex CLI tool.
//! Handles binary detection, headless command building (`codex exec -p`),
//! interactive launch, and config file discovery in `.codex/`.

pub mod attribution;
pub mod commands;
pub mod conversations;
pub mod detect;
pub mod errors;
pub mod sessions;
pub mod worktrees;

use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider::{
    AiBackgroundRunInput, AiConfigFile, AiConversation, AiError, AiProvider, AiProviderKind,
    AiSession, AiWorktree, AttributionMatch, AttributionPattern, ConfigKind, ConfigScope,
    ExecuteOptions,
};

/// AI provider for the Codex CLI.
pub struct CodexProvider {
    binary: Option<PathBuf>,
}

impl CodexProvider {
    /// Create a new `CodexProvider`, auto-detecting the binary on PATH.
    pub fn new() -> Self {
        Self {
            binary: detect::detect_binary(),
        }
    }
}

impl Default for CodexProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AiProvider for CodexProvider {
    // ─── Identity ───

    fn provider_kind(&self) -> AiProviderKind {
        AiProviderKind::Codex
    }

    fn binary_name(&self) -> &str {
        "codex"
    }

    // ─── Detection ───

    fn detect_binary(&self) -> Option<PathBuf> {
        self.binary.clone()
    }

    fn version(&self) -> Result<String, AiError> {
        let binary = self
            .binary
            .as_ref()
            .ok_or_else(|| AiError::BinaryNotFound("codex".into()))?;
        detect::parse_version(binary).ok_or_else(|| AiError::BinaryNotFound("codex".into()))
    }

    fn is_installed(&self) -> bool {
        self.binary.is_some()
    }

    fn detect_in_repo(&self, repo_path: &Path) -> bool {
        detect::detect_in_repo(repo_path)
    }

    // ─── Headless Execution ───

    fn build_execute_command(
        &self,
        prompt: &str,
        cwd: &Path,
        options: &ExecuteOptions,
    ) -> Result<Command, AiError> {
        let binary = self
            .binary
            .as_ref()
            .ok_or_else(|| AiError::BinaryNotFound("codex".into()))?;
        commands::build_execute_command(binary, prompt, cwd, options)
    }

    // ─── Interactive Launch ───

    fn build_interactive_cmd(&self, cwd: &Path) -> Result<Command, AiError> {
        let binary = self
            .binary
            .as_ref()
            .ok_or_else(|| AiError::BinaryNotFound("codex".into()))?;
        commands::build_interactive_cmd(binary, cwd)
    }

    /// Codex doesn't have a `--skill` flag; skill and saved-prompt content is
    /// expected to already be inlined in `input.prompt` by the coordinator.
    fn launch_background(&self, input: AiBackgroundRunInput) -> Result<Command, AiError> {
        let binary = self
            .binary
            .as_ref()
            .ok_or_else(|| AiError::BinaryNotFound("codex".into()))?;
        Ok(commands::build_background_command(binary, &input))
    }

    fn background_uses_stdin_prompt(&self) -> bool {
        false
    }

    /// Build a resume command for a previously-recorded Codex session.
    ///
    /// Shape: `codex exec resume <session_id> -C <cwd>`. Returns `None` only
    /// if the binary isn't on PATH — matches the Claude Code idiom.
    fn build_resume_session_cmd(&self, session_id: &str, cwd: &Path) -> Option<Command> {
        let binary = self.binary.as_ref()?;
        Some(commands::build_resume_session_cmd(binary, session_id, cwd))
    }

    // ─── Session Introspection ───

    /// List Codex sessions whose `cwd` matches `repo_path`.
    ///
    /// Reads `~/.codex/sessions/` (filtering by cwd) — Codex has no built-in
    /// JSON session-list command, so we parse the on-disk JSONL rollouts
    /// directly. See [`sessions`] for the format.
    fn list_sessions(&self, repo_path: &Path) -> Result<Vec<AiSession>, AiError> {
        let Some(home) = dirs::home_dir() else {
            return Ok(Vec::new());
        };
        let base_dir = home.join(".codex/sessions");
        let all = sessions::load_sessions(&base_dir);
        let target = repo_path;
        Ok(all.into_iter().filter(|s| s.cwd == target).collect())
    }

    /// List Codex conversation transcripts whose rollout `cwd` matches
    /// `repo_path`.
    ///
    /// Transcript-first sibling of [`list_sessions`] — reads the same
    /// `~/.codex/sessions/` tree but surfaces every matching rollout as
    /// an [`AiConversation`] regardless of live-process state. See
    /// [`conversations`] for the walker + filter contract.
    fn list_conversations(&self, repo_path: &Path) -> Result<Vec<AiConversation>, AiError> {
        conversations::list_conversations(repo_path)
    }

    /// Whether a Codex session is still "live".
    ///
    /// Codex does not record a PID in its session metadata, so liveness
    /// falls back to a recency heuristic: sessions whose rollout file has
    /// been written within [`sessions::ACTIVE_WINDOW`] are reported as
    /// active. The [`AiSession::is_active`] field populated at discovery
    /// time takes precedence; we re-scan the session directory only when
    /// that flag is already false (the session may have been written to
    /// since the last list).
    fn is_session_active(&self, session: &AiSession) -> bool {
        if session.is_active {
            return true;
        }
        let Some(home) = dirs::home_dir() else {
            return false;
        };
        let base_dir = home.join(".codex/sessions");
        sessions::is_session_active(&base_dir, &session.id)
    }

    // ─── Worktree Introspection ───

    /// List BeardGit-spawned Codex worktrees under
    /// `<repo>/.beardgit/ai-worktrees/codex/`.
    fn list_worktrees(&self, repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
        worktrees::list_worktrees(repo_path)
    }

    /// Remove the worktree's directory (recursive).
    fn cleanup_worktree(&self, worktree: &AiWorktree) -> Result<(), AiError> {
        worktrees::cleanup_worktree(worktree)
    }

    // ─── Configuration Discovery ───

    /// Discover Codex configuration files for the given repo.
    ///
    /// Scans for:
    /// - `~/.codex/config.toml` (user-scoped settings)
    /// - `<repo>/.codex/config.toml` (project-scoped settings)
    fn config_files(&self, repo_path: &Path) -> Vec<AiConfigFile> {
        let mut files = Vec::new();
        let home = dirs::home_dir().unwrap_or_default();

        // User-level config
        let user_config = home.join(".codex/config.toml");
        if user_config.is_file() {
            files.push(AiConfigFile {
                path: user_config,
                kind: ConfigKind::Settings,
                scope: ConfigScope::User,
            });
        }

        // Project-level config
        let project_config = repo_path.join(".codex/config.toml");
        if project_config.is_file() {
            files.push(AiConfigFile {
                path: project_config,
                kind: ConfigKind::Settings,
                scope: ConfigScope::Project,
            });
        }

        files
    }

    /// Codex reads instructions from conventional repo-level files.
    ///
    /// Modern Codex releases honour `AGENTS.md` (shared convention with other
    /// agents); older / alternate installs may still look at `CODEX.md`. Both
    /// are returned so callers can surface whichever exists.
    fn instruction_files(&self, repo_path: &Path) -> Vec<PathBuf> {
        vec![repo_path.join("AGENTS.md"), repo_path.join("CODEX.md")]
    }

    // ─── Attribution ───

    fn attribution_patterns(&self) -> Vec<AttributionPattern> {
        vec![
            AttributionPattern {
                kind: AttributionMatch::Trailer,
                pattern: "Co-authored-by:.*codex".to_string(),
            },
            AttributionPattern {
                kind: AttributionMatch::AuthorName,
                pattern: "(?i)codex".to_string(),
            },
        ]
    }

    fn is_ai_authored(&self, message: &str, author: &str) -> bool {
        attribution::is_ai_authored(message, author)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn provider_kind_is_codex() {
        let provider = CodexProvider { binary: None };
        assert_eq!(provider.provider_kind(), AiProviderKind::Codex);
    }

    #[test]
    fn binary_name_is_codex() {
        let provider = CodexProvider { binary: None };
        assert_eq!(provider.binary_name(), "codex");
    }

    #[test]
    fn config_files_returns_empty_for_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let provider = CodexProvider { binary: None };
        // Only check project-scoped files — user home may have a real config.
        let files = provider.config_files(dir.path());
        let project_files: Vec<_> = files
            .iter()
            .filter(|f| f.scope == ConfigScope::Project)
            .collect();
        assert!(project_files.is_empty());
    }

    #[test]
    fn instruction_files_returns_agents_and_codex_md() {
        let dir = tempfile::tempdir().unwrap();
        let provider = CodexProvider { binary: None };
        let files = provider.instruction_files(dir.path());
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("AGENTS.md"));
        assert!(files[1].ends_with("CODEX.md"));
    }

    #[test]
    fn attribution_patterns_cover_trailer_and_author() {
        let provider = CodexProvider { binary: None };
        let patterns = provider.attribution_patterns();
        assert!(patterns.iter().any(|p| p.kind == AttributionMatch::Trailer));
        assert!(
            patterns
                .iter()
                .any(|p| p.kind == AttributionMatch::AuthorName)
        );
    }

    #[test]
    fn is_ai_authored_detects_codex_trailer() {
        let provider = CodexProvider { binary: None };
        let msg = "feat: x\n\nCo-authored-by: Codex CLI <codex@openai.com>";
        assert!(provider.is_ai_authored(msg, "Alice"));
        assert!(!provider.is_ai_authored("feat: x", "Alice"));
    }

    #[test]
    fn config_files_finds_project_toml() {
        let dir = tempfile::tempdir().unwrap();
        let codex_dir = dir.path().join(".codex");
        fs::create_dir(&codex_dir).unwrap();
        fs::write(codex_dir.join("config.toml"), "[settings]\n").unwrap();

        let provider = CodexProvider { binary: None };
        let files = provider.config_files(dir.path());
        let project_settings: Vec<_> = files
            .iter()
            .filter(|f| f.scope == ConfigScope::Project && f.kind == ConfigKind::Settings)
            .collect();
        assert_eq!(project_settings.len(), 1);
        assert!(project_settings[0].path.ends_with(".codex/config.toml"));
    }
}
