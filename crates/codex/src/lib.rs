//! Codex AI provider implementation.
//!
//! Implements [`ai_provider::AiProvider`] for the Codex CLI tool.
//! Handles binary detection, headless command building (`codex exec -p`),
//! interactive launch, and config file discovery in `.codex/`.

pub mod commands;
pub mod detect;

use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider::{
    AiBackgroundRunInput, AiConfigFile, AiError, AiProvider, AiProviderKind, AttributionMatch,
    AttributionPattern, ConfigKind, ConfigScope, ExecuteOptions,
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

    /// Codex does not use instruction files — returns empty vec.
    fn instruction_files(&self, _repo_path: &Path) -> Vec<PathBuf> {
        vec![]
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
