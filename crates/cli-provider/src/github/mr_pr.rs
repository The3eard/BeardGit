//! MR/PR CRUD + comments for the GitHub CLI provider.
//!
//! Covers list / get / diff / create / edit / merge / close / approve /
//! request-changes / add-comment / add-inline-comment.

use forge_provider::{
    Comment, CreateMrPrInput, EditMrPrPatch, ForgeError, MergeStrategy, MrPr, MrPrDetail,
    MrPrDiffFile, MrPrFilter, ReviewStatus,
};

use super::{GitHubCli, state_to_gh_str};
use crate::parsers::{GITHUB_FIELDS, parse_github_comment, parse_mr_pr};

impl GitHubCli {
    pub(super) fn list_mr_prs_impl(
        &self,
        filter: MrPrFilter,
        limit: u32,
    ) -> Result<Vec<MrPr>, ForgeError> {
        let state_str = filter.state.map(state_to_gh_str);
        let limit_str = limit.to_string();
        let mut args = vec![
            "pr",
            "list",
            "--json",
            "number,title,state,author,headRefName,baseRefName,url,isDraft,labels,reviewRequests,createdAt,updatedAt,additions,deletions,changedFiles",
            "--limit",
            &limit_str,
        ];
        if let Some(s) = state_str {
            args.extend(["--state", s]);
        }
        let stdout = self.run(&args)?;
        let raw: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).map_err(|e| ForgeError::Cli(format!("json: {e}")))?;
        Ok(raw.iter().map(|i| parse_mr_pr(i, &GITHUB_FIELDS)).collect())
    }

    pub(super) fn get_mr_pr_impl(&self, number: u64) -> Result<MrPrDetail, ForgeError> {
        let num_str = number.to_string();
        let raw: serde_json::Value = self.run_json(&[
            "pr", "view", &num_str,
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

        let comments: Vec<Comment> = raw["comments"]
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

    pub(super) fn get_mr_pr_diff_impl(&self, number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError> {
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

    pub(super) fn create_mr_pr_impl(&self, input: CreateMrPrInput) -> Result<MrPr, ForgeError> {
        let mut args = vec![
            "pr",
            "create",
            "--head",
            &input.source,
            "--base",
            &input.target,
            "--title",
            &input.title,
            "--body",
            &input.body,
        ];
        if input.draft {
            args.push("--draft");
        }
        let labels_str = input.labels.join(",");
        if !input.labels.is_empty() {
            args.extend(["--label", &labels_str]);
        }
        let reviewers_str = input.reviewers.join(",");
        if !input.reviewers.is_empty() {
            args.extend(["--reviewer", &reviewers_str]);
        }
        let url_output = self.run(&args)?;
        let url = url_output.trim();
        let number: u64 = url
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ForgeError::Cli("could not parse PR number from URL".into()))?;
        self.list_mr_prs_impl(MrPrFilter::default(), 1)
            .and_then(|list| {
                list.into_iter()
                    .find(|p| p.number == number)
                    .ok_or_else(|| ForgeError::Cli("created PR not found in list".into()))
            })
    }

    pub(super) fn edit_mr_pr_impl(
        &self,
        number: u64,
        patch: EditMrPrPatch,
    ) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        let mut args = vec!["pr", "edit", &num_str];
        if let Some(t) = &patch.title {
            args.extend(["--title", t.as_str()]);
        }
        if let Some(b) = &patch.body {
            args.extend(["--body", b.as_str()]);
        }
        self.run(&args)?;
        Ok(())
    }

    pub(super) fn merge_mr_pr_impl(
        &self,
        number: u64,
        strategy: MergeStrategy,
    ) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        let mut args = vec!["pr", "merge", &num_str];
        match strategy {
            MergeStrategy::Merge => args.push("--merge"),
            MergeStrategy::Squash => args.push("--squash"),
            MergeStrategy::Rebase => args.push("--rebase"),
        }
        self.run(&args)?;
        Ok(())
    }

    pub(super) fn close_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "close", &num_str])?;
        Ok(())
    }

    pub(super) fn approve_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "review", &num_str, "--approve"])?;
        Ok(())
    }

    pub(super) fn request_changes_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&[
            "pr",
            "review",
            &num_str,
            "--request-changes",
            "--body",
            body,
        ])?;
        Ok(())
    }

    pub(super) fn add_mr_pr_comment_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "comment", &num_str, "--body", body])?;
        Ok(())
    }

    pub(super) fn add_mr_pr_inline_comment_impl(
        &self,
        number: u64,
        path: &str,
        line: u64,
        body: &str,
    ) -> Result<(), ForgeError> {
        let json_body = serde_json::json!({
            "body": body,
            "path": path,
            "line": line,
            "side": "RIGHT",
        });
        let api_path = format!("repos/{{owner}}/{{repo}}/pulls/{number}/comments");
        self.run_with_stdin(
            &["api", &api_path, "--method", "POST", "--input", "-"],
            &json_body.to_string(),
        )?;
        Ok(())
    }
}
