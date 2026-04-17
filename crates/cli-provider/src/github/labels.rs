//! Label management for GitHub MRs/PRs.

use forge_provider::{ForgeError, Label};

use super::GitHubCli;
use crate::parsers::parse_github_labels;

impl GitHubCli {
    pub(super) fn add_mr_pr_labels_impl(
        &self,
        number: u64,
        labels: &[String],
    ) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = labels.join(",");
        self.run(&["pr", "edit", &num_str, "--add-label", &joined])?;
        Ok(())
    }

    pub(super) fn remove_mr_pr_labels_impl(
        &self,
        number: u64,
        labels: &[String],
    ) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = labels.join(",");
        self.run(&["pr", "edit", &num_str, "--remove-label", &joined])?;
        Ok(())
    }

    pub(super) fn list_labels_impl(&self) -> Result<Vec<Label>, ForgeError> {
        let stdout = self.run(&[
            "label",
            "list",
            "--json",
            "name,color,description",
            "--limit",
            "500",
        ])?;
        parse_github_labels(&stdout).map_err(Into::into)
    }
}
