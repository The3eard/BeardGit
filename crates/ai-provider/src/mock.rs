//! A configurable mock [`AiProvider`] for downstream command-layer tests.
//!
//! Mirrors the shape of `forge-provider::mock::MockProvider`: every method
//! on the [`AiProvider`] trait returns deterministic output driven by public
//! fields on [`MockAiProvider`]. Tests override individual fields to exercise
//! specific branches without spawning real binaries.
//!
//! ```no_run
//! use ai_provider::mock::MockAiProvider;
//! use ai_provider::AiProviderKind;
//!
//! let mut mock = MockAiProvider::default();
//! mock.installed = false;
//! assert!(!mock.is_installed_value);
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    AiBackgroundRunInput, AiConfigFile, AiError, AiProvider, AiProviderKind, AiSession, AiWorktree,
    AttributionPattern, ExecuteOptions,
};

/// A configurable mock [`AiProvider`] for tests.
///
/// Every field corresponds to a single answer the mock can give. `Default`
/// yields a happy-path "installed provider" so most tests only need to
/// override one or two fields.
pub struct MockAiProvider {
    /// Kind reported by [`AiProvider::provider_kind`].
    pub kind: AiProviderKind,
    /// Binary name reported by [`AiProvider::binary_name`].
    pub binary_name_value: &'static str,
    /// If true, [`detect_binary`](AiProvider::detect_binary) returns `binary_path`.
    pub installed: bool,
    /// Path returned by [`detect_binary`](AiProvider::detect_binary) when `installed`.
    pub binary_path: PathBuf,
    /// Convenience cache of `installed` for test readability.
    pub is_installed_value: bool,
    /// Result of [`AiProvider::version`].
    pub version: Result<String, AiError>,
    /// If true, [`detect_in_repo`](AiProvider::detect_in_repo) always returns true.
    pub in_repo: bool,
    /// Sessions returned by [`list_sessions`](AiProvider::list_sessions).
    pub sessions: Vec<AiSession>,
    /// Worktrees returned by [`list_worktrees`](AiProvider::list_worktrees).
    pub worktrees: Vec<AiWorktree>,
    /// Config files returned by [`config_files`](AiProvider::config_files).
    pub config_files_value: Vec<AiConfigFile>,
    /// Instruction files returned by [`instruction_files`](AiProvider::instruction_files).
    pub instruction_files_value: Vec<PathBuf>,
    /// Attribution patterns returned by [`attribution_patterns`](AiProvider::attribution_patterns).
    pub attribution_patterns_value: Vec<AttributionPattern>,
    /// If true, [`launch_background`](AiProvider::launch_background) is supported and returns a trivial `echo` command.
    pub background_supported: bool,
    /// Override return value of [`background_uses_stdin_prompt`](AiProvider::background_uses_stdin_prompt).
    pub background_uses_stdin_prompt_value: bool,
    /// If true, [`cleanup_worktree`](AiProvider::cleanup_worktree) returns `Ok(())`; otherwise `NotSupported`.
    pub cleanup_worktree_ok: bool,
    /// If true, [`is_session_active`](AiProvider::is_session_active) always returns true.
    pub session_active: bool,
}

impl Default for MockAiProvider {
    fn default() -> Self {
        Self {
            kind: AiProviderKind::ClaudeCode,
            binary_name_value: "mock-ai",
            installed: true,
            binary_path: PathBuf::from("/mock/bin/mock-ai"),
            is_installed_value: true,
            version: Ok("1.0.0-mock".to_string()),
            in_repo: false,
            sessions: Vec::new(),
            worktrees: Vec::new(),
            config_files_value: Vec::new(),
            instruction_files_value: Vec::new(),
            attribution_patterns_value: Vec::new(),
            background_supported: false,
            background_uses_stdin_prompt_value: false,
            cleanup_worktree_ok: false,
            session_active: false,
        }
    }
}

/// A tiny cross-platform "no-op" command used to satisfy methods that must
/// return a [`Command`] without actually spawning anything useful.
fn noop_command() -> Command {
    if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", "exit", "0"]);
        c
    } else {
        Command::new("true")
    }
}

impl AiProvider for MockAiProvider {
    fn provider_kind(&self) -> AiProviderKind {
        self.kind
    }

    fn binary_name(&self) -> &str {
        self.binary_name_value
    }

    fn detect_binary(&self) -> Option<PathBuf> {
        if self.installed {
            Some(self.binary_path.clone())
        } else {
            None
        }
    }

