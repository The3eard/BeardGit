//! Minimal [`CiProvider`] implementation for integration tests.
//!
//! Gated behind the `mock` feature so production builds never pull the stub
//! in. Returns canned empty lists / stub users so downstream consumers can
//! exercise their IPC wiring without a real GitLab/GitHub account.

use async_trait::async_trait;

use crate::{
    CiFilters, CiProvider, CiRun, CiRunDetail, Project, ProviderError, ProviderKind, ProviderUser,
};

/// A stub [`CiProvider`] that reports as authenticated and returns empty data.
///
/// Useful as a stand-in when wiring frontend UI or `app-core` commands in
/// tests where an actual HTTP provider can't run.
pub struct MockCiProvider {
    kind: ProviderKind,
    base_url: String,
}

impl MockCiProvider {
    /// Create a mock for the given provider kind with a default base URL.
    pub fn new(kind: ProviderKind) -> Self {
        let base_url = match kind {
            ProviderKind::GitHub => "https://github.com".to_string(),
            ProviderKind::GitLab => "https://gitlab.com".to_string(),
        };
        Self { kind, base_url }
    }

    /// Create a mock with a custom base URL (e.g. self-hosted instance).
    pub fn with_base_url(kind: ProviderKind, base_url: impl Into<String>) -> Self {
        Self {
            kind,
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl CiProvider for MockCiProvider {
    async fn validate_token(&self) -> Result<ProviderUser, ProviderError> {
        Ok(ProviderUser {
            id: 1,
            username: "mock".to_string(),
            display_name: "Mock User".to_string(),
            email: Some("mock@example.test".to_string()),
            avatar_url: None,
            profile_url: format!("{}/mock", self.base_url),
        })
    }

    async fn get_project(&self, project_ref: &str) -> Result<Project, ProviderError> {
        Ok(Project {
            id: 1,
            name: project_ref
                .rsplit('/')
                .next()
                .unwrap_or(project_ref)
                .to_string(),
            full_path: project_ref.to_string(),
            default_branch: Some("main".to_string()),
            web_url: format!("{}/{project_ref}", self.base_url),
        })
    }

    async fn list_ci_runs(
        &self,
        _project_ref: &str,
        _filters: &CiFilters,
        _per_page: u32,
        _page: u32,
    ) -> Result<Vec<CiRun>, ProviderError> {
        Ok(Vec::new())
    }

    async fn get_ci_run_detail(
        &self,
        _project_ref: &str,
        _run_id: u64,
    ) -> Result<CiRunDetail, ProviderError> {
        Err(ProviderError::NotSupported)
    }

    async fn get_job_log(&self, _project_ref: &str, _job_id: u64) -> Result<String, ProviderError> {
        Ok(String::new())
    }

    fn provider_kind(&self) -> ProviderKind {
        self.kind
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_validates_token() {
        let m = MockCiProvider::new(ProviderKind::GitHub);
        let user = m.validate_token().await.unwrap();
        assert_eq!(user.username, "mock");
    }

    #[tokio::test]
    async fn mock_lists_empty_runs() {
        let m = MockCiProvider::new(ProviderKind::GitLab);
        let runs = m
            .list_ci_runs("group/project", &CiFilters::default(), 20, 1)
            .await
            .unwrap();
        assert!(runs.is_empty());
    }

    #[tokio::test]
    async fn mock_with_custom_base_url() {
        let m = MockCiProvider::with_base_url(ProviderKind::GitLab, "https://gl.example.test");
        assert_eq!(m.base_url(), "https://gl.example.test");
    }

    #[tokio::test]
    async fn mock_triggers_are_not_supported() {
        use crate::TriggerWorkflowInput;

        let m = MockCiProvider::new(ProviderKind::GitHub);
        let err = m
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
        assert!(matches!(err, ProviderError::NotSupported));
    }
}
