//! Shared types used by [`crate::CiProvider`] implementations and consumers.
//!
//! No HTTP logic lives here — only data structures. Types are grouped by
//! role: CI status + runs + jobs + stages, user + project identity, trigger
//! inputs + workflow metadata, and the connected-provider status response
//! returned to the frontend.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// User identity
// ---------------------------------------------------------------------------

/// Authenticated user profile returned by token validation.
///
/// Fields that are provider-specific are `Option`:
/// - `email`: GitHub may return `null` if the user's email is private.
/// - `avatar_url`: may be absent on self-hosted instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUser {
    /// Unique user ID on the provider.
    pub id: u64,
    /// Login/username (e.g. `"octocat"` on GitHub, `"johndoe"` on GitLab).
    pub username: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Email address. `None` if the user has a private email on GitHub.
    pub email: Option<String>,
    /// URL to the user's avatar image.
    pub avatar_url: Option<String>,
    /// URL to the user's profile page on the provider.
    pub profile_url: String,
}

// ---------------------------------------------------------------------------
// CI status
// ---------------------------------------------------------------------------

/// Normalized CI/CD status across providers.
///
/// GitLab maps from a single `status` string field.
/// GitHub maps from the combination of `status` + `conclusion` fields.
///
/// Provider-exclusive statuses are included — providers that don't support
/// them simply never return them:
/// - [`CiStatus::Manual`] — GitLab only
/// - [`CiStatus::TimedOut`] — GitHub only
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiStatus {
    /// Job/run is queued but not yet assigned to a runner.
    Queued,
    /// Job/run is waiting for conditions to be met.
    Pending,
    /// Job/run is actively executing.
    Running,
    /// Job/run completed successfully.
    Success,
    /// Job/run completed with failures.
    Failed,
    /// Job/run was canceled by a user.
    Canceled,
    /// Job/run was skipped (e.g., rules not met).
    Skipped,
    /// Job requires manual intervention to proceed (GitLab only).
    Manual,
    /// Job/run exceeded its time limit (GitHub only).
    TimedOut,
    /// Status could not be mapped to a known value.
    Unknown,
}

impl CiStatus {
    /// Returns `true` if this status represents an active (in-progress) state
    /// that should trigger polling for updates.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Queued | Self::Pending | Self::Running)
    }
}

// ---------------------------------------------------------------------------
// Project metadata
// ---------------------------------------------------------------------------

/// Project/repository metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Numeric project/repo ID on the provider.
    pub id: u64,
    /// Human-readable project name.
    pub name: String,
    /// Full path including namespace: `"group/project"` (GitLab) or `"owner/repo"` (GitHub).
    pub full_path: String,
    /// Default branch name (e.g. `"main"`).
    pub default_branch: Option<String>,
    /// URL to the project in the provider's web UI.
    pub web_url: String,
}

// ---------------------------------------------------------------------------
// CI run types
// ---------------------------------------------------------------------------

/// Server-side filters for listing CI runs.
///
/// All fields are optional. The provider translates them into the correct
/// query parameters for its API (e.g., `ref` for GitLab, `branch` for GitHub).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CiFilters {
    /// Filter by branch or tag name.
    pub branch: Option<String>,
    /// Filter by status string (provider normalizes to its own API values).
    pub status: Option<String>,
    /// Filter by trigger source/event.
    pub source: Option<String>,
}

/// Summary of a CI/CD run (pipeline or workflow run) as returned by list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiRun {
    /// Provider's global run ID.
    pub id: u64,
    /// Human-friendly run number: `iid` (GitLab) or `run_number` (GitHub).
    pub display_id: u64,
    /// Normalized status.
    pub status: CiStatus,
    /// Branch or tag name the run executed against.
    pub ref_name: String,
    /// Commit SHA that triggered the run.
    pub sha: String,
    /// Trigger source: `"push"`, `"web"`, `"schedule"` (GitLab) or event name (GitHub).
    pub source: Option<String>,
    /// Human-readable name: pipeline name (GitLab 16.3+) or workflow name (GitHub).
    pub name: Option<String>,
    /// Username of the user / bot that triggered this run.
    ///
    /// GitHub: `triggering_actor.login` on the workflow run.
    /// GitLab: `user.username` on the pipeline.
    /// `None` when the upstream payload omits the actor (e.g. scheduled runs on
    /// some self-hosted GitLab versions).
    pub actor: Option<String>,
    /// ISO 8601 creation timestamp.
    pub created_at: Option<String>,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: Option<String>,
    /// URL to view this run in the provider's web UI.
    pub web_url: String,
}

