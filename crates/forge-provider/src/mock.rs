//! A minimal [`ForgeProvider`] implementation for tests.
//!
//! Only implements the required identity + MR/PR methods by returning empty
//! data / `NotSupported`, so it exercises the trait's method shape. Useful
//! both for this crate's own tests (validates that the trait can be object-
//! safe-ed into `Arc<dyn ForgeProvider>`) and for downstream crates that
//! need a stand-in in unit tests.

use crate::{
    CreateMrPrInput, EditMrPrPatch, ForgeAuthStatus, ForgeError, ForgeKind, ForgeProvider,
    MergeStrategy, MrPr, MrPrDetail, MrPrDiffFile, MrPrFilter,
};

/// A stub provider that reports as authenticated and returns empty data.
pub struct MockProvider {
    kind: ForgeKind,
}

impl MockProvider {
    /// Create a mock for the given forge kind.
    pub fn new(kind: ForgeKind) -> Self {
        Self { kind }
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
    ) -> Result<(), ForgeError> {
        Ok(())
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
}
