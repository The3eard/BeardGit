//! Fetch detailed MR/PR information including comments via CLI.

use crate::CliProvider;
use crate::error::CliError;
use crate::parsers::{
    GITHUB_FIELDS, GITLAB_FIELDS, parse_github_comment, parse_gitlab_comment, parse_mr_pr,
};
use crate::types::{MrPrDetail, MrPrDiffFile, ReviewStatus};
use provider::ProviderKind;

impl CliProvider {
    /// Fetch detailed information about a single MR/PR.
    pub fn get_mr_pr_detail_impl(&self, number: u64) -> Result<MrPrDetail, CliError> {
        match self.kind {
            ProviderKind::GitHub => self.get_github_pr_detail(number),
            ProviderKind::GitLab => self.get_gitlab_mr_detail(number),
        }
    }

    /// Fetch GitHub PR detail via `gh pr view --json`.
    fn get_github_pr_detail(&self, number: u64) -> Result<MrPrDetail, CliError> {
        let num_str = number.to_string();
        let raw: serde_json::Value = self.run_json(&[
            "pr",
            "view",
            &num_str,
            "--json",
            "number,title,state,author,headRefName,baseRefName,url,isDraft,labels,reviewRequests,createdAt,updatedAt,body,mergeable,reviewDecision,additions,deletions,changedFiles,comments",
        ])?;

        let summary = parse_mr_pr(&raw, &GITHUB_FIELDS);

        let review_status = match raw["reviewDecision"].as_str().unwrap_or("") {
            "APPROVED" => ReviewStatus::Approved,
            "CHANGES_REQUESTED" => ReviewStatus::ChangesRequested,
            "REVIEW_REQUIRED" => ReviewStatus::Pending,
            _ => ReviewStatus::Pending,
        };

        let comments = raw["comments"]
            .as_array()
            .map(|arr| arr.iter().map(parse_github_comment).collect())
            .unwrap_or_default();

        let mergeable = match raw["mergeable"].as_str().unwrap_or("") {
            "MERGEABLE" => Some(true),
            "CONFLICTING" => Some(false),
            _ => None,
        };

        Ok(MrPrDetail {
            summary,
            body: raw["body"].as_str().unwrap_or("").to_string(),
            comments,
            review_status,
            mergeable,
        })
    }

    /// Fetch GitLab MR detail via `glab mr view -F json`.
    fn get_gitlab_mr_detail(&self, number: u64) -> Result<MrPrDetail, CliError> {
        let num_str = number.to_string();
        let raw: serde_json::Value = self.run_json(&["mr", "view", &num_str, "-F", "json"])?;

        let summary = parse_mr_pr(&raw, &GITLAB_FIELDS);

        let comments = raw["notes"]
            .as_array()
            .map(|arr| arr.iter().map(parse_gitlab_comment).collect())
            .unwrap_or_default();

        let merge_status = raw["merge_status"].as_str().unwrap_or("");
        let mergeable = match merge_status {
            "can_be_merged" => Some(true),
            "cannot_be_merged" => Some(false),
            _ => None,
        };

        Ok(MrPrDetail {
            summary,
            body: raw["description"].as_str().unwrap_or("").to_string(),
            comments,
            review_status: ReviewStatus::Pending,
            mergeable,
        })
    }

    /// Get the list of changed files in a MR/PR diff.
    pub fn get_mr_pr_diff_impl(&self, number: u64) -> Result<Vec<MrPrDiffFile>, CliError> {
        match self.kind {
            ProviderKind::GitHub => self.get_github_pr_diff(number),
            ProviderKind::GitLab => self.get_gitlab_mr_diff(number),
        }
    }

    /// Fetch GitHub PR file list via `gh api`.
    fn get_github_pr_diff(&self, number: u64) -> Result<Vec<MrPrDiffFile>, CliError> {
        let files: Vec<serde_json::Value> = self.run_json(&[
            "api",
            &format!("repos/{{owner}}/{{repo}}/pulls/{number}/files"),
            "--paginate",
        ])?;

        Ok(files
            .iter()
            .map(|f| MrPrDiffFile {
                path: f["filename"].as_str().unwrap_or("").to_string(),
                old_path: f["previous_filename"].as_str().map(|s| s.to_string()),
                status: f["status"].as_str().unwrap_or("modified").to_string(),
                additions: f["additions"].as_u64().unwrap_or(0),
                deletions: f["deletions"].as_u64().unwrap_or(0),
                patch: f["patch"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    /// Fetch GitLab MR diff file list via `glab api`.
    fn get_gitlab_mr_diff(&self, number: u64) -> Result<Vec<MrPrDiffFile>, CliError> {
        let raw: Vec<serde_json::Value> = self.run_json(&[
            "api",
            &format!("projects/:id/merge_requests/{number}/diffs"),
        ])?;

        Ok(raw
            .iter()
            .map(|f| MrPrDiffFile {
                path: f["new_path"].as_str().unwrap_or("").to_string(),
                old_path: Some(f["old_path"].as_str().unwrap_or("").to_string()),
                status: if f["new_file"].as_bool().unwrap_or(false) {
                    "added".to_string()
                } else if f["deleted_file"].as_bool().unwrap_or(false) {
                    "deleted".to_string()
                } else if f["renamed_file"].as_bool().unwrap_or(false) {
                    "renamed".to_string()
                } else {
                    "modified".to_string()
                },
                additions: 0, // GitLab diff endpoint doesn't include line counts
                deletions: 0,
                patch: f["diff"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }
}
