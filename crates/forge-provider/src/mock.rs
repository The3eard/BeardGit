//! A minimal [`ForgeProvider`] implementation for tests.
//!
//! Only implements the required identity + MR/PR methods by returning empty
//! data / `NotSupported`, so it exercises the trait's method shape. Useful
//! both for this crate's own tests (validates that the trait can be object-
//! safe-ed into `Arc<dyn ForgeProvider>`) and for downstream crates that
//! need a stand-in in unit tests.

use crate::{
    CreateMrPrInput, CreateRepoInput, EditMrPrPatch, ForgeAuthStatus, ForgeError, ForgeKind,
    ForgeProvider, MergeStrategy, MrPr, MrPrDetail, MrPrDiffFile, MrPrFilter, RepoCreated,
};

/// A stub provider that reports as authenticated and returns empty data.
pub struct MockProvider {
    kind: ForgeKind,
    /// Optional one-shot override for the next [`MockProvider::create_repo`]
    /// call. Tests use [`MockProvider::set_create_repo_error`] /
    /// [`MockProvider::set_create_repo_response`] to inject a specific
    /// outcome; the override is consumed on the first read.
    create_repo_override: std::sync::Mutex<Option<Result<RepoCreated, ForgeError>>>,
}

impl MockProvider {
    /// Create a mock for the given forge kind.
    pub fn new(kind: ForgeKind) -> Self {
        Self {
            kind,
            create_repo_override: std::sync::Mutex::new(None),
        }
    }

    /// Inject an error result for the next `create_repo` call.
    ///
    /// One-shot: the override is consumed by the next call, after which
    /// `create_repo` reverts to returning the default fixture.
    pub fn set_create_repo_error(&self, err: ForgeError) {
        *self.create_repo_override.lock().unwrap() = Some(Err(err));
    }

    /// Inject a custom success result for the next `create_repo` call.
    ///
    /// One-shot: the override is consumed by the next call, after which
    /// `create_repo` reverts to returning the default fixture.
    pub fn set_create_repo_response(&self, resp: RepoCreated) {
        *self.create_repo_override.lock().unwrap() = Some(Ok(resp));
    }
}

impl ForgeProvider for MockProvider {
    fn kind(&self) -> ForgeKind {
        self.kind
    }

    fn auth_status(&self) -> ForgeAuthStatus {
        ForgeAuthStatus::Authenticated {
            username: Some("mock".to_string()),
        }
    }

    fn list_mr_prs(&self, _filter: MrPrFilter, _limit: u32) -> Result<Vec<MrPr>, ForgeError> {
        Ok(vec![])
    }

    fn get_mr_pr(&self, _number: u64) -> Result<MrPrDetail, ForgeError> {
        Err(ForgeError::NotFound("mock".into()))
    }