/// Detailed information about a single CI run, including timing and jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiRunDetail {
    /// The run summary (same fields as list response).
    pub run: CiRun,
    /// Total duration in seconds. GitLab provides this natively.
    /// GitHub computes it from `run_started_at` and `updated_at`.
    pub duration: Option<f64>,
    /// ISO 8601 completion timestamp.
    pub finished_at: Option<String>,
    /// Jobs grouped by stage. GitLab has real stages.
    /// GitHub groups all jobs under a single "Jobs" virtual stage.
    pub stages: Vec<CiStage>,
}

/// A CI stage grouping its constituent jobs.
///
/// GitLab pipelines have named stages (`build`, `test`, `deploy`).
/// GitHub Actions has no stage concept — all jobs are grouped under a
/// single virtual stage named `"Jobs"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiStage {
    /// Stage name. Real for GitLab, `"Jobs"` for GitHub.
    pub name: String,
    /// Jobs belonging to this stage.
    pub jobs: Vec<CiJob>,
}

/// A single CI/CD job within a stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiJob {
    /// Unique job ID.
    pub id: u64,
    /// Human-readable job name.
    pub name: String,
    /// Stage name this job belongs to (GitLab only, `None` for GitHub).
    pub stage: Option<String>,
    /// Normalized status.
    pub status: CiStatus,
    /// Wall-clock duration in seconds. GitLab provides natively.
    /// GitHub computes from `started_at` and `completed_at`.
    pub duration: Option<f64>,
    /// ISO 8601 timestamp when the job started executing.
    pub started_at: Option<String>,
    /// ISO 8601 timestamp when the job finished.
    pub finished_at: Option<String>,
    /// URL to view this job in the provider's web UI.
    pub web_url: String,
    /// Whether this job is allowed to fail without failing the run (GitLab only).
    pub allow_failure: Option<bool>,
    /// Individual steps within this job (GitHub only, `None` for GitLab).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<CiJobStep>>,
}

/// A single step within a CI/CD job (GitHub Actions only).
///
/// GitHub jobs are composed of steps that run sequentially. Each step
/// has its own status, allowing real-time progress tracking for running jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiJobStep {
    /// Step number (1-based).
    pub number: u32,
    /// Human-readable step name.
    pub name: String,
    /// Normalized status.
    pub status: CiStatus,
    /// Wall-clock duration in seconds (`None` if not yet completed).
    pub duration: Option<f64>,
}

// ---------------------------------------------------------------------------
// CI/CD control (Phase 8.4) — trigger + workflow metadata
// ---------------------------------------------------------------------------

/// Input for triggering a new CI run.
///
/// For GitHub Actions, `workflow_id` must be the workflow file name
/// (e.g. `"ci.yml"`) or the numeric workflow ID. For GitLab, `workflow_id`
/// is ignored — there is a single `.gitlab-ci.yml` per project and the
/// trigger is parameterized by `git_ref` only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerWorkflowInput {
    /// GitHub: workflow file name or numeric ID. GitLab: ignored.
    pub workflow_id: String,
    /// Branch or tag name to run against.
    pub git_ref: String,
    /// Workflow-dispatch inputs (GitHub) or pipeline variables (GitLab).
    pub inputs: HashMap<String, String>,
}

/// Result of a successful trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerResult {
    /// Provider-specific run identifier (returned as a string so callers
    /// don't have to know whether GitHub `run_id` is `u64` or `u128`).
    pub run_id: String,
    /// URL to the run in the provider's web UI.
    pub url: String,
}

/// Workflow definition metadata (GitHub only — GitLab returns a placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Provider-specific ID (GitHub: numeric workflow ID as string;
    /// GitLab: always `"default"`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Path to the workflow file inside the repository.
    pub path: String,
    /// Enablement state.
    pub state: WorkflowState,
}

