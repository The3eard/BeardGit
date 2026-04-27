//! Job-related API methods for [`GitLabClient`].

use crate::client::{ApiError, GitLabClient};
use crate::types::Job;

impl GitLabClient {
    /// Fetch detailed information for a single CI/CD job.
    pub async fn get_job(&self, project_id: u64, job_id: u64) -> Result<Job, ApiError> {
        self.get(&format!("/projects/{project_id}/jobs/{job_id}"))
            .await
    }

    /// Fetch the raw log (trace) output for a job as plain text.
    pub async fn get_job_log(&self, project_id: u64, job_id: u64) -> Result<String, ApiError> {
        self.get_text(&format!("/projects/{project_id}/jobs/{job_id}/trace"))
            .await
    }
}
