//! [`CiProvider`] implementation for GitLab.
//!
//! Wraps the existing [`GitLabClient`] HTTP methods and normalizes
//! GitLab-specific types into the unified provider types.

use async_trait::async_trait;
use provider::{
    CiFilters, CiJob, CiProvider, CiRun, CiRunDetail, CiStage, CiStatus, Project, ProviderError,
    ProviderKind, ProviderUser,
};

use crate::client::{ApiError, GitLabClient};
use crate::types;

/// GitLab implementation of the [`CiProvider`] trait.
///
/// Delegates HTTP calls to [`GitLabClient`] and converts the GitLab-specific
/// response types into unified provider types.
pub struct GitLabProvider {
    client: GitLabClient,
}

impl GitLabProvider {
    /// Create a new GitLab provider targeting `base_url` with the given PAT.
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            client: GitLabClient::new(base_url, token),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal request/response shapes for CI/CD control (Phase 8.4)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct GitLabTriggerBody {
    #[serde(rename = "ref")]
    ref_name: String,
    variables: Vec<GitLabVariable>,
}

#[derive(serde::Serialize)]
struct GitLabVariable {
    key: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct GitLabPipelineCreated {
    id: u64,
    web_url: String,
}

#[async_trait]
impl CiProvider for GitLabProvider {
    async fn validate_token(&self) -> Result<ProviderUser, ProviderError> {
        let user: types::GitLabApiUser = self
            .client
            .get("/user")
            .await
            .map_err(into_provider_error)?;
        Ok(gitlab_user_to_provider_user(user))
    }

    async fn get_project(&self, project_ref: &str) -> Result<Project, ProviderError> {
        let encoded = urlencoding::encode(project_ref);
        let project: types::Project = self
            .client
            .get(&format!("/projects/{encoded}"))
            .await
            .map_err(into_provider_error)?;
        Ok(gitlab_project_to_provider_project(project))
    }

    async fn list_ci_runs(
        &self,
        project_ref: &str,
        filters: &CiFilters,
        per_page: u32,
        page: u32,
    ) -> Result<Vec<CiRun>, ProviderError> {
        let encoded = urlencoding::encode(project_ref);
        let mut path = format!(
            "/projects/{encoded}/pipelines?per_page={per_page}&page={page}&order_by=id&sort=desc"
        );
        if let Some(ref branch) = filters.branch {
            path.push_str(&format!("&ref={}", urlencoding::encode(branch)));
        }
        if let Some(ref status) = filters.status {
            path.push_str(&format!("&status={status}"));
        }
        if let Some(ref source) = filters.source {
            path.push_str(&format!("&source={source}"));
        }

        let pipelines: Vec<types::Pipeline> =
            self.client.get(&path).await.map_err(into_provider_error)?;
        Ok(pipelines.into_iter().map(pipeline_to_ci_run).collect())
    }

    async fn get_ci_run_detail(
        &self,
        project_ref: &str,
        run_id: u64,
    ) -> Result<CiRunDetail, ProviderError> {
        let encoded = urlencoding::encode(project_ref);

        // Fetch pipeline detail
        let detail: types::PipelineDetail = self
            .client
            .get(&format!("/projects/{encoded}/pipelines/{run_id}"))
            .await
            .map_err(into_provider_error)?;

        // Fetch jobs and group by stage
        let jobs: Vec<types::Job> = self
            .client
            .get(&format!(
                "/projects/{encoded}/pipelines/{run_id}/jobs?per_page=100"
            ))
            .await
            .map_err(into_provider_error)?;

        let stages = group_jobs_by_stage(jobs);

        Ok(CiRunDetail {
            run: CiRun {
                id: detail.id,
                display_id: detail.iid,
                status: normalize_gitlab_status(&detail.status),
                ref_name: detail.ref_name,
                sha: detail.sha,
                source: None,
                name: None,
                created_at: detail.created_at.clone(),
                updated_at: None,
                web_url: detail.web_url,
            },
            duration: detail.duration.map(|d| d as f64),
            finished_at: detail.finished_at,
            stages,
        })
    }