    fn get_mr_pr_diff(&self, _number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError> {
        Ok(vec![])
    }

    fn create_mr_pr(&self, _input: CreateMrPrInput) -> Result<MrPr, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    fn edit_mr_pr(&self, _number: u64, _patch: EditMrPrPatch) -> Result<(), ForgeError> {
        Ok(())
    }

    fn merge_mr_pr(&self, _number: u64, _strategy: MergeStrategy) -> Result<(), ForgeError> {
        Ok(())
    }

    fn close_mr_pr(&self, _number: u64) -> Result<(), ForgeError> {
        Ok(())
    }

    fn approve_mr_pr(&self, _number: u64) -> Result<(), ForgeError> {
        Ok(())
    }

    fn request_changes(&self, _number: u64, _body: &str) -> Result<(), ForgeError> {
        Ok(())
    }

    fn add_mr_pr_comment(&self, _number: u64, _body: &str) -> Result<(), ForgeError> {
        Ok(())
    }

    fn add_mr_pr_inline_comment(
        &self,
        _number: u64,
        _path: &str,
        _line: u64,
        _body: &str,
        _base_sha: &str,
        _head_sha: &str,
    ) -> Result<(), ForgeError> {
        Ok(())
    }

    fn create_repo(&self, input: CreateRepoInput) -> Result<RepoCreated, ForgeError> {
        if let Some(over) = self.create_repo_override.lock().unwrap().take() {
            return over;
        }
        Ok(RepoCreated {
            clone_url: format!("https://example.test/mock/{}.git", input.name),
            web_url: format!("https://example.test/mock/{}", input.name),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn mock_is_object_safe_as_dyn_trait() {
        let mock: Arc<dyn ForgeProvider> = Arc::new(MockProvider::new(ForgeKind::GitHub));
        assert_eq!(mock.kind(), ForgeKind::GitHub);
    }

    #[test]
    fn default_methods_return_not_supported() {
        let mock = MockProvider::new(ForgeKind::GitLab);
        assert!(matches!(
            mock.add_mr_pr_labels(1, &["bug".into()]),
            Err(ForgeError::NotSupported)
        ));
        assert!(matches!(
            mock.mark_mr_pr_ready(1),
            Err(ForgeError::NotSupported)
        ));
        assert!(matches!(
            mock.reopen_mr_pr(1),
            Err(ForgeError::NotSupported)
        ));
        assert!(matches!(
            mock.resolve_discussion(1, "abc"),
            Err(ForgeError::NotSupported)
        ));
        assert!(matches!(
            mock.unresolve_discussion(1, "abc"),
            Err(ForgeError::NotSupported)
        ));
        assert!(matches!(
            mock.checkout_mr_pr(1),
            Err(ForgeError::NotSupported)
        ));
        assert!(matches!(mock.list_labels(), Err(ForgeError::NotSupported)));
    }

    #[test]
    fn mock_auth_status_is_authenticated() {
        let mock = MockProvider::new(ForgeKind::GitHub);
        match mock.auth_status() {
            ForgeAuthStatus::Authenticated { username } => {
                assert_eq!(username.as_deref(), Some("mock"))
            }
            _ => panic!("expected authenticated"),
        }
    }

    // ─── Phase 8.5 — release trait surface ─────────────────────────────

    #[test]
    fn mock_list_releases_returns_not_supported() {
        let p = MockProvider::new(ForgeKind::GitHub);
        assert!(matches!(p.list_releases(30), Err(ForgeError::NotSupported)));
    }

    #[test]
    fn mock_get_release_returns_not_supported() {
        let p = MockProvider::new(ForgeKind::GitHub);
        assert!(matches!(
            p.get_release("v1.0.0"),
            Err(ForgeError::NotSupported)
        ));
    }

    #[test]
    fn mock_publish_release_returns_not_supported() {
        let p = MockProvider::new(ForgeKind::GitHub);
        assert!(matches!(
            p.publish_release("v1.0.0"),
            Err(ForgeError::NotSupported)
        ));
    }

    #[test]
    fn mock_upload_release_asset_returns_not_supported() {
        let p = MockProvider::new(ForgeKind::GitHub);
        let path = std::path::Path::new("/tmp/asset.bin");
        assert!(matches!(
            p.upload_release_asset("v1.0.0", path, None),
            Err(ForgeError::NotSupported)
        ));
    }
}

#[cfg(test)]
mod create_repo_tests {
    use super::*;
    use crate::{CreateRepoInput, ForgeError, ForgeProvider, RepoCreated};

    #[test]
    fn mock_create_repo_returns_fixture_by_default() {
        let mock = MockProvider::new(ForgeKind::GitHub);
        let out = mock
            .create_repo(CreateRepoInput {
                name: "hello".into(),
                private: true,
            })
            .unwrap();
        assert!(out.clone_url.contains("hello"));
        assert!(out.web_url.contains("hello"));
    }

    #[test]
    fn mock_create_repo_can_be_set_to_error() {
        let mock = MockProvider::new(ForgeKind::GitHub);
        mock.set_create_repo_error(ForgeError::NameTaken);
        let err = mock
            .create_repo(CreateRepoInput {
                name: "taken".into(),
                private: false,
            })
            .unwrap_err();
        assert!(matches!(err, ForgeError::NameTaken));
    }

    #[test]
    fn mock_create_repo_response_override_returns_custom_data() {
        let mock = MockProvider::new(ForgeKind::GitHub);
        mock.set_create_repo_response(RepoCreated {
            clone_url: "https://my.test/custom.git".into(),
            web_url: "https://my.test/custom".into(),
        });
        let out = mock
            .create_repo(CreateRepoInput {
                name: "anything".into(),
                private: false,
            })
            .unwrap();
        assert_eq!(out.clone_url, "https://my.test/custom.git");
        assert_eq!(out.web_url, "https://my.test/custom");
    }
}
