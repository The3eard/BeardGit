//! Label management for GitLab MRs.

use forge_provider::{ForgeError, Label};

use super::GitLabCli;
use crate::parsers::parse_gitlab_labels;

impl GitLabCli {
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
        self.run(&["mr", "update", &num_str, "--label", &joined])?;
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
        self.run(&["mr", "update", &num_str, "--unlabel", &joined])?;
        Ok(())
    }

    pub(super) fn list_labels_impl(&self) -> Result<Vec<Label>, ForgeError> {
        let stdout = self.run(&["label", "list", "-F", "json"])?;
        parse_gitlab_labels(&stdout).map_err(Into::into)
    }
}
