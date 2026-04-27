//! Reviewer management for GitHub PRs.

use forge_provider::ForgeError;

use super::GitHubCli;

impl GitHubCli {
    pub(super) fn add_mr_pr_reviewers_impl(
        &self,
        number: u64,
        reviewers: &[String],
    ) -> Result<(), ForgeError> {
        if reviewers.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = reviewers.join(",");
        self.run(&["pr", "edit", &num_str, "--add-reviewer", &joined])?;
        Ok(())
    }

    pub(super) fn remove_mr_pr_reviewers_impl(
        &self,
        number: u64,
        reviewers: &[String],
    ) -> Result<(), ForgeError> {
        if reviewers.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = reviewers.join(",");
        self.run(&["pr", "edit", &num_str, "--remove-reviewer", &joined])?;
        Ok(())
    }
}
