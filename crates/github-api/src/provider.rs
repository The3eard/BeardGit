//! [`CiProvider`] implementation for GitHub.
//!
//! Normalizes GitHub Actions API responses into the unified provider types.
//! Key differences from GitLab:
//! - Status uses `status` + `conclusion` fields (normalized via [`normalize_github_status`])
//! - No stages — all jobs grouped under a single `"Jobs"` virtual stage
//! - Duration computed from timestamps (not provided by API)
//! - Job logs require following a 302 redirect (handled by reqwest)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use provider::{
    CiFilters, CiJob, CiJobStep, CiProvider, CiRun, CiRunDetail, CiStage, CiStatus, Project,
    ProviderError, ProviderKind, ProviderUser,
};

use crate::client::{ApiError, GitHubClient};
use crate::types;

/// GitHub implementation of the [`CiProvider`] trait.
///
/// Delegates HTTP calls to [`GitHubClient`] and converts the GitHub-specific
/// response types into unified provider types. GitHub Actions has no stage
/// concept, so all jobs are grouped under a single virtual stage.
pub struct GitHubProvider {
    client: GitHubClient,
}

impl GitHubProvider {
    /// Create a new GitHub provider.
    ///
    /// For github.com, use `base_url = "https://api.github.com"`.
    /// For GitHub Enterprise, use `base_url = "https://<host>/api/v3"`.
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            client: GitHubClient::new(base_url, token),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal request shapes for CI/CD control (Phase 8.4)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct GitHubDispatchBody<'a> {
    #[serde(rename = "ref")]
    ref_name: &'a str,
    inputs: &'a std::collections::HashMap<String, String>,
}

/// Normalize a GitHub workflow `state` string into [`provider::WorkflowState`].
///
/// GitHub documents these variants: `active`, `deleted`, `disabled_fork`,
/// `disabled_inactivity`, `disabled_manually`. Everything except `active`
/// maps to [`provider::WorkflowState::Disabled`].
fn normalize_workflow_state(state: &str) -> provider::WorkflowState {
    match state {
        "active" => provider::WorkflowState::Active,
        _ => provider::WorkflowState::Disabled,
    }
}

#[async_trait]
impl CiProvider for GitHubProvider {
    async fn validate_token(&self) -> Result<ProviderUser, ProviderError> {
        let user: types::GitHubUser = self
            .client
            .get("/user")
            .await
            .map_err(into_provider_error)?;
        Ok(github_user_to_provider_user(user))
    }

    async fn get_project(&self, project_ref: &str) -> Result<Project, ProviderError> {
        let repo: types::GitHubRepo = self
            .client
            .get(&format!("/repos/{project_ref}"))
            .await
            .map_err(into_provider_error)?;
        Ok(github_repo_to_project(repo))
    }

    async fn list_ci_runs(
        &self,
        project_ref: &str,
        filters: &CiFilters,
        per_page: u32,
        page: u32,
    ) -> Result<Vec<CiRun>, ProviderError> {
        let mut path = format!("/repos/{project_ref}/actions/runs?per_page={per_page}&page={page}");
        if let Some(ref branch) = filters.branch {
            path.push_str(&format!("&branch={branch}"));
        }
        if let Some(ref status) = filters.status {
            path.push_str(&format!("&status={status}"));
        }
        if let Some(ref event) = filters.source {
            path.push_str(&format!("&event={event}"));
        }

        let resp: types::WorkflowRunsResponse =
            self.client.get(&path).await.map_err(into_provider_error)?;
        Ok(resp
            .workflow_runs
            .into_iter()
            .map(workflow_run_to_ci_run)
            .collect())
    }

    async fn get_ci_run_detail(
        &self,
        project_ref: &str,
        run_id: u64,
    ) -> Result<CiRunDetail, ProviderError> {
        // Fetch run detail
        let run: types::WorkflowRun = self
            .client
            .get(&format!("/repos/{project_ref}/actions/runs/{run_id}"))
            .await
            .map_err(into_provider_error)?;

        // Fetch jobs
        let jobs_resp: types::WorkflowJobsResponse = self
            .client
            .get(&format!(
                "/repos/{project_ref}/actions/runs/{run_id}/jobs?per_page=100"
            ))
            .await
            .map_err(into_provider_error)?;

        let ci_jobs: Vec<CiJob> = jobs_resp
            .jobs
            .into_iter()
            .map(workflow_job_to_ci_job)
            .collect();

        // GitHub has no stages — group all jobs under a single virtual stage
        let stages = vec![CiStage {
            name: "Jobs".to_string(),
            jobs: ci_jobs,
        }];

        // Compute duration from timestamps
        let updated_at_opt = Some(run.updated_at.clone());
        let duration = compute_duration(&run.run_started_at, &updated_at_opt);

        // Determine finished_at: only set when run is completed
        let finished_at = if run.status == "completed" {
            Some(run.updated_at.clone())
        } else {
            None
        };

        Ok(CiRunDetail {
            run: workflow_run_to_ci_run(run),
            duration,
            finished_at,
            stages,
        })
    }

