//! MR discussion resolve/unresolve (GitLab-specific).
//!
//! GitHub has no equivalent concept — the [`ForgeProvider`] trait's default
//! `NotSupported` impl applies there.

use forge_provider::ForgeError;

use super::GitLabCli;

impl GitLabCli {
    pub(super) fn resolve_discussion_impl(
        &self,
        number: u64,
        discussion_id: &str,
    ) -> Result<(), ForgeError> {
        let path = format!("projects/:id/merge_requests/{number}/discussions/{discussion_id}");
        self.run(&["api", &path, "--method", "PUT", "-f", "resolved=true"])?;
        Ok(())
    }

    pub(super) fn unresolve_discussion_impl(
        &self,
        number: u64,
        discussion_id: &str,
    ) -> Result<(), ForgeError> {
        let path = format!("projects/:id/merge_requests/{number}/discussions/{discussion_id}");
        self.run(&["api", &path, "--method", "PUT", "-f", "resolved=false"])?;
        Ok(())
    }
}
