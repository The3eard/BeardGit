//! GitHub REST API response types.
//!
//! These types mirror the JSON returned by the GitHub Actions API.
//! They are internal to this crate — only the unified provider types
//! are exposed to the rest of the application.

use serde::Deserialize;

/// Wrapper for the `GET /repos/{owner}/{repo}/actions/runs` response.
#[derive(Debug, Deserialize)]
pub struct WorkflowRunsResponse {
    /// Total number of matching workflow runs.
    pub total_count: u64,
    /// Array of workflow runs for the current page.
    pub workflow_runs: Vec<WorkflowRun>,
}

/// A single GitHub Actions workflow run.
#[derive(Debug, Deserialize)]
pub struct WorkflowRun {
    /// Unique run ID.
    pub id: u64,
    /// Human-friendly run number (incrementing per workflow).
    pub run_number: u64,
    /// Workflow name (e.g. `"CI"`, `"Deploy"`).
    pub name: Option<String>,
    /// Lifecycle status: `"queued"`, `"in_progress"`, `"completed"`.
    pub status: String,
    /// Result when completed: `"success"`, `"failure"`, `"cancelled"`, etc.
    pub conclusion: Option<String>,
    /// Branch the run executed against.
    pub head_branch: Option<String>,
    /// Commit SHA that triggered the run.
    pub head_sha: String,
    /// Event that triggered the run (e.g. `"push"`, `"pull_request"`).
    pub event: String,
    /// URL to the run in the GitHub UI.
    pub html_url: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
    /// ISO 8601 timestamp when the run actually started executing.
    pub run_started_at: Option<String>,
}

/// Wrapper for the `GET /repos/{owner}/{repo}/actions/runs/{id}/jobs` response.
#[derive(Debug, Deserialize)]
pub struct WorkflowJobsResponse {
    /// Total number of jobs in this run.
    pub total_count: u64,
    /// Array of jobs.
    pub jobs: Vec<WorkflowJob>,
}

/// A single GitHub Actions workflow job.
#[derive(Debug, Deserialize)]
pub struct WorkflowJob {
    /// Unique job ID.
    pub id: u64,
    /// Human-readable job name.
    pub name: String,
    /// Lifecycle status: `"queued"`, `"in_progress"`, `"completed"`.
    pub status: String,
    /// Result when completed.
    pub conclusion: Option<String>,
    /// ISO 8601 timestamp when the job started executing.
    pub started_at: Option<String>,
    /// ISO 8601 timestamp when the job completed.
    pub completed_at: Option<String>,
    /// URL to the job in the GitHub UI.
    pub html_url: String,
    /// Individual steps within this job.
    #[serde(default)]
    pub steps: Vec<WorkflowJobStep>,
}

/// A single step within a GitHub Actions workflow job.
#[derive(Debug, Deserialize)]
pub struct WorkflowJobStep {
    /// Step number (1-based).
    pub number: u32,
    /// Human-readable step name.
    pub name: String,
    /// Lifecycle status: `"queued"`, `"in_progress"`, `"completed"`.
    pub status: String,
    /// Result when completed.
    pub conclusion: Option<String>,
    /// ISO 8601 timestamp when the step started.
    pub started_at: Option<String>,
    /// ISO 8601 timestamp when the step completed.
    pub completed_at: Option<String>,
}

/// GitHub repository metadata as returned by `GET /repos/{owner}/{repo}`.
#[derive(Debug, Deserialize)]
pub struct GitHubRepo {
    /// Numeric repository ID.
    pub id: u64,
    /// Repository name (without owner).
    pub name: String,
    /// Full name including owner: `"owner/repo"`.
    pub full_name: String,
    /// Default branch name.
    pub default_branch: Option<String>,
    /// URL to the repo in the GitHub UI.
    pub html_url: String,
}

/// GitHub user profile as returned by `GET /user`.
#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    /// Numeric user ID.
    pub id: u64,
    /// Login username.
    pub login: String,
    /// Display name (may be null).
    pub name: Option<String>,
    /// Email (null if private).
    pub email: Option<String>,
    /// Avatar URL.
    pub avatar_url: Option<String>,
    /// URL to the user's profile page.
    pub html_url: String,
}
