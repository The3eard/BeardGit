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
///
/// Optional fields (`worktree_path`, `background_status`, `task_id`) are
/// populated only for background runs launched via
/// [`launch_background`](crate::AiProvider::launch_background). Sessions
/// discovered from provider storage (e.g. Claude Code's session files) leave
/// these as `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSession {
    pub id: String,
    pub provider: AiProviderKind,
    pub cwd: PathBuf,
    pub started_at: Option<u64>,
    pub kind: SessionKind,
    pub is_active: bool,
    /// Worktree path for background runs. `None` for ordinary provider
    /// sessions detected on disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_path: Option<PathBuf>,
    /// Live status for background runs. `None` for regular sessions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_status: Option<AiBackgroundRunStatus>,
    /// The [`task_runner::TaskId`] this session is attached to, encoded as
    /// `u64` to keep `ai-provider` free of a `task-runner` dependency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<u64>,
    /// User-typed prompt for background runs.
    ///
    /// Captures the free-text prompt the user submitted via the "New AI
    /// worktree run" dialog so the AI Sessions detail pane can echo what
    /// was asked alongside the captured transcript. Inlined skill/saved
    /// prompt content is excluded — it can be hundreds of lines long
    /// and would dominate the section meant to summarise the user's
    /// command. `None` for sessions discovered on disk and for runs
    /// launched before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// A conversation transcript on disk — the canonical source of truth for a
/// provider-hosted AI session.
///
/// Replaces [`AiSession`] for the on-disk listing path. `AiSession` stays in
/// place for background runs, which are BeardGit-owned processes with a
/// `task_id` + `background_status`, not transcript files on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConversation {
    /// Conversation UUID — the filename stem of the transcript (Claude:
    /// `{uuid}.jsonl`; Codex/OpenCode: provider-specific).
    pub id: String,
    /// Which AI tool produced the transcript.
    pub provider: AiProviderKind,
    /// Repo path the conversation was scoped to at creation.
    pub cwd: PathBuf,
    /// Unix ms timestamp of the earliest parseable message's own timestamp,
    /// falling back to the file's mtime when no timestamp can be extracted.
    pub created_at: u64,
    /// Unix ms timestamp from the file's mtime — "last activity".
    pub last_activity_at: u64,
    /// First real user prompt truncated to ~80 chars (internal newlines
    /// collapsed to spaces). Empty string when the transcript holds no
    /// suitable user message.
    pub title: String,
    /// First 8 chars of the first record's `parentUuid` when non-null —
    /// signals this transcript was forked from another. The UI shows this
    /// as a "Forked from abcd…" badge. `None` for root conversations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
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

/// Input for launching a headless AI background run in a fresh worktree.
///
/// Produced by [`AiProvider::launch_background`](crate::AiProvider::launch_background).
/// All paths are absolute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBackgroundRunInput {
    /// Which AI provider should run the prompt.
    pub provider: AiProviderKind,
    /// Absolute path to the freshly-created worktree where the command runs.
    pub worktree_path: PathBuf,
    /// Free-text prompt. Concatenated with the saved prompt content when both
    /// are provided (the free text acts as the user task on top of the template).
    pub prompt: String,
    /// Optional skill name to invoke via the provider's `--skill` flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    /// Optional path to a saved prompt file whose contents should be prefixed
    /// to the free-text prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_prompt_path: Option<PathBuf>,
    /// Optional session ID to resume (Claude Code `--resume`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_session_id: Option<String>,
    /// When true, pass the provider's permission-skip flag where supported
    /// (e.g. Claude Code's `--dangerously-skip-permissions`). Default: false.
    #[serde(default)]
    pub auto_accept_permissions: bool,
}

/// Lifecycle state of a background run, internally tagged by `state`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AiBackgroundRunStatus {
    /// Waiting in the concurrency queue.
    Queued,
    /// Process spawned and emitting output.
    Running,
    /// Process exited normally.
    Completed {
        exit_code: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token_usage: Option<AiTokenUsage>,
    },
    /// Process failed to spawn or emitted a non-zero exit.
    Failed { message: String },
    /// User cancelled the run.
    Cancelled,
}

impl AiBackgroundRunStatus {
    /// Whether the run is still occupying a concurrency slot.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Whether the run has reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::Cancelled
        )
    }
}

