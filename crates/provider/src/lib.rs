//! Unified provider abstraction for git hosting services.
//!
//! This crate defines the [`CiProvider`] trait and all shared types used by
//! both the GitLab and GitHub provider implementations. It contains no HTTP
//! logic — only the contract and data structures.
//!
//! ## Architecture
//!
//! - [`CiProvider`] — async trait that GitLab and GitHub providers implement
//! - [`CiRun`], [`CiJob`], [`CiStage`] — normalized CI/CD types
//! - [`CiStatus`] — unified status enum across both providers
//! - [`ProviderUser`], [`Project`] — common identity and project types
//! - [`ProviderError`] — provider-agnostic error type
//! - [`parse_remote_url`] — detect provider from git remote URL

pub mod log_preprocessor;
pub mod types;
pub use types::{TriggerResult, TriggerWorkflowInput, Workflow, WorkflowState};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Provider kind
// ---------------------------------------------------------------------------

/// Identifies which git hosting provider is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    /// GitLab (cloud or self-hosted).
    GitLab,
    /// GitHub (cloud or GitHub Enterprise).
    GitHub,
}

impl ProviderKind {
    /// Parse a provider kind from a config string.
    ///
    /// Returns `None` for unrecognized strings.
    pub fn from_config_str(s: &str) -> Option<Self> {
        match s {
            "gitlab" => Some(Self::GitLab),
            "github" => Some(Self::GitHub),
            _ => None,
        }
    }

    /// Return the string representation used in config files.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GitLab => "gitlab",
            Self::GitHub => "github",
        }
    }
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
// User and project types
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

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by provider operations.
///
/// Wraps HTTP transport errors, API-level errors (non-2xx responses), and
/// JSON deserialization failures. Provider implementations convert their
/// internal errors into this type.
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    /// HTTP transport-level error (timeout, DNS, TLS).
    #[error("HTTP error: {0}")]
    Http(String),
    /// Provider API returned a non-2xx status code.
    #[error("API error {status}: {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Response body or error message.
        message: String,
    },
    /// Failed to deserialize the response body.
    #[error("JSON error: {0}")]
    Json(String),
    /// Rate limit exceeded (GitHub: 5,000 req/hour).
    #[error("Rate limited — retry after {retry_after_secs}s")]
    RateLimited {
        /// Seconds until the rate limit resets.
        retry_after_secs: u64,
    },
    /// Operation is not supported by this provider (e.g. GitLab has no draft releases).
    #[error("operation not supported by this provider")]
    NotSupported,
}

// ---------------------------------------------------------------------------
// CiProvider trait
// ---------------------------------------------------------------------------

/// Unified interface for git hosting provider integrations.
///
/// Each provider (GitLab, GitHub) implements this trait to normalize their
/// API responses into the shared types defined in this crate. The application
/// holds a `Box<dyn CiProvider>` and never interacts with provider-specific
/// types directly.
///
/// # Error Handling
///
/// All methods return [`ProviderError`], which wraps HTTP errors, API errors,
/// and JSON deserialization failures in a provider-agnostic way.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to work with Tauri's async command
/// system and the `Mutex`-wrapped `AppState`.
#[async_trait::async_trait]
pub trait CiProvider: Send + Sync {
    /// Validate the stored token by fetching the authenticated user's profile.
    ///
    /// Returns the user's profile on success, or an error if the token is
    /// invalid or expired.
    async fn validate_token(&self) -> Result<ProviderUser, ProviderError>;

    /// Fetch metadata about a project/repository.
    ///
    /// `project_ref` is provider-specific: URL-encoded path for GitLab,
    /// `"owner/repo"` for GitHub.
    async fn get_project(&self, project_ref: &str) -> Result<Project, ProviderError>;

    /// List CI/CD runs (pipelines/workflow runs) with optional filters.
    ///
    /// Results are ordered newest-first. Pagination uses 1-based page numbers.
    /// All filtering is server-side — the caller does not filter results locally.
    async fn list_ci_runs(
        &self,
        project_ref: &str,
        filters: &CiFilters,
        per_page: u32,
        page: u32,
    ) -> Result<Vec<CiRun>, ProviderError>;

