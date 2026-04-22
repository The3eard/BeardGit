//! OpenCode AI provider implementation.
//!
//! Implements [`ai_provider::AiProvider`] for the OpenCode CLI tool.
//! Handles binary detection, headless command building (`-p` flag),
//! config discovery, and commit attribution.

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

/// AI provider for the OpenCode CLI.
pub struct OpenCodeProvider {
    binary: Option<PathBuf>,
}

impl OpenCodeProvider {
    /// Create a new `OpenCodeProvider` instance.
    ///
    /// Performs binary detection at construction time so subsequent calls
    /// to [`AiProvider::is_installed`] and [`AiProvider::detect_binary`] are free.
    pub fn new() -> Self {
        Self {
            binary: detect::detect_binary(),
        }
    }
}

impl Default for OpenCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AiProvider for OpenCodeProvider {
    fn provider_kind(&self) -> AiProviderKind {
        AiProviderKind::OpenCode
    }

    fn binary_name(&self) -> &str {
        "opencode"
    }

    fn detect_binary(&self) -> Option<PathBuf> {
        self.binary.clone()
    }

    fn version(&self) -> Result<String, AiError> {
        let binary = self
            .binary
            .as_ref()
            .ok_or_else(|| AiError::BinaryNotFound("opencode".into()))?;
        detect::version(binary)
    }