/// Token usage and cost tallies reported by a provider.
///
/// Populated opportunistically — if a provider's headless output doesn't
/// include usage info, this will be `None` in the surrounding
/// [`AiBackgroundRunStatus::Completed`] variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiTokenUsage {
    pub input: u64,
    pub output: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
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
    fn background_input_roundtrip() {
        let input = AiBackgroundRunInput {
            provider: AiProviderKind::ClaudeCode,
            worktree_path: "/tmp/.beardgit/ai-worktrees/feat".into(),
            prompt: "refactor logger".into(),
            skill: Some("review".into()),
            saved_prompt_path: None,
            resume_session_id: None,
            auto_accept_permissions: true,
        };
        let json = serde_json::to_string(&input).unwrap();
        let decoded: AiBackgroundRunInput = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.prompt, "refactor logger");
        assert_eq!(decoded.skill.as_deref(), Some("review"));
        assert!(decoded.auto_accept_permissions);
    }

    #[test]
    fn background_status_completed_serializes_with_state_tag() {
        let status = AiBackgroundRunStatus::Completed {
            exit_code: 0,
            token_usage: Some(AiTokenUsage {
                input: 10,
                output: 20,
                total_cost_usd: Some(0.0025),
            }),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"state\":\"completed\""));
        assert!(json.contains("\"exit_code\":0"));
        assert!(json.contains("\"token_usage\""));

        let decoded: AiBackgroundRunStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, status);
    }

    #[test]
    fn background_status_queued_running_failed_cancelled() {
        for (status, expected_tag) in [
            (AiBackgroundRunStatus::Queued, "queued"),
            (AiBackgroundRunStatus::Running, "running"),
            (
                AiBackgroundRunStatus::Failed {
                    message: "boom".into(),
                },
                "failed",
            ),
            (AiBackgroundRunStatus::Cancelled, "cancelled"),
        ] {
            let json = serde_json::to_string(&status).unwrap();
            assert!(
                json.contains(&format!("\"state\":\"{expected_tag}\"")),
                "expected state={expected_tag}, got {json}"
            );
            let decoded: AiBackgroundRunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, status);
        }
    }

    #[test]
    fn background_status_is_running_and_is_terminal() {
        assert!(AiBackgroundRunStatus::Running.is_running());
        assert!(!AiBackgroundRunStatus::Queued.is_running());
        assert!(
            AiBackgroundRunStatus::Completed {
                exit_code: 0,
                token_usage: None,
            }
            .is_terminal()
        );
        assert!(
            AiBackgroundRunStatus::Failed {
                message: "x".into(),
            }
            .is_terminal()
        );
        assert!(AiBackgroundRunStatus::Cancelled.is_terminal());
        assert!(!AiBackgroundRunStatus::Running.is_terminal());
    }

    #[test]
    fn token_usage_roundtrip() {
        let usage = AiTokenUsage {
            input: 1_000,
            output: 2_000,
            total_cost_usd: Some(0.12),
        };
        let json = serde_json::to_string(&usage).unwrap();
        let decoded: AiTokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, usage);
    }

    #[test]
    fn ai_session_backward_compatible_without_new_fields() {
        // Old session JSON that pre-dates background-run fields should still
        // deserialize — the new optional fields default to None via #[serde(default)].
        let json = r#"{
            "id": "s1",
            "provider": "claude_code",
            "cwd": "/tmp/repo",
            "started_at": null,
            "kind": "headless",
            "is_active": false
        }"#;
        let session: AiSession = serde_json::from_str(json).unwrap();
        assert!(session.worktree_path.is_none());
        assert!(session.background_status.is_none());
        assert!(session.task_id.is_none());
    }

    #[test]
    fn ai_conversation_serializes() {
        let convo = AiConversation {
            id: "abc-123".into(),
            provider: AiProviderKind::ClaudeCode,
            cwd: "/tmp/repo".into(),
            created_at: 1_700_000_000_000,
            last_activity_at: 1_700_000_100_000,
            title: "fix the flaky test".into(),
            parent_id: None,
        };
        let json = serde_json::to_string(&convo).unwrap();
        assert!(json.contains("\"id\":\"abc-123\""));
        assert!(json.contains("\"provider\":\"claude_code\""));
        assert!(json.contains("\"cwd\":\"/tmp/repo\""));
        assert!(json.contains("\"created_at\":1700000000000"));
        assert!(json.contains("\"last_activity_at\":1700000100000"));
        assert!(json.contains("\"title\":\"fix the flaky test\""));
        // `parent_id` must be omitted when `None`.
        assert!(
            !json.contains("parent_id"),
            "parent_id should be skipped when None, got {json}"
        );
    }

    #[test]
    fn ai_conversation_roundtrip() {
        let convo = AiConversation {
            id: "abc-123".into(),
            provider: AiProviderKind::ClaudeCode,
            cwd: "/tmp/repo".into(),
            created_at: 1_700_000_000_000,
            last_activity_at: 1_700_000_100_000,
            title: "fix the flaky test".into(),
            parent_id: Some("deadbeef".into()),
        };
        let json = serde_json::to_string(&convo).unwrap();
        assert!(json.contains("\"parent_id\":\"deadbeef\""));
        let decoded: AiConversation = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, convo.id);
        assert_eq!(decoded.provider, convo.provider);
        assert_eq!(decoded.cwd, convo.cwd);
        assert_eq!(decoded.created_at, convo.created_at);
        assert_eq!(decoded.last_activity_at, convo.last_activity_at);
        assert_eq!(decoded.title, convo.title);
        assert_eq!(decoded.parent_id, convo.parent_id);
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