    async fn get_job_log(&self, project_ref: &str, job_id: u64) -> Result<String, ProviderError> {
        // GitHub returns a 302 redirect — reqwest follows it automatically
        self.client
            .get_text(&format!("/repos/{project_ref}/actions/jobs/{job_id}/logs"))
            .await
            .map_err(into_provider_error)
    }

    async fn trigger_workflow(
        &self,
        project_ref: &str,
        input: &provider::TriggerWorkflowInput,
    ) -> Result<provider::TriggerResult, ProviderError> {
        let path = format!(
            "/repos/{project_ref}/actions/workflows/{}/dispatches",
            input.workflow_id
        );
        let body = GitHubDispatchBody {
            ref_name: &input.git_ref,
            inputs: &input.inputs,
        };
        self.client
            .post_no_body(&path, &body)
            .await
            .map_err(into_provider_error)?;
        // GitHub's dispatch endpoint does not return the new run ID. The
        // frontend discovers it on the next polling cycle.
        Ok(provider::TriggerResult {
            run_id: String::new(),
            url: format!("https://github.com/{project_ref}/actions"),
        })
    }

    async fn retry_run(&self, project_ref: &str, run_id: &str) -> Result<(), ProviderError> {
        self.client
            .post_no_body(
                &format!("/repos/{project_ref}/actions/runs/{run_id}/rerun"),
                &serde_json::json!({}),
            )
            .await
            .map_err(into_provider_error)
    }

    async fn retry_failed_jobs(
        &self,
        project_ref: &str,
        run_id: &str,
    ) -> Result<(), ProviderError> {
        self.client
            .post_no_body(
                &format!("/repos/{project_ref}/actions/runs/{run_id}/rerun-failed-jobs"),
                &serde_json::json!({}),
            )
            .await
            .map_err(into_provider_error)
    }

    async fn retry_job(&self, project_ref: &str, job_id: &str) -> Result<(), ProviderError> {
        self.client
            .post_no_body(
                &format!("/repos/{project_ref}/actions/jobs/{job_id}/rerun"),
                &serde_json::json!({}),
            )
            .await
            .map_err(into_provider_error)
    }

    async fn cancel_run(&self, project_ref: &str, run_id: &str) -> Result<(), ProviderError> {
        self.client
            .post_no_body(
                &format!("/repos/{project_ref}/actions/runs/{run_id}/cancel"),
                &serde_json::json!({}),
            )
            .await
            .map_err(into_provider_error)
    }

    async fn list_workflows(
        &self,
        project_ref: &str,
    ) -> Result<Vec<provider::Workflow>, ProviderError> {
        let resp: types::WorkflowsResponse = self
            .client
            .get(&format!("/repos/{project_ref}/actions/workflows"))
            .await
            .map_err(into_provider_error)?;
        Ok(resp
            .workflows
            .into_iter()
            .map(|w| provider::Workflow {
                id: w.id.to_string(),
                name: w.name,
                path: w.path,
                state: normalize_workflow_state(&w.state),
            })
            .collect())
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::GitHub
    }

    fn base_url(&self) -> &str {
        self.client.base_url()
    }
}

// ---------------------------------------------------------------------------
// Status normalization
// ---------------------------------------------------------------------------