    /// Fetch full detail for a single CI run, including stages and jobs.
    ///
    /// GitLab returns real stages. GitHub has no stages — the implementation
    /// groups all jobs into a single virtual stage named `"Jobs"`.
    async fn get_ci_run_detail(
        &self,
        project_ref: &str,
        run_id: u64,
    ) -> Result<CiRunDetail, ProviderError>;

    /// Fetch the raw log output for a CI job as plain text.
    ///
    /// GitLab returns the log body directly. GitHub returns a 302 redirect
    /// that the implementation follows transparently.
    async fn get_job_log(&self, project_ref: &str, job_id: u64) -> Result<String, ProviderError>;

    // ---- CI/CD control (Phase 8.4) ----
    // Default impls return NotSupported. Providers opt-in by overriding.

    /// Trigger a new CI run on `git_ref` with optional inputs/variables.
    ///
    /// GitHub: dispatches `workflow_id` via `POST /actions/workflows/{id}/dispatches`.
    /// GitLab: triggers a new pipeline via `POST /projects/:id/pipeline?ref=...`.
    async fn trigger_workflow(
        &self,
        _project_ref: &str,
        _input: &crate::types::TriggerWorkflowInput,
    ) -> Result<crate::types::TriggerResult, ProviderError> {
        Err(ProviderError::NotSupported)
    }

    /// Re-run all jobs of a previously completed run.
    async fn retry_run(&self, _project_ref: &str, _run_id: &str) -> Result<(), ProviderError> {
        Err(ProviderError::NotSupported)
    }

