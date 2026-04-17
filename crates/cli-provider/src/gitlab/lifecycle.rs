//! Lifecycle transitions (ready/draft/reopen) for GitLab MRs.

use forge_provider::ForgeError;

use super::GitLabCli;

impl GitLabCli {
    pub(super) fn mark_mr_pr_ready_impl(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["mr", "update", &n, "--ready"])?;
        Ok(())
    }

    pub(super) fn mark_mr_pr_draft_impl(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["mr", "update", &n, "--draft"])?;
        Ok(())
    }

    pub(super) fn reopen_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["mr", "reopen", &n])?;
        Ok(())
    }
}