/// Normalize GitHub's split status model into the unified [`CiStatus`] enum.
///
/// GitHub uses two fields:
/// - `status`: lifecycle state (`"queued"`, `"in_progress"`, `"completed"`)
/// - `conclusion`: result (only set when `status == "completed"`)
///
/// This function combines them into a single [`CiStatus`] value.
pub fn normalize_github_status(status: &str, conclusion: Option<&str>) -> CiStatus {
    match (status, conclusion) {
        ("queued", _) => CiStatus::Queued,
        ("pending", _) => CiStatus::Pending,
        ("in_progress", _) => CiStatus::Running,
        ("completed", Some("success")) => CiStatus::Success,
        ("completed", Some("failure")) => CiStatus::Failed,
        ("completed", Some("cancelled")) => CiStatus::Canceled,
        ("completed", Some("skipped")) => CiStatus::Skipped,
        ("completed", Some("timed_out")) => CiStatus::TimedOut,
        _ => CiStatus::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Type conversions
// ---------------------------------------------------------------------------

/// Convert a GitHub workflow run to a unified CI run.
fn workflow_run_to_ci_run(r: types::WorkflowRun) -> CiRun {
    CiRun {
        id: r.id,
        display_id: r.run_number,
        status: normalize_github_status(&r.status, r.conclusion.as_deref()),
        ref_name: r.head_branch.unwrap_or_default(),
        sha: r.head_sha,
        source: Some(r.event),
        name: r.name,
        created_at: Some(r.created_at),
        updated_at: Some(r.updated_at),
        web_url: r.html_url,
    }
}

/// Convert a GitHub workflow job to a unified CI job.
fn workflow_job_to_ci_job(j: types::WorkflowJob) -> CiJob {
    let duration = compute_duration(&j.started_at, &j.completed_at);
    let steps: Vec<CiJobStep> = j
        .steps
        .into_iter()
        .map(|s| {
            let step_duration = compute_duration(&s.started_at, &s.completed_at);
            CiJobStep {
                number: s.number,
                name: s.name,
                status: normalize_github_status(&s.status, s.conclusion.as_deref()),
                duration: step_duration,
            }
        })
        .collect();
    CiJob {
        id: j.id,
        name: j.name,
        stage: None, // GitHub has no stages
        status: normalize_github_status(&j.status, j.conclusion.as_deref()),
        duration,
        started_at: j.started_at,
        finished_at: j.completed_at,
        web_url: j.html_url,
        allow_failure: None,
        steps: if steps.is_empty() { None } else { Some(steps) },
    }
}

/// Convert a GitHub user to a unified provider user.
fn github_user_to_provider_user(u: types::GitHubUser) -> ProviderUser {
    ProviderUser {
        id: u.id,
        username: u.login,
        display_name: u.name.unwrap_or_default(),
        email: u.email,
        avatar_url: u.avatar_url,
        profile_url: u.html_url,
    }
}

/// Convert a GitHub repo to a unified project.
fn github_repo_to_project(r: types::GitHubRepo) -> Project {
    Project {
        id: r.id,
        name: r.name,
        full_path: r.full_name,
        default_branch: r.default_branch,
        web_url: r.html_url,
    }
}

/// Compute duration in seconds between two ISO 8601 timestamps.
///
/// Returns `None` if either timestamp is missing or cannot be parsed.
fn compute_duration(start: &Option<String>, end: &Option<String>) -> Option<f64> {
    let start = start.as_ref()?;
    let end = end.as_ref()?;
    let start_dt = start.parse::<DateTime<Utc>>().ok()?;
    let end_dt = end.parse::<DateTime<Utc>>().ok()?;
    let duration = end_dt.signed_duration_since(start_dt);
    if duration.num_seconds() >= 0 {
        Some(duration.num_milliseconds() as f64 / 1000.0)
    } else {
        None
    }
}

/// Convert a [`GitHubClient`] [`ApiError`] into a [`ProviderError`].
fn into_provider_error(e: ApiError) -> ProviderError {
    match e {
        ApiError::Http(e) => ProviderError::Http(e.to_string()),
        ApiError::Api { status, message } => ProviderError::Api { status, message },
        ApiError::Json(e) => ProviderError::Json(e.to_string()),
        ApiError::RateLimited { retry_after_secs } => {
            ProviderError::RateLimited { retry_after_secs }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_github_status_all_known() {
        assert_eq!(normalize_github_status("queued", None), CiStatus::Queued);
        assert_eq!(normalize_github_status("pending", None), CiStatus::Pending);
        assert_eq!(
            normalize_github_status("in_progress", None),
            CiStatus::Running
        );
        assert_eq!(
            normalize_github_status("completed", Some("success")),
            CiStatus::Success
        );
        assert_eq!(
            normalize_github_status("completed", Some("failure")),
            CiStatus::Failed
        );
        assert_eq!(
            normalize_github_status("completed", Some("cancelled")),
            CiStatus::Canceled
        );
        assert_eq!(
            normalize_github_status("completed", Some("skipped")),
            CiStatus::Skipped
        );
        assert_eq!(
            normalize_github_status("completed", Some("timed_out")),
            CiStatus::TimedOut
        );
    }

    #[test]
    fn test_normalize_github_status_unknown() {
        assert_eq!(
            normalize_github_status("completed", Some("stale")),
            CiStatus::Unknown
        );
        assert_eq!(normalize_github_status("waiting", None), CiStatus::Unknown);
        assert_eq!(
            normalize_github_status("completed", None),
            CiStatus::Unknown
        );
    }

    #[test]
    fn test_compute_duration_valid() {
        let start = Some("2026-01-01T00:00:00Z".to_string());
        let end = Some("2026-01-01T00:05:30Z".to_string());
        assert_eq!(compute_duration(&start, &end), Some(330.0));
    }

    #[test]
    fn test_compute_duration_missing_start() {
        let end = Some("2026-01-01T00:05:30Z".to_string());
        assert_eq!(compute_duration(&None, &end), None);
    }

    #[test]
    fn test_compute_duration_missing_end() {
        let start = Some("2026-01-01T00:00:00Z".to_string());
        assert_eq!(compute_duration(&start, &None), None);
    }

    #[test]
    fn test_workflow_run_to_ci_run() {
        let run = types::WorkflowRun {
            id: 500,
            run_number: 42,
            name: Some("CI".to_string()),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            head_branch: Some("main".to_string()),
            head_sha: "abc123".to_string(),
            event: "push".to_string(),
            html_url: "https://github.com/owner/repo/actions/runs/500".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:05:00Z".to_string(),
            run_started_at: Some("2026-01-01T00:00:10Z".to_string()),
        };
        let ci_run = workflow_run_to_ci_run(run);
        assert_eq!(ci_run.id, 500);
        assert_eq!(ci_run.display_id, 42);
        assert_eq!(ci_run.status, CiStatus::Success);
        assert_eq!(ci_run.ref_name, "main");
        assert_eq!(ci_run.source, Some("push".to_string()));
        assert_eq!(ci_run.name, Some("CI".to_string()));
    }

    #[test]
    fn test_workflow_job_to_ci_job() {
        let job = types::WorkflowJob {
            id: 600,
            name: "build".to_string(),
            status: "completed".to_string(),
            conclusion: Some("failure".to_string()),
            started_at: Some("2026-01-01T00:01:00Z".to_string()),
            completed_at: Some("2026-01-01T00:02:30Z".to_string()),
            html_url: "https://github.com/o/r/actions/runs/1/jobs/600".to_string(),
            steps: vec![
                types::WorkflowJobStep {
                    number: 1,
                    name: "Set up job".to_string(),
                    status: "completed".to_string(),
                    conclusion: Some("success".to_string()),
                    started_at: Some("2026-01-01T00:01:00Z".to_string()),
                    completed_at: Some("2026-01-01T00:01:05Z".to_string()),
                },
                types::WorkflowJobStep {
                    number: 2,
                    name: "Build".to_string(),
                    status: "in_progress".to_string(),
                    conclusion: None,
                    started_at: Some("2026-01-01T00:01:05Z".to_string()),
                    completed_at: None,
                },
            ],
        };
        let ci_job = workflow_job_to_ci_job(job);
        assert_eq!(ci_job.id, 600);
        assert_eq!(ci_job.name, "build");
        assert!(ci_job.stage.is_none()); // GitHub has no stages
        assert_eq!(ci_job.status, CiStatus::Failed);
        assert_eq!(ci_job.duration, Some(90.0));
        // Steps parsed correctly
        let steps = ci_job.steps.unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].name, "Set up job");
        assert_eq!(steps[0].status, CiStatus::Success);
        assert_eq!(steps[0].duration, Some(5.0));
        assert_eq!(steps[1].name, "Build");
        assert_eq!(steps[1].status, CiStatus::Running);
        assert!(steps[1].duration.is_none());
    }

    #[test]
    fn test_github_user_private_email() {
        let user = types::GitHubUser {
            id: 1,
            login: "octocat".to_string(),
            name: Some("Mona Lisa".to_string()),
            email: None, // private email
            avatar_url: Some("https://github.com/avatar.png".to_string()),
            html_url: "https://github.com/octocat".to_string(),
        };
        let pu = github_user_to_provider_user(user);
        assert_eq!(pu.username, "octocat");
        assert_eq!(pu.email, None);
    }
}