    /// Re-run only the failed jobs of a previously completed run.
    async fn retry_failed_jobs(
        &self,
        _project_ref: &str,
        _run_id: &str,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::NotSupported)
    }

    /// Re-run a specific failed job.
    async fn retry_job(&self, _project_ref: &str, _job_id: &str) -> Result<(), ProviderError> {
        Err(ProviderError::NotSupported)
    }

    /// Cancel an in-progress run.
    async fn cancel_run(&self, _project_ref: &str, _run_id: &str) -> Result<(), ProviderError> {
        Err(ProviderError::NotSupported)
    }

    /// List workflow definitions for the project.
    ///
    /// GitHub returns all `.github/workflows/*.yml` files. GitLab returns
    /// a single placeholder element representing the effective `.gitlab-ci.yml`.
    async fn list_workflows(
        &self,
        _project_ref: &str,
    ) -> Result<Vec<crate::types::Workflow>, ProviderError> {
        Err(ProviderError::NotSupported)
    }

    /// Returns which provider this instance represents.
    fn provider_kind(&self) -> ProviderKind;

    /// Returns the base URL of the provider instance (e.g. `"https://gitlab.com"`).
    fn base_url(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Remote URL parser
// ---------------------------------------------------------------------------

/// Parse a git remote URL to detect the provider and extract the project reference.
///
/// Returns `(ProviderKind, project_ref)` where `project_ref` is:
/// - GitLab: `"group/project"` (used URL-encoded in API calls)
/// - GitHub: `"owner/repo"`
///
/// For well-known hosts (`github.com`, `gitlab.com`), detection is automatic.
/// For self-hosted instances, pass the connected provider's base URL and kind
/// so the parser can match the domain.
///
/// # Examples
///
/// ```
/// use provider::{parse_remote_url, ProviderKind};
///
/// // GitHub SSH
/// let (kind, project) = parse_remote_url(
///     "git@github.com:owner/repo.git", None, None,
/// ).unwrap();
/// assert_eq!(kind, ProviderKind::GitHub);
/// assert_eq!(project, "owner/repo");
///
/// // GitLab HTTPS
/// let (kind, project) = parse_remote_url(
///     "https://gitlab.com/group/project.git", None, None,
/// ).unwrap();
/// assert_eq!(kind, ProviderKind::GitLab);
/// assert_eq!(project, "group/project");
/// ```
pub fn parse_remote_url(
    remote_url: &str,
    provider_base_url: Option<&str>,
    provider_kind: Option<ProviderKind>,
) -> Option<(ProviderKind, String)> {
    // 1. Try well-known hosts
    if let Some(result) = try_well_known_host(remote_url) {
        return Some(result);
    }

    // 2. Try matching against the connected provider's base URL
    if let (Some(base_url), Some(kind)) = (provider_base_url, provider_kind)
        && let Some(path) = try_match_base_url(remote_url, base_url)
    {
        return Some((kind, path));
    }

    None
}

/// Check if the remote URL points to a well-known host (github.com or gitlab.com).
fn try_well_known_host(remote_url: &str) -> Option<(ProviderKind, String)> {
    // SSH format: git@<host>:<path>.git
    if let Some(after_at) = remote_url.strip_prefix("git@")
        && let Some((host, path_with_git)) = after_at.split_once(':')
    {
        let path = path_with_git.trim_end_matches(".git");
        let kind = host_to_kind(host)?;
        return Some((kind, path.to_string()));
    }

    // HTTPS format: https://<host>/<path>.git
    if remote_url.starts_with("http") {
        let without_scheme = remote_url
            .strip_prefix("https://")
            .or_else(|| remote_url.strip_prefix("http://"))?;
        let (host, path_with_slash) = without_scheme.split_once('/')?;
        let kind = host_to_kind(host)?;
        let path = path_with_slash.trim_end_matches(".git");
        return Some((kind, path.to_string()));
    }

    None
}

/// Map a hostname to a well-known provider kind.
fn host_to_kind(host: &str) -> Option<ProviderKind> {
    if host == "github.com" {
        Some(ProviderKind::GitHub)
    } else if host == "gitlab.com" {
        Some(ProviderKind::GitLab)
    } else {
        None
    }
}

/// Try to match a remote URL against a provider's base URL.
///
/// Extracts the project path from both SSH and HTTPS URLs by matching
/// the domain from the base URL.
fn try_match_base_url(remote_url: &str, base_url: &str) -> Option<String> {
    let base_domain = extract_domain(base_url)?;

    // SSH format: git@<domain>:<path>.git
    if let Some(after_at) = remote_url.strip_prefix("git@")
        && let Some((host, path_with_git)) = after_at.split_once(':')
        && host == base_domain
    {
        let path = path_with_git.trim_end_matches(".git");
        return Some(path.to_string());
    }

    // HTTPS format: https://<domain>/<path>.git
    if remote_url.starts_with("http") {
        let base_trimmed = base_url.trim_end_matches('/');
        if let Some(path) = remote_url.strip_prefix(base_trimmed) {
            let path = path.trim_start_matches('/').trim_end_matches(".git");
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
    }

    None
}

/// Extract the domain from a URL (e.g. `"https://gitlab.example.com"` → `"gitlab.example.com"`).
fn extract_domain(url: &str) -> Option<&str> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let domain = without_scheme.split('/').next()?;
    Some(domain)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- CiStatus tests --

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
    fn test_provider_kind_from_str() {
        assert_eq!(
            ProviderKind::from_config_str("gitlab"),
            Some(ProviderKind::GitLab)
        );
        assert_eq!(
            ProviderKind::from_config_str("github"),
            Some(ProviderKind::GitHub)
        );
        assert_eq!(ProviderKind::from_config_str("bitbucket"), None);
    }

    #[test]
    fn test_provider_kind_serialization() {
        let json = serde_json::to_string(&ProviderKind::GitHub).unwrap();
        assert_eq!(json, "\"github\"");

        let kind: ProviderKind = serde_json::from_str("\"gitlab\"").unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
    }

    #[test]
    fn test_ci_filters_default() {
        let filters = CiFilters::default();
        assert!(filters.branch.is_none());
        assert!(filters.status.is_none());
        assert!(filters.source.is_none());
    }

    // -- Remote URL parser tests --

    #[test]
    fn test_parse_github_ssh() {
        let (kind, project) =
            parse_remote_url("git@github.com:owner/repo.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitHub);
        assert_eq!(project, "owner/repo");
    }

    #[test]
    fn test_parse_github_https() {
        let (kind, project) =
            parse_remote_url("https://github.com/owner/repo.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitHub);
        assert_eq!(project, "owner/repo");
    }

    #[test]
    fn test_parse_gitlab_ssh() {
        let (kind, project) =
            parse_remote_url("git@gitlab.com:group/project.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "group/project");
    }

    #[test]
    fn test_parse_gitlab_https() {
        let (kind, project) =
            parse_remote_url("https://gitlab.com/group/project.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "group/project");
    }

    #[test]
    fn test_parse_gitlab_https_no_git_suffix() {
        let (kind, project) =
            parse_remote_url("https://gitlab.com/group/project", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "group/project");
    }

    #[test]
    fn test_parse_self_hosted_gitlab_ssh() {
        let result = parse_remote_url(
            "git@gitlab.internal.com:team/app.git",
            Some("https://gitlab.internal.com"),
            Some(ProviderKind::GitLab),
        );
        let (kind, project) = result.unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "team/app");
    }

    #[test]
    fn test_parse_self_hosted_gitlab_https() {
        let result = parse_remote_url(
            "https://gitlab.internal.com/team/app.git",
            Some("https://gitlab.internal.com"),
            Some(ProviderKind::GitLab),
        );
        let (kind, project) = result.unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "team/app");
    }

    #[test]
    fn test_parse_github_enterprise_ssh() {
        let result = parse_remote_url(
            "git@github.enterprise.com:org/repo.git",
            Some("https://github.enterprise.com"),
            Some(ProviderKind::GitHub),
        );
        let (kind, project) = result.unwrap();
        assert_eq!(kind, ProviderKind::GitHub);
        assert_eq!(project, "org/repo");
    }

    #[test]
    fn test_parse_unknown_host_no_base_url() {
        let result = parse_remote_url("git@unknown.com:org/repo.git", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_subgroup_gitlab() {
        let (kind, project) =
            parse_remote_url("git@gitlab.com:group/subgroup/project.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "group/subgroup/project");
    }

    // -- TriggerWorkflowInput / Workflow tests --

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

    // -- CiProvider default method tests --

    struct MockCiProvider;

    #[async_trait::async_trait]
    impl CiProvider for MockCiProvider {
        async fn validate_token(&self) -> Result<ProviderUser, ProviderError> {
            unreachable!()
        }
        async fn get_project(&self, _: &str) -> Result<Project, ProviderError> {
            unreachable!()
        }
        async fn list_ci_runs(
            &self,
            _: &str,
            _: &CiFilters,
            _: u32,
            _: u32,
        ) -> Result<Vec<CiRun>, ProviderError> {
            unreachable!()
        }
        async fn get_ci_run_detail(&self, _: &str, _: u64) -> Result<CiRunDetail, ProviderError> {
            unreachable!()
        }
        async fn get_job_log(&self, _: &str, _: u64) -> Result<String, ProviderError> {
            unreachable!()
        }
        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::GitHub
        }
        fn base_url(&self) -> &str {
            "https://example.test"
        }
        // Do NOT override trigger_workflow / retry_run / retry_failed_jobs /
        // retry_job / cancel_run / list_workflows — default impls must return
        // NotSupported for a mock that doesn't implement them.
    }

    fn is_not_supported(err: &ProviderError) -> bool {
        matches!(err, ProviderError::NotSupported)
    }

    #[tokio::test]
    async fn test_default_trigger_workflow_not_supported() {
        let p = MockCiProvider;
        let err = p
            .trigger_workflow(
                "owner/repo",
                &TriggerWorkflowInput {
                    workflow_id: "ci.yml".into(),
                    git_ref: "main".into(),
                    inputs: Default::default(),
                },
            )
            .await
            .unwrap_err();
        assert!(is_not_supported(&err), "expected NotSupported, got {err:?}");
    }

    #[tokio::test]
    async fn test_default_retry_run_not_supported() {
        assert!(is_not_supported(
            &MockCiProvider
                .retry_run("owner/repo", "1")
                .await
                .unwrap_err()
        ));
    }

    #[tokio::test]
    async fn test_default_retry_failed_jobs_not_supported() {
        assert!(is_not_supported(
            &MockCiProvider
                .retry_failed_jobs("owner/repo", "1")
                .await
                .unwrap_err()
        ));
    }

    #[tokio::test]
    async fn test_default_retry_job_not_supported() {
        assert!(is_not_supported(
            &MockCiProvider
                .retry_job("owner/repo", "1")
                .await
                .unwrap_err()
        ));
    }

    #[tokio::test]
    async fn test_default_cancel_run_not_supported() {
        assert!(is_not_supported(
            &MockCiProvider
                .cancel_run("owner/repo", "1")
                .await
                .unwrap_err()
        ));
    }

    #[tokio::test]
    async fn test_default_list_workflows_not_supported() {
        assert!(is_not_supported(
            &MockCiProvider
                .list_workflows("owner/repo")
                .await
                .unwrap_err()
        ));
    }
}
