//! List merge requests / pull requests via CLI.

use crate::CliProvider;
use crate::error::CliError;
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

        let mut results = Vec::with_capacity(raw.len());
        for item in raw {
            results.push(MrPr {
                number: item["number"].as_u64().unwrap_or(0),
                title: item["title"].as_str().unwrap_or("").to_string(),
                state: match item["state"].as_str().unwrap_or("") {
                    "OPEN" => MrPrState::Open,
                    "CLOSED" => MrPrState::Closed,
                    "MERGED" => MrPrState::Merged,
                    _ => MrPrState::Open,
                },
                author: item["author"]["login"].as_str().unwrap_or("").to_string(),
                source_branch: item["headRefName"].as_str().unwrap_or("").to_string(),
                target_branch: item["baseRefName"].as_str().unwrap_or("").to_string(),
                url: item["url"].as_str().unwrap_or("").to_string(),
                draft: item["isDraft"].as_bool().unwrap_or(false),
                labels: item["labels"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                reviewers: item["reviewRequests"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v["login"].as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                created_at: item["createdAt"].as_str().unwrap_or("").to_string(),
                updated_at: item["updatedAt"].as_str().unwrap_or("").to_string(),
                additions: item["additions"].as_u64(),
                deletions: item["deletions"].as_u64(),
                changed_files: item["changedFiles"].as_u64(),
            });
        }

        Ok(results)
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

        let mut results = Vec::with_capacity(raw.len());
        for item in raw {
            results.push(MrPr {
                number: item["iid"].as_u64().unwrap_or(0),
                title: item["title"].as_str().unwrap_or("").to_string(),
                state: match item["state"].as_str().unwrap_or("") {
                    "opened" => MrPrState::Open,
                    "closed" => MrPrState::Closed,
                    "merged" => MrPrState::Merged,
                    _ => MrPrState::Open,
                },
                author: item["author"]["username"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                source_branch: item["source_branch"].as_str().unwrap_or("").to_string(),
                target_branch: item["target_branch"].as_str().unwrap_or("").to_string(),
                url: item["web_url"].as_str().unwrap_or("").to_string(),
                draft: item["draft"].as_bool().unwrap_or(false)
                    || item["work_in_progress"].as_bool().unwrap_or(false),
                labels: item["labels"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                reviewers: item["reviewers"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v["username"].as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                created_at: item["created_at"].as_str().unwrap_or("").to_string(),
                updated_at: item["updated_at"].as_str().unwrap_or("").to_string(),
                additions: None, // GitLab list doesn't include stats
                deletions: None,
                changed_files: None,
            });
        }

        Ok(results)
    }
}
