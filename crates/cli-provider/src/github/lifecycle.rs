//! Lifecycle transitions (ready/draft/reopen) for GitHub PRs.

use forge_provider::ForgeError;

use super::GitHubCli;

impl GitHubCli {
    pub(super) fn mark_mr_pr_ready_impl(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["pr", "ready", &n])?;
        Ok(())
    }

    pub(super) fn mark_mr_pr_draft_impl(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["pr", "ready", &n, "--undo"])?;
        Ok(())
    }

    pub(super) fn reopen_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["pr", "reopen", &n])?;
        Ok(())
    }
}