/// Enablement state for a workflow definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    /// Workflow is active and can be triggered.
    Active,
    /// Workflow is disabled by the repo admin (GitHub).
    Disabled,
}

// ---------------------------------------------------------------------------
// Provider status (returned to frontend)
// ---------------------------------------------------------------------------

/// Info about a single connected provider, returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedProvider {
    /// Provider type: `"gitlab"` or `"github"`.
    pub kind: String,
    /// Base URL of the provider instance.
    pub instance_url: String,
    /// Authenticated user profile.
    pub user: ProviderUser,
    /// Detected project name from the current repo, or `None`.
    pub project_name: Option<String>,
}

/// Full provider connection status returned to the frontend.
///
/// Contains all authenticated providers and which one (if any) is active
/// for the currently open repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusResponse {
    /// All authenticated provider connections.
    pub providers: Vec<ConnectedProvider>,
    /// Index into `providers` for the active provider (matching repo remote).
    /// `None` if no repo is open or no provider matches.
    pub active_index: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_status_is_active() {
        assert!(CiStatus::Queued.is_active());
        assert!(CiStatus::Pending.is_active());
        assert!(CiStatus::Running.is_active());
        assert!(!CiStatus::Success.is_active());
        assert!(!CiStatus::Failed.is_active());
        assert!(!CiStatus::Canceled.is_active());
        assert!(!CiStatus::Unknown.is_active());
    }

    #[test]
    fn test_ci_status_serialization() {
        let json = serde_json::to_string(&CiStatus::Success).unwrap();
        assert_eq!(json, "\"success\"");

        let status: CiStatus = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(status, CiStatus::Failed);

        let status: CiStatus = serde_json::from_str("\"timed_out\"").unwrap();
        assert_eq!(status, CiStatus::TimedOut);
    }

    #[test]
    fn test_ci_filters_default() {
        let filters = CiFilters::default();
        assert!(filters.branch.is_none());
        assert!(filters.status.is_none());
        assert!(filters.source.is_none());
    }

    #[test]
    fn test_trigger_workflow_input_serialization() {
        let mut inputs = std::collections::HashMap::new();
        inputs.insert("env".to_string(), "staging".to_string());
        let input = TriggerWorkflowInput {
            workflow_id: "ci.yml".to_string(),
            git_ref: "main".to_string(),
            inputs,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"workflow_id\":\"ci.yml\""));
        assert!(json.contains("\"git_ref\":\"main\""));
        assert!(json.contains("\"env\":\"staging\""));
    }

    #[test]
    fn test_trigger_result_roundtrip() {
        let r = TriggerResult {
            run_id: "12345".to_string(),
            url: "https://gitlab.com/x/p/-/pipelines/12345".to_string(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let decoded: TriggerResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.run_id, "12345");
        assert_eq!(decoded.url, r.url);
    }

    #[test]
    fn test_workflow_state_serialization() {
        let json = serde_json::to_string(&WorkflowState::Active).unwrap();
        assert_eq!(json, "\"active\"");
        let s: WorkflowState = serde_json::from_str("\"disabled\"").unwrap();
        assert_eq!(s, WorkflowState::Disabled);
    }

    #[test]
    fn test_workflow_serialization() {
        let w = Workflow {
            id: "12".to_string(),
            name: "CI".to_string(),
            path: ".github/workflows/ci.yml".to_string(),
            state: WorkflowState::Active,
        };
        let json = serde_json::to_string(&w).unwrap();
        assert!(json.contains("\"id\":\"12\""));
        assert!(json.contains("\"state\":\"active\""));
    }

    #[test]
    fn ci_run_actor_defaults_to_none_and_serializes_snake_case() {
        let r = CiRun {
            id: 1,
            display_id: 1,
            status: CiStatus::Success,
            ref_name: "main".into(),
            sha: "deadbeef".into(),
            source: None,
            name: None,
            actor: None,
            created_at: None,
            updated_at: None,
            web_url: "https://example.test/runs/1".into(),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert!(
            json.get("actor").is_some(),
            "actor must be serialised even when None"
        );
        assert_eq!(json["actor"], serde_json::Value::Null);
    }
}
