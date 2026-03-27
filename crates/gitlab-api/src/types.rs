//! Data types returned by the GitLab REST API v4, mirrored on the Svelte frontend.

use serde::{Deserialize, Serialize};

/// Summary of a GitLab pipeline as returned by the list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Global pipeline ID.
    pub id: u64,
    /// Project-scoped pipeline IID.
    pub iid: u64,
    /// ID of the project that owns this pipeline.
    pub project_id: u64,
    /// Current pipeline status (e.g. `"success"`, `"failed"`, `"running"`).
    pub status: String,
    /// Branch or tag name the pipeline ran against.
    #[serde(rename = "ref")]
    pub ref_name: String,
    /// Commit SHA that triggered the pipeline.
    pub sha: String,
    /// Pipeline source (e.g. `"push"`, `"web"`, `"schedule"`).
    pub source: Option<String>,
    /// Optional human-readable pipeline name.
    pub name: Option<String>,
    /// ISO 8601 creation timestamp.
    pub created_at: Option<String>,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: Option<String>,
    /// URL to the pipeline in the GitLab UI.
    pub web_url: String,
}

/// Detailed information about a single pipeline, including timing data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDetail {
    /// Global pipeline ID.
    pub id: u64,
    /// Project-scoped pipeline IID.
    pub iid: u64,
    /// Current pipeline status.
    pub status: String,
    /// Branch or tag name the pipeline ran against.
    #[serde(rename = "ref")]
    pub ref_name: String,
    /// Commit SHA that triggered the pipeline.
    pub sha: String,
    /// Total wall-clock duration in seconds (available once finished).
    pub duration: Option<u64>,
    /// ISO 8601 creation timestamp.
    pub created_at: Option<String>,
    /// ISO 8601 completion timestamp.
    pub finished_at: Option<String>,
    /// URL to the pipeline in the GitLab UI.
    pub web_url: String,
}

/// A single CI/CD job belonging to a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job ID.
    pub id: u64,
    /// Human-readable job name defined in `.gitlab-ci.yml`.
    pub name: String,
    /// Stage this job belongs to.
    pub stage: String,
    /// Current job status (e.g. `"success"`, `"failed"`, `"pending"`).
    pub status: String,
    /// Wall-clock duration in seconds (available once finished).
    pub duration: Option<f64>,
    /// ISO 8601 creation timestamp.
    pub created_at: Option<String>,
    /// ISO 8601 timestamp when the job started running.
    pub started_at: Option<String>,
    /// ISO 8601 timestamp when the job finished.
    pub finished_at: Option<String>,
    /// URL to the job in the GitLab UI.
    pub web_url: String,
    /// Slim reference to the owning pipeline (included in job detail responses).
    pub pipeline: Option<JobPipeline>,
    /// Whether this job is allowed to fail without marking the pipeline as failed.
    pub allow_failure: Option<bool>,
}

/// Slim pipeline reference embedded inside a [`Job`] response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPipeline {
    /// Pipeline ID.
    pub id: u64,
    /// Pipeline status at the time of the request.
    pub status: String,
}

/// A CI/CD stage grouping its constituent jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    /// Stage name (e.g. `"build"`, `"test"`, `"deploy"`).
    pub name: String,
    /// Jobs belonging to this stage, in the order returned by the API.
    pub jobs: Vec<Job>,
}

/// GitLab project metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Numeric project ID.
    pub id: u64,
    /// Human-readable project name.
    pub name: String,
    /// Full path including namespace (e.g. `"group/project-name"`).
    pub path_with_namespace: String,
    /// URL to the project in the GitLab UI.
    pub web_url: String,
    /// Default branch name (e.g. `"main"`).
    pub default_branch: Option<String>,
}

/// GitLab user profile as returned by `GET /api/v4/user`.
///
/// Used internally for token validation. Converted to [`provider::ProviderUser`]
/// before crossing the crate boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabApiUser {
    /// Numeric user ID.
    pub id: u64,
    /// Login username.
    pub username: String,
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: String,
    /// URL to the user's avatar image.
    pub avatar_url: Option<String>,
    /// URL to the user's profile page.
    pub web_url: String,
}
