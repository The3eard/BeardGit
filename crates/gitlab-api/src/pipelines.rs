//! Pipeline-related API methods for [`GitLabClient`].

use crate::client::{ApiError, GitLabClient};
use crate::types::{Job, Pipeline, PipelineDetail, Stage};

/// Jobs requested per page. GitLab's documented maximum.
const JOBS_PER_PAGE: u32 = 100;

/// Safety bound on job pagination, so a misbehaving endpoint cannot spin
/// forever. Reaching it is logged, never silent.
const MAX_JOB_PAGES: u32 = 20;

impl GitLabClient {
    /// List pipelines for a project with optional server-side filters and pagination.
    ///
    /// All filter parameters map directly to GitLab REST API query parameters.
    /// Results are ordered by pipeline ID descending (newest first).
    /// `ref_name` filters by branch/tag, `scope` by lifecycle state (e.g. `"finished"`),
    /// `source` by trigger origin (e.g. `"push"`), and `status` by outcome (e.g. `"failed"`).
    #[allow(clippy::too_many_arguments)]
    pub async fn list_pipelines(
        &self,
        project_id: u64,
        ref_name: Option<&str>,
        scope: Option<&str>,
        source: Option<&str>,
        status: Option<&str>,
        per_page: u32,
        page: u32,
    ) -> Result<Vec<Pipeline>, ApiError> {
        let mut path = format!(
            "/projects/{project_id}/pipelines?per_page={per_page}&page={page}&order_by=id&sort=desc"
        );
        if let Some(r) = ref_name {
            path.push_str(&format!("&ref={}", urlencoding::encode(r)));
        }
        if let Some(s) = scope {
            path.push_str(&format!("&scope={}", urlencoding::encode(s)));
        }
        if let Some(s) = source {
            path.push_str(&format!("&source={}", urlencoding::encode(s)));
        }
        if let Some(s) = status {
            path.push_str(&format!("&status={}", urlencoding::encode(s)));
        }
        self.get(&path).await
    }

    /// Fetch detailed information for a single pipeline, including duration and finish time.
    pub async fn get_pipeline(
        &self,
        project_id: u64,
        pipeline_id: u64,
    ) -> Result<PipelineDetail, ApiError> {
        self.get(&format!("/projects/{project_id}/pipelines/{pipeline_id}"))
            .await
    }

    /// List **all** jobs belonging to a pipeline, following pagination.
    ///
    /// Takes the project as an already-formatted path segment so both callers
    /// can use it: `provider.rs` has a URL-encoded `group/project`, this
    /// module's other callers have a numeric id.
    ///
    /// This used to be a single `?per_page=100` with no paging, so a pipeline
    /// with more than 100 jobs — a wide CI matrix — showed the first 100 and
    /// said nothing. GitLab returns a bare array with no total, so the end is
    /// a short page.
    pub async fn list_pipeline_jobs_by_ref(
        &self,
        project: &str,
        pipeline_id: u64,
    ) -> Result<Vec<Job>, ApiError> {
        let mut all: Vec<Job> = Vec::new();

        for page in 1..=MAX_JOB_PAGES {
            let batch: Vec<Job> = self
                .get(&format!(
                    "/projects/{project}/pipelines/{pipeline_id}/jobs?per_page={JOBS_PER_PAGE}&page={page}"
                ))
                .await?;
            let short_page = batch.len() < JOBS_PER_PAGE as usize;
            all.extend(batch);
            if short_page {
                return Ok(all);
            }
        }

        // Hit the safety bound. Say so rather than returning a silently
        // truncated list — `MAX_JOB_PAGES * JOBS_PER_PAGE` jobs in one
        // pipeline is not something we expect to see.
        tracing::warn!(
            pipeline_id,
            fetched = all.len(),
            "stopped paginating pipeline jobs at the page cap — the list may be incomplete"
        );
        Ok(all)
    }

    /// List all jobs belonging to a pipeline, by numeric project id.
    pub async fn list_pipeline_jobs(
        &self,
        project_id: u64,
        pipeline_id: u64,
    ) -> Result<Vec<Job>, ApiError> {
        self.list_pipeline_jobs_by_ref(&project_id.to_string(), pipeline_id)
            .await
    }

    /// Fetch all jobs for a pipeline and group them by stage, preserving order.
    pub async fn get_pipeline_stages(
        &self,
        project_id: u64,
        pipeline_id: u64,
    ) -> Result<Vec<Stage>, ApiError> {
        let jobs = self.list_pipeline_jobs(project_id, pipeline_id).await?;
        let mut stage_map: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut stages: Vec<Stage> = Vec::new();

        for job in jobs {
            if let Some(&idx) = stage_map.get(&job.stage) {
                stages[idx].jobs.push(job);
            } else {
                stage_map.insert(job.stage.clone(), stages.len());
                stages.push(Stage {
                    name: job.stage.clone(),
                    jobs: vec![job],
                });
            }
        }

        Ok(stages)
    }
}
