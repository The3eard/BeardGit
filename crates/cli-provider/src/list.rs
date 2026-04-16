//! List merge requests / pull requests via CLI.

use crate::CliProvider;
use crate::error::CliError;
use crate::parsers::{GITHUB_FIELDS, GITLAB_FIELDS, parse_mr_pr};
use crate::types::{MrPr, MrPrState};
use provider::ProviderKind;

impl CliProvider {
    /// List MR/PRs for the current repository.
    ///
    /// Filters by state. Returns newest-first.
    pub fn list_mr_prs_impl(
        &self,
        state_filter: Option<MrPrState>,
        limit: u32,
    ) -> Result<Vec<MrPr>, CliError> {
        let state_str = state_filter.map(|s| match s {
            MrPrState::Open => "open",
            MrPrState::Closed => "closed",
            MrPrState::Merged => "merged",
        });

        match self.kind {
            ProviderKind::GitHub => self.list_github_prs(state_str, limit),
            ProviderKind::GitLab => self.list_gitlab_mrs(state_str, limit),
        }
    }

    /// List GitHub pull requests via `gh pr list --json`.
    fn list_github_prs(&self, state: Option<&str>, limit: u32) -> Result<Vec<MrPr>, CliError> {
        let limit_str = limit.to_string();
        let mut args = vec![
            "pr",
            "list",
            "--json",
            "number,title,state,author,headRefName,baseRefName,url,isDraft,labels,reviewRequests,createdAt,updatedAt,additions,deletions,changedFiles",
            "--limit",
            &limit_str,
        ];
        let state_owned;
        if let Some(s) = state {
            state_owned = s.to_string();
            args.extend(["--state", &state_owned]);
        }

        let stdout = self.run(&args)?;
        let raw: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).map_err(|e| CliError::JsonError(e.to_string()))?;

        Ok(raw
            .iter()
            .map(|item| parse_mr_pr(item, &GITHUB_FIELDS))
            .collect())
    }

    /// List GitLab merge requests via `glab mr list -F json`.
    fn list_gitlab_mrs(&self, state: Option<&str>, limit: u32) -> Result<Vec<MrPr>, CliError> {
        let per_page = limit.to_string();
        let mut args = vec!["mr", "list", "--per-page", &per_page];
        let state_owned;
        if let Some(s) = state {
            state_owned = s.to_string();
            args.extend(["--state", &state_owned]);
        }
        // glab mr list outputs a table by default; use -F json for JSON
        args.extend(["-F", "json"]);

        let stdout = self.run(&args)?;
        let raw: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).map_err(|e| CliError::JsonError(e.to_string()))?;

        Ok(raw
            .iter()
            .map(|item| parse_mr_pr(item, &GITLAB_FIELDS))
            .collect())
    }
}
