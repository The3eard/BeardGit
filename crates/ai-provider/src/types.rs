//! Shared types for AI provider integration.
//!
//! All types use `serde` for Tauri IPC serialization. Enums use `snake_case`
//! rename to match the workspace convention.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Identifies which AI coding tool a provider represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProviderKind {
    ClaudeCode,
    Codex,
    OpenCode,
}

impl AiProviderKind {
    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::OpenCode => "OpenCode",
        }
    }
}

/// A detected AI session for a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSession {
    pub id: String,
    pub provider: AiProviderKind,
    pub cwd: PathBuf,
    pub started_at: Option<u64>,
    pub kind: SessionKind,
    pub is_active: bool,
}

/// Whether a session was interactive (REPL) or headless (single-shot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Interactive,
    Headless,
}

/// An AI-created git worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiWorktree {
    pub path: PathBuf,
    pub branch: String,
    pub provider: AiProviderKind,
    pub session_id: Option<String>,
    pub status: WorktreeStatus,
}

/// Status of an AI worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeStatus {
    /// Has an active session (PID alive).
    Active,
    /// Exists but no active session.
    Clean,
    /// Branch or path is dangling.
    Orphaned,
}

/// A configuration file belonging to an AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfigFile {
    pub path: PathBuf,
    pub kind: ConfigKind,
    pub scope: ConfigScope,
}

/// Type of AI configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigKind {
    Settings,
    Instructions,
    Agent,
    Skill,
}

/// Scope of a configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigScope {
    User,
    Project,
    Local,
}

/// Options for headless CLI execution.
#[derive(Debug, Clone, Default)]
pub struct ExecuteOptions {
    /// Output format for the CLI tool.
    pub output_format: OutputFormat,
    /// Override the model used by the AI tool.
    pub model: Option<String>,
    /// Extra CLI flags appended to the command.
    pub extra_args: Vec<String>,
    /// Maximum spend budget (provider-specific).
    pub max_budget: Option<f64>,
}

/// Output format for headless execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// A pattern for detecting AI-authored commits.
#[derive(Debug, Clone)]
pub struct AttributionPattern {
    pub kind: AttributionMatch,
    pub pattern: String,
}

/// Where to look for attribution evidence in a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributionMatch {
    /// Match in commit message footer (e.g., `Authored-by:`).
    Footer,
    /// Match in git trailer (e.g., `Co-authored-by:`).
    Trailer,
    /// Match in the commit author name.
    AuthorName,
}

/// Lightweight metadata about an installed AI provider.
/// Stored in `AppState` — no trait object, no HTTP client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableAiProvider {
    pub kind: AiProviderKind,
    pub binary_path: PathBuf,
    pub version: Option<String>,
}

/// Per-repo AI status returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoAiStatus {
    pub kind: AiProviderKind,
    pub has_config: bool,
    pub session_count: usize,
    pub worktree_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_serializes_snake_case() {
        let json = serde_json::to_string(&AiProviderKind::ClaudeCode).unwrap();
        assert_eq!(json, "\"claude_code\"");

        let json = serde_json::to_string(&AiProviderKind::OpenCode).unwrap();
        assert_eq!(json, "\"open_code\"");
    }

    #[test]
    fn provider_kind_deserializes() {
        let kind: AiProviderKind = serde_json::from_str("\"claude_code\"").unwrap();
        assert_eq!(kind, AiProviderKind::ClaudeCode);
    }

    #[test]
    fn provider_kind_display_name() {
        assert_eq!(AiProviderKind::ClaudeCode.display_name(), "Claude Code");
        assert_eq!(AiProviderKind::Codex.display_name(), "Codex");
    }

    #[test]
    fn session_kind_serializes() {
        let json = serde_json::to_string(&SessionKind::Interactive).unwrap();
        assert_eq!(json, "\"interactive\"");
    }

    #[test]
    fn worktree_status_serializes() {
        let json = serde_json::to_string(&WorktreeStatus::Orphaned).unwrap();
        assert_eq!(json, "\"orphaned\"");
    }

    #[test]
    fn execute_options_default() {
        let opts = ExecuteOptions::default();
        assert_eq!(opts.output_format, OutputFormat::Text);
        assert!(opts.model.is_none());
        assert!(opts.extra_args.is_empty());
        assert!(opts.max_budget.is_none());
    }

    #[test]
    fn available_provider_roundtrip() {
        let provider = AvailableAiProvider {
            kind: AiProviderKind::ClaudeCode,
            binary_path: "/usr/local/bin/claude".into(),
            version: Some("2.1.104".into()),
        };
        let json = serde_json::to_string(&provider).unwrap();
        let decoded: AvailableAiProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.kind, AiProviderKind::ClaudeCode);
        assert_eq!(decoded.version.as_deref(), Some("2.1.104"));
    }

    #[test]
    fn repo_ai_status_serializes() {
        let status = RepoAiStatus {
            kind: AiProviderKind::ClaudeCode,
            has_config: true,
            session_count: 2,
            worktree_count: 1,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"claude_code\""));
        assert!(json.contains("\"session_count\":2"));
    }
}
