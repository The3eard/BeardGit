//! [`CiProvider`] trait — the contract implemented by every CI backend.
//!
//! Each provider (GitLab, GitHub) implements this trait to normalize their
//! API responses into the shared types defined in [`crate::types`]. The
//! application holds a `Box<dyn CiProvider>` and never interacts with
//! provider-specific types directly.

use crate::error::ProviderError;
use crate::kind::ProviderKind;
use crate::types::{
    CiFilters, CiRun, CiRunDetail, Project, ProviderUser, TriggerResult, TriggerWorkflowInput,
    Workflow,
};

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
        _input: &TriggerWorkflowInput,
    ) -> Result<TriggerResult, ProviderError> {
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
    async fn list_workflows(&self, _project_ref: &str) -> Result<Vec<Workflow>, ProviderError> {
        Err(ProviderError::NotSupported)
    }

    /// Returns which provider this instance represents.
    fn provider_kind(&self) -> ProviderKind;

    /// Returns the base URL of the provider instance (e.g. `"https://gitlab.com"`).
    fn base_url(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

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