    fn version(&self) -> Result<String, AiError> {
        match &self.version {
            Ok(v) => Ok(v.clone()),
            Err(e) => Err(match e {
                AiError::BinaryNotFound(s) => AiError::BinaryNotFound(s.clone()),
                AiError::CommandBuild(s) => AiError::CommandBuild(s.clone()),
                AiError::Parse(s) => AiError::Parse(s.clone()),
                AiError::NotSupported => AiError::NotSupported,
                AiError::Io(e) => AiError::Io(std::io::Error::new(e.kind(), e.to_string())),
            }),
        }
    }

    fn is_installed(&self) -> bool {
        self.is_installed_value
    }

    fn detect_in_repo(&self, _repo_path: &Path) -> bool {
        self.in_repo
    }

    fn build_execute_command(
        &self,
        _prompt: &str,
        _cwd: &Path,
        _options: &ExecuteOptions,
    ) -> Result<Command, AiError> {
        Ok(noop_command())
    }

    fn launch_background(&self, _input: AiBackgroundRunInput) -> Result<Command, AiError> {
        if self.background_supported {
            Ok(noop_command())
        } else {
            Err(AiError::NotSupported)
        }
    }

    fn background_uses_stdin_prompt(&self) -> bool {
        self.background_uses_stdin_prompt_value
    }

    fn build_interactive_cmd(&self, _cwd: &Path) -> Result<Command, AiError> {
        Ok(noop_command())
    }

    fn list_sessions(&self, _repo_path: &Path) -> Result<Vec<AiSession>, AiError> {
        Ok(self.sessions.clone())
    }

    fn is_session_active(&self, _session: &AiSession) -> bool {
        self.session_active
    }

    fn list_worktrees(&self, _repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
        Ok(self.worktrees.clone())
    }

    fn cleanup_worktree(&self, _worktree: &AiWorktree) -> Result<(), AiError> {
        if self.cleanup_worktree_ok {
            Ok(())
        } else {
            Err(AiError::NotSupported)
        }
    }

    fn config_files(&self, _repo_path: &Path) -> Vec<AiConfigFile> {
        self.config_files_value.clone()
    }

    fn instruction_files(&self, _repo_path: &Path) -> Vec<PathBuf> {
        self.instruction_files_value.clone()
    }

    fn attribution_patterns(&self) -> Vec<AttributionPattern> {
        self.attribution_patterns_value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn default_mock_is_happy_path() {
        let mock = MockAiProvider::default();
        assert_eq!(mock.provider_kind(), AiProviderKind::ClaudeCode);
        assert!(mock.is_installed());
        assert!(mock.detect_binary().is_some());
        assert_eq!(mock.version().unwrap(), "1.0.0-mock");
    }

    #[test]
    fn overriding_installed_propagates() {
        let mock = MockAiProvider {
            installed: false,
            is_installed_value: false,
            ..Default::default()
        };
        assert!(!mock.is_installed());
        assert!(mock.detect_binary().is_none());
    }

    #[test]
    fn overriding_version_error_surfaces() {
        let mock = MockAiProvider {
            version: Err(AiError::BinaryNotFound("mock-ai".into())),
            ..Default::default()
        };
        assert!(matches!(mock.version(), Err(AiError::BinaryNotFound(_))));
    }

    #[test]
    fn launch_background_respects_supported_flag() {
        let mock = MockAiProvider::default();
        assert!(matches!(
            mock.launch_background(AiBackgroundRunInput {
                provider: AiProviderKind::ClaudeCode,
                worktree_path: PathBuf::from("/tmp/wt"),
                prompt: "hi".into(),
                skill: None,
                saved_prompt_path: None,
                resume_session_id: None,
                auto_accept_permissions: false,
            }),
            Err(AiError::NotSupported)
        ));

        let mock = MockAiProvider {
            background_supported: true,
            ..Default::default()
        };
        assert!(
            mock.launch_background(AiBackgroundRunInput {
                provider: AiProviderKind::ClaudeCode,
                worktree_path: PathBuf::from("/tmp/wt"),
                prompt: "hi".into(),
                skill: None,
                saved_prompt_path: None,
                resume_session_id: None,
                auto_accept_permissions: false,
            })
            .is_ok()
        );
    }

    #[test]
    fn mock_is_object_safe_as_dyn_trait() {
        let mock: Arc<dyn AiProvider> = Arc::new(MockAiProvider::default());
        assert_eq!(mock.provider_kind(), AiProviderKind::ClaudeCode);
    }
}