    async fn get_job_log(&self, project_ref: &str, job_id: u64) -> Result<String, ProviderError> {
        let encoded = urlencoding::encode(project_ref);
        self.client
            .get_text(&format!("/projects/{encoded}/jobs/{job_id}/trace"))
            .await
            .map_err(into_provider_error)
    }

    async fn trigger_workflow(
        &self,
        project_ref: &str,
        input: &provider::TriggerWorkflowInput,
    ) -> Result<provider::TriggerResult, ProviderError> {
        let encoded = urlencoding::encode(project_ref);
        let variables: Vec<GitLabVariable> = input
            .inputs
            .iter()
            .map(|(k, v)| GitLabVariable {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();
        let body = GitLabTriggerBody {
            ref_name: input.git_ref.clone(),
            variables,
        };
        let created: GitLabPipelineCreated = self
            .client
            .post_json(&format!("/projects/{encoded}/pipeline"), &body)
            .await
            .map_err(into_provider_error)?;
        Ok(provider::TriggerResult {
            run_id: created.id.to_string(),
            url: created.web_url,
        })
    }

    async fn retry_run(&self, project_ref: &str, run_id: &str) -> Result<(), ProviderError> {
        let encoded = urlencoding::encode(project_ref);
        // Empty body satisfies reqwest's content-type expectations.
        self.client
            .post_no_body(
                &format!("/projects/{encoded}/pipelines/{run_id}/retry"),
                &serde_json::json!({}),
            )
            .await
            .map_err(into_provider_error)
    }

    /// GitLab does not distinguish "retry all" from "retry failed" — the
    /// `/retry` endpoint already only re-runs jobs that did not succeed.
    async fn retry_failed_jobs(
        &self,
        project_ref: &str,
        run_id: &str,
    ) -> Result<(), ProviderError> {
        self.retry_run(project_ref, run_id).await
    }

    async fn retry_job(&self, project_ref: &str, job_id: &str) -> Result<(), ProviderError> {
        let encoded = urlencoding::encode(project_ref);
        self.client
            .post_no_body(
                &format!("/projects/{encoded}/jobs/{job_id}/retry"),
                &serde_json::json!({}),
            )
            .await
            .map_err(into_provider_error)
    }

    async fn cancel_run(&self, project_ref: &str, run_id: &str) -> Result<(), ProviderError> {
        let encoded = urlencoding::encode(project_ref);
        self.client
            .post_no_body(
                &format!("/projects/{encoded}/pipelines/{run_id}/cancel"),
                &serde_json::json!({}),
            )
            .await
            .map_err(into_provider_error)
    }

    async fn list_workflows(
        &self,
        _project_ref: &str,
    ) -> Result<Vec<provider::Workflow>, ProviderError> {
        Ok(vec![provider::Workflow {
            id: "default".to_string(),
            name: "Pipeline".to_string(),
            path: ".gitlab-ci.yml".to_string(),
            state: provider::WorkflowState::Active,
        }])
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::GitLab
    }

    fn base_url(&self) -> &str {
        self.client.base_url()
    }
}

// ---------------------------------------------------------------------------
// Status normalization
// ---------------------------------------------------------------------------

/// Normalize a GitLab status string into the unified [`CiStatus`] enum.
///
/// GitLab uses a single `status` field with these known values:
/// `created`, `waiting_for_resource`, `preparing`, `pending`, `running`,
/// `success`, `failed`, `canceled`, `skipped`, `manual`.
pub fn normalize_gitlab_status(status: &str) -> CiStatus {
    match status {
        "created" | "waiting_for_resource" | "preparing" => CiStatus::Queued,
        "pending" => CiStatus::Pending,
        "running" => CiStatus::Running,
        "success" => CiStatus::Success,
        "failed" => CiStatus::Failed,
        "canceled" => CiStatus::Canceled,
        "skipped" => CiStatus::Skipped,
        "manual" => CiStatus::Manual,
        _ => CiStatus::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Type conversions
// ---------------------------------------------------------------------------

/// Convert a GitLab pipeline list item to a unified CI run.
fn pipeline_to_ci_run(p: types::Pipeline) -> CiRun {
    CiRun {
        id: p.id,
        display_id: p.iid,
        status: normalize_gitlab_status(&p.status),
        ref_name: p.ref_name,
        sha: p.sha,
        source: p.source,
        name: p.name,
        created_at: p.created_at,
        updated_at: p.updated_at,
        web_url: p.web_url,
    }
}

/// Convert a GitLab job to a unified CI job.
fn gitlab_job_to_ci_job(j: types::Job) -> CiJob {
    CiJob {
        id: j.id,
        name: j.name,
        stage: Some(j.stage),
        status: normalize_gitlab_status(&j.status),
        duration: j.duration,
        started_at: j.started_at,
        finished_at: j.finished_at,
        web_url: j.web_url,
        allow_failure: j.allow_failure,
        steps: None,
    }
}

/// Group a flat list of GitLab jobs into stages, preserving order.
fn group_jobs_by_stage(jobs: Vec<types::Job>) -> Vec<CiStage> {
    let mut stage_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut stages: Vec<CiStage> = Vec::new();
    for job in jobs {
        let stage_name = job.stage.clone();
        let ci_job = gitlab_job_to_ci_job(job);
        if let Some(&idx) = stage_map.get(&stage_name) {
            stages[idx].jobs.push(ci_job);
        } else {
            stage_map.insert(stage_name.clone(), stages.len());
            stages.push(CiStage {
                name: stage_name,
                jobs: vec![ci_job],
            });
        }
    }
    stages
}

/// Convert a GitLab user API response to a unified provider user.
fn gitlab_user_to_provider_user(u: types::GitLabApiUser) -> ProviderUser {
    ProviderUser {
        id: u.id,
        username: u.username,
        display_name: u.name,
        email: Some(u.email),
        avatar_url: u.avatar_url,
        profile_url: u.web_url,
    }
}

/// Convert a GitLab project API response to a unified project.
fn gitlab_project_to_provider_project(p: types::Project) -> Project {
    Project {
        id: p.id,
        name: p.name,
        full_path: p.path_with_namespace,
        default_branch: p.default_branch,
        web_url: p.web_url,
    }
}

/// Convert a [`GitLabClient`] [`ApiError`] into a [`ProviderError`].
fn into_provider_error(e: ApiError) -> ProviderError {
    match e {
        ApiError::Http(e) => ProviderError::Http(e.to_string()),
        ApiError::Api { status, message } => ProviderError::Api { status, message },
        ApiError::Json(e) => ProviderError::Json(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_gitlab_status_all_known() {
        assert_eq!(normalize_gitlab_status("created"), CiStatus::Queued);
        assert_eq!(
            normalize_gitlab_status("waiting_for_resource"),
            CiStatus::Queued
        );
        assert_eq!(normalize_gitlab_status("preparing"), CiStatus::Queued);
        assert_eq!(normalize_gitlab_status("pending"), CiStatus::Pending);
        assert_eq!(normalize_gitlab_status("running"), CiStatus::Running);
        assert_eq!(normalize_gitlab_status("success"), CiStatus::Success);
        assert_eq!(normalize_gitlab_status("failed"), CiStatus::Failed);
        assert_eq!(normalize_gitlab_status("canceled"), CiStatus::Canceled);
        assert_eq!(normalize_gitlab_status("skipped"), CiStatus::Skipped);
        assert_eq!(normalize_gitlab_status("manual"), CiStatus::Manual);
    }

    #[test]
    fn test_normalize_gitlab_status_unknown() {
        assert_eq!(normalize_gitlab_status("something_new"), CiStatus::Unknown);
        assert_eq!(normalize_gitlab_status(""), CiStatus::Unknown);
    }

    #[test]
    fn test_pipeline_to_ci_run() {
        let pipeline = types::Pipeline {
            id: 100,
            iid: 42,
            project_id: 1,
            status: "success".to_string(),
            ref_name: "main".to_string(),
            sha: "abc123".to_string(),
            source: Some("push".to_string()),
            name: Some("Build Pipeline".to_string()),
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T00:05:00Z".to_string()),
            web_url: "https://gitlab.com/p/100".to_string(),
        };
        let run = pipeline_to_ci_run(pipeline);
        assert_eq!(run.id, 100);
        assert_eq!(run.display_id, 42);
        assert_eq!(run.status, CiStatus::Success);
        assert_eq!(run.ref_name, "main");
        assert_eq!(run.source, Some("push".to_string()));
        assert_eq!(run.name, Some("Build Pipeline".to_string()));
    }

    #[test]
    fn test_gitlab_job_to_ci_job() {
        let job = types::Job {
            id: 200,
            name: "test-unit".to_string(),
            stage: "test".to_string(),
            status: "failed".to_string(),
            duration: Some(45.5),
            created_at: None,
            started_at: Some("2026-01-01T00:01:00Z".to_string()),
            finished_at: Some("2026-01-01T00:01:45Z".to_string()),
            web_url: "https://gitlab.com/j/200".to_string(),
            pipeline: None,
            allow_failure: Some(true),
        };
        let ci_job = gitlab_job_to_ci_job(job);
        assert_eq!(ci_job.id, 200);
        assert_eq!(ci_job.name, "test-unit");
        assert_eq!(ci_job.stage, Some("test".to_string()));
        assert_eq!(ci_job.status, CiStatus::Failed);
        assert_eq!(ci_job.duration, Some(45.5));
        assert_eq!(ci_job.allow_failure, Some(true));
    }

    #[test]
    fn test_group_jobs_by_stage() {
        let jobs = vec![
            types::Job {
                id: 1,
                name: "build".to_string(),
                stage: "build".to_string(),
                status: "success".to_string(),
                duration: Some(10.0),
                created_at: None,
                started_at: None,
                finished_at: None,
                web_url: "".to_string(),
                pipeline: None,
                allow_failure: None,
            },
            types::Job {
                id: 2,
                name: "lint".to_string(),
                stage: "test".to_string(),
                status: "success".to_string(),
                duration: Some(5.0),
                created_at: None,
                started_at: None,
                finished_at: None,
                web_url: "".to_string(),
                pipeline: None,
                allow_failure: None,
            },
            types::Job {
                id: 3,
                name: "unit".to_string(),
                stage: "test".to_string(),
                status: "failed".to_string(),
                duration: Some(20.0),
                created_at: None,
                started_at: None,
                finished_at: None,
                web_url: "".to_string(),
                pipeline: None,
                allow_failure: None,
            },
        ];
        let stages = group_jobs_by_stage(jobs);
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].name, "build");
        assert_eq!(stages[0].jobs.len(), 1);
        assert_eq!(stages[1].name, "test");
        assert_eq!(stages[1].jobs.len(), 2);
    }

    #[test]
    fn test_gitlab_user_to_provider_user() {
        let user = types::GitLabApiUser {
            id: 1,
            username: "john".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            avatar_url: Some("https://gitlab.com/avatar.png".to_string()),
            web_url: "https://gitlab.com/john".to_string(),
        };
        let pu = gitlab_user_to_provider_user(user);
        assert_eq!(pu.username, "john");
        assert_eq!(pu.display_name, "John Doe");
        assert_eq!(pu.email, Some("john@example.com".to_string()));
        assert_eq!(pu.profile_url, "https://gitlab.com/john");
    }
}
