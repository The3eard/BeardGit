//! OpenCode AI provider implementation.
//!
//! Implements [`ai_provider::AiProvider`] for the OpenCode CLI tool.
//! Handles binary detection, headless command building (`-p` flag),
//! config discovery, and commit attribution.

pub mod commands;
pub mod detect;

use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider::{
    AiBackgroundRunInput, AiConfigFile, AiError, AiProvider, AiProviderKind, AttributionMatch,
    AttributionPattern, ConfigKind, ConfigScope, ExecuteOptions,
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

    fn config_files(&self, repo_path: &Path) -> Vec<AiConfigFile> {
        let mut files = Vec::new();
        let home = dirs::home_dir().unwrap_or_default();

        // User-level settings
        let user_config = home.join(".opencode/config.json");
        if user_config.is_file() {
            files.push(AiConfigFile {
                path: user_config,
                kind: ConfigKind::Settings,
                scope: ConfigScope::User,
            });
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

    fn instruction_files(&self, _repo_path: &Path) -> Vec<PathBuf> {
        vec![]
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
}
