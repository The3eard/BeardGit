//! Reviewer management for GitLab MRs.
//!
//! `glab mr update --reviewer` replaces the full list rather than appending,
//! so add/remove fetch the current reviewers and compute the new set locally.

use forge_provider::ForgeError;

use super::GitLabCli;

impl GitLabCli {
    pub(super) fn add_mr_pr_reviewers_impl(
        &self,
        number: u64,
        reviewers: &[String],
    ) -> Result<(), ForgeError> {
        if reviewers.is_empty() {
            return Ok(());
        }
        // `glab mr update --reviewer` replaces the full reviewer list rather
        // than appending. Fetch current reviewers and merge manually.
        let detail = self.get_mr_pr_impl(number)?;
        let mut set: std::collections::BTreeSet<String> =
            detail.summary.reviewers.into_iter().collect();
        for r in reviewers {
            set.insert(r.clone());
        }
        let joined = set.into_iter().collect::<Vec<_>>().join(",");
        let num_str = number.to_string();
        self.run(&["mr", "update", &num_str, "--reviewer", &joined])?;
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
        let detail = self.get_mr_pr_impl(number)?;
        let to_remove: std::collections::HashSet<&str> =
            reviewers.iter().map(|s| s.as_str()).collect();
        let remaining: Vec<String> = detail
            .summary
            .reviewers
            .into_iter()
            .filter(|r| !to_remove.contains(r.as_str()))
            .collect();
        let joined = remaining.join(",");
        let num_str = number.to_string();
        // Empty string clears the reviewer list in glab.
        self.run(&["mr", "update", &num_str, "--reviewer", &joined])?;
        Ok(())
    }
}