    fn is_installed(&self) -> bool {
        self.binary.is_some()
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

    /// OpenCode has no `--skill` or `--prompt-file` flag; the coordinator
    /// must inline saved-prompt content into `input.prompt` before calling.
    fn launch_background(&self, input: AiBackgroundRunInput) -> Result<Command, AiError> {
        let binary = self
            .binary
            .as_ref()
            .ok_or_else(|| AiError::BinaryNotFound("opencode".into()))?;
        Ok(commands::build_background_command(binary, &input))
    }

    fn background_uses_stdin_prompt(&self) -> bool {
        false
    }

    /// Build a resume command for a previously-recorded OpenCode session.
    ///
    /// Shape: `opencode run --session <id> --dir <cwd>`. Returns `None`
    /// only if the `opencode` binary isn't on PATH — matches the idiom
    /// used by Claude Code and Codex.
    fn build_resume_session_cmd(&self, session_id: &str, cwd: &Path) -> Option<Command> {
        let binary = self.binary.as_ref()?;
        Some(commands::build_resume_session_cmd(binary, session_id, cwd))
    }

    /// List OpenCode sessions by shelling out to
    /// `opencode session list --format json` via [`sessions::CliSessionRunner`].
    ///
    /// OpenCode stores sessions in a SQLite DB; rather than parsing the DB
    /// directly we use the CLI's built-in JSON output. The returned `Vec`
    /// includes ALL sessions — OpenCode's session model doesn't carry a
    /// first-class `cwd` filter arg, so the provider layer / UI can filter
    /// on [`AiSession::cwd`] if per-repo scoping becomes required.
    ///
    /// Returns `Ok(Vec::new())` when the binary isn't installed.
    fn list_sessions(&self, _repo_path: &Path) -> Result<Vec<AiSession>, AiError> {
        let Some(binary) = self.binary.clone() else {
            return Ok(Vec::new());
        };
        let runner = sessions::CliSessionRunner::new(binary);
        Ok(sessions::load_sessions(&runner))
    }

    /// List OpenCode conversation transcripts scoped to `repo_path`.
    ///
    /// Transcript-first sibling of [`AiProvider::list_sessions`] — shells
    /// out to the same `opencode session list --format json` command but
    /// returns [`AiConversation`] rows filtered by `directory` /
    /// `repo_path`. See [`conversations`] for the filter + sort contract.
    ///
    /// Returns `Ok(Vec::new())` when the binary isn't installed.
    fn list_conversations(&self, repo_path: &Path) -> Result<Vec<AiConversation>, AiError> {
        let Some(binary) = self.binary.clone() else {
            return Ok(Vec::new());
        };
        let runner = sessions::CliSessionRunner::new(binary);
        conversations::list_conversations(&runner, repo_path)
    }

    /// Whether an OpenCode session is still "live".
    ///
    /// OpenCode reports `updated` as a Unix-millis timestamp in its
    /// `session list` output. We treat a session as active when its
    /// `updated` is within [`sessions::ACTIVE_WINDOW`] of now. The
    /// [`AiSession::is_active`] flag populated at discovery time takes
    /// precedence; only when it's stale do we re-shell-out to the CLI so
    /// the UI can pick up activity that happened between refreshes.
    fn is_session_active(&self, session: &AiSession) -> bool {
        if session.is_active {
            return true;
        }
        let Some(binary) = self.binary.clone() else {
            return false;
        };
        let runner = sessions::CliSessionRunner::new(binary);
        sessions::is_session_active_by_id(&runner, &session.id)
    }

    /// List BeardGit-spawned OpenCode worktrees under
    /// `<repo>/.beardgit/ai-worktrees/opencode/`.
    fn list_worktrees(&self, repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
        worktrees::list_worktrees(repo_path)
    }

    /// Remove the worktree's directory (recursive, idempotent).
    fn cleanup_worktree(&self, worktree: &AiWorktree) -> Result<(), AiError> {
        worktrees::cleanup_worktree(worktree)
    }

    /// Discover OpenCode configuration files for the given repo.
    ///
    /// Scans for:
    /// - `~/.config/opencode/config.json` (user-scoped settings, XDG path)
    /// - `<repo>/.opencode/config.json` (project-scoped settings)
    fn config_files(&self, repo_path: &Path) -> Vec<AiConfigFile> {
        let mut files = Vec::new();

        // User-level config under the XDG-style location OpenCode uses.
        if let Some(home) = dirs::home_dir() {
            let user_config = home.join(".config/opencode/config.json");
            if user_config.is_file() {
                files.push(AiConfigFile {
                    path: user_config,
                    kind: ConfigKind::Settings,
                    scope: ConfigScope::User,
                });
            }
        }

        // Project-level settings
        let project_config = repo_path.join(".opencode/config.json");
        if project_config.is_file() {
            files.push(AiConfigFile {
                path: project_config,
                kind: ConfigKind::Settings,
                scope: ConfigScope::Project,
            });
        }

        files
    }

    /// OpenCode reads instructions from conventional repo-level files.
    ///
    /// Returns `AGENTS.md` (shared convention across modern AI CLIs) plus
    /// a provider-specific `OPENCODE.md` for installs that still look
    /// there. Both paths are returned unconditionally — existence-filtering
    /// is the caller's responsibility.
    fn instruction_files(&self, repo_path: &Path) -> Vec<PathBuf> {
        vec![repo_path.join("AGENTS.md"), repo_path.join("OPENCODE.md")]
    }

    fn attribution_patterns(&self) -> Vec<AttributionPattern> {
        vec![
            AttributionPattern {
                kind: AttributionMatch::Trailer,
                pattern: "Co-authored-by:.*opencode".to_string(),
            },
            AttributionPattern {
                kind: AttributionMatch::AuthorName,
                pattern: "(?i)opencode".to_string(),
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
    fn provider_kind_is_opencode() {
        let provider = OpenCodeProvider { binary: None };
        assert_eq!(provider.provider_kind(), AiProviderKind::OpenCode);
    }

    #[test]
    fn binary_name_is_opencode() {
        let provider = OpenCodeProvider { binary: None };
        assert_eq!(provider.binary_name(), "opencode");
    }

    #[test]
    fn config_files_returns_empty_for_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let provider = OpenCodeProvider { binary: None };
        let files = provider.config_files(dir.path());
        // Only user-scope files could appear (from actual home dir); project scope must be empty.
        assert!(files.iter().all(|f| f.scope != ConfigScope::Project));
    }

    #[test]
    fn config_files_finds_project_json() {
        let dir = tempfile::tempdir().unwrap();
        let opencode_dir = dir.path().join(".opencode");
        fs::create_dir(&opencode_dir).unwrap();
        fs::write(opencode_dir.join("config.json"), "{}").unwrap();

        let provider = OpenCodeProvider { binary: None };
        let files = provider.config_files(dir.path());
        let project_files: Vec<_> = files
            .iter()
            .filter(|f| f.scope == ConfigScope::Project)
            .collect();
        assert_eq!(project_files.len(), 1);
        assert_eq!(project_files[0].kind, ConfigKind::Settings);
        assert!(project_files[0].path.ends_with(".opencode/config.json"));
    }

    #[test]
    fn instruction_files_returns_agents_and_opencode_md() {
        let dir = tempfile::tempdir().unwrap();
        let provider = OpenCodeProvider { binary: None };
        let files = provider.instruction_files(dir.path());
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("AGENTS.md"));
        assert!(files[1].ends_with("OPENCODE.md"));
    }

    #[test]
    fn attribution_patterns_cover_trailer_and_author() {
        let provider = OpenCodeProvider { binary: None };
        let patterns = provider.attribution_patterns();
        assert!(patterns.iter().any(|p| p.kind == AttributionMatch::Trailer));
        assert!(
            patterns
                .iter()
                .any(|p| p.kind == AttributionMatch::AuthorName)
        );
    }

    #[test]
    fn is_ai_authored_detects_opencode_trailer_and_author() {
        let provider = OpenCodeProvider { binary: None };
        let msg = "feat: x\n\nCo-authored-by: opencode <bot@opencode.ai>";
        assert!(provider.is_ai_authored(msg, "Alice"));
        assert!(provider.is_ai_authored("feat: x", "OpenCode Bot <bot@opencode.ai>"));
        assert!(!provider.is_ai_authored("feat: x", "Alice"));
    }
}
