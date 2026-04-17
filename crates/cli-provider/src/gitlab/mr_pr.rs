//! MR/PR CRUD + comments for the GitLab CLI provider.
//!
//! Covers list / get / diff / create / edit / merge / close / approve /
//! request-changes / add-comment / add-inline-comment.

use forge_provider::{
    Comment, CreateMrPrInput, EditMrPrPatch, ForgeError, MergeStrategy, MrPr, MrPrDetail,
    MrPrDiffFile, MrPrFilter, ReviewStatus,
};

use super::{GitLabCli, state_to_glab_str};
use crate::parsers::{GITLAB_FIELDS, parse_gitlab_comment, parse_mr_pr};

impl GitLabCli {
    pub(super) fn list_mr_prs_impl(
        &self,
        filter: MrPrFilter,
        limit: u32,
    ) -> Result<Vec<MrPr>, ForgeError> {
        let state_str = filter.state.map(state_to_glab_str);
        let per_page = limit.to_string();
        let mut args = vec!["mr", "list", "--per-page", &per_page];
        if let Some(s) = state_str {
            args.extend(["--state", s]);
        }
        args.extend(["-F", "json"]);
        let raw: Vec<serde_json::Value> = self.run_json(&args)?;
        Ok(raw.iter().map(|i| parse_mr_pr(i, &GITLAB_FIELDS)).collect())
    }

    pub(super) fn get_mr_pr_impl(&self, number: u64) -> Result<MrPrDetail, ForgeError> {
        let num_str = number.to_string();
        let raw: serde_json::Value = self.run_json(&["mr", "view", &num_str, "-F", "json"])?;
        let summary = parse_mr_pr(&raw, &GITLAB_FIELDS);

        // Fetch discussions separately to get discussion IDs for resolve
        // support. Each discussion groups one or more notes under a shared
        // `id`, which is the identifier the resolve/unresolve API takes.
        let discussions_path = format!("projects/:id/merge_requests/{number}/discussions");
        let discussions: Vec<serde_json::Value> = self
            .run_json(&["api", &discussions_path])
            .unwrap_or_default();

        let mut comments: Vec<Comment> = Vec::new();
        for disc in &discussions {
            let disc_id = disc["id"].as_str().map(|s| s.to_string());
            if let Some(notes) = disc["notes"].as_array() {
                for note in notes {
                    let mut c = parse_gitlab_comment(note);
                    c.discussion_id = disc_id.clone();
                    comments.push(c);
                }
            }
        }

        // Fallback: if the discussions endpoint failed or returned nothing,
        // fall back to the `notes` field from the MR view — discussion_id
        // stays `None`, resolve buttons will not render but comments still do.
        if comments.is_empty()
            && let Some(arr) = raw["notes"].as_array()
        {
            comments = arr.iter().map(parse_gitlab_comment).collect();
        }

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

    pub(super) fn get_mr_pr_diff_impl(&self, number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError> {
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
                additions: 0,
                deletions: 0,
                patch: f["diff"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    pub(super) fn create_mr_pr_impl(&self, input: CreateMrPrInput) -> Result<MrPr, ForgeError> {
        let mut args = vec![
            "mr",
            "create",
            "--source-branch",
            &input.source,
            "--target-branch",
            &input.target,
            "--title",
            &input.title,
            "--description",
            &input.body,
            "--no-editor",
        ];
        if input.draft {
            args.push("--draft");
        }
        let labels_str = input.labels.join(",");
        if !input.labels.is_empty() {
            args.extend(["--label", &labels_str]);
        }
        let reviewer_refs: Vec<&str> = input.reviewers.iter().map(|r| r.as_str()).collect();
        for r in &reviewer_refs {
            args.extend(["--reviewer", r]);
        }
        let output = self.run(&args)?;
        let number = output
            .lines()
            .find_map(|line| line.rsplit('/').next().and_then(|s| s.parse::<u64>().ok()))
            .ok_or_else(|| ForgeError::Cli("could not parse MR number".into()))?;
        self.get_mr_pr_impl(number).map(|d| d.summary)
    }

    pub(super) fn edit_mr_pr_impl(
        &self,
        number: u64,
        patch: EditMrPrPatch,
    ) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        let mut args = vec!["mr", "update", &num_str];
        if let Some(t) = &patch.title {
            args.extend(["--title", t.as_str()]);
        }
        if let Some(b) = &patch.body {
            args.extend(["--description", b.as_str()]);
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
        let mut args = vec!["mr", "merge", &num_str];
        if strategy == MergeStrategy::Squash {
            args.push("--squash");
        }
        // glab has no --rebase flag; rebase is configured on the MR itself.
        self.run(&args)?;
        Ok(())
    }

    pub(super) fn close_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "close", &num_str])?;
        Ok(())
    }

    pub(super) fn approve_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "approve", &num_str])?;
        Ok(())
    }

    pub(super) fn request_changes_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        // GitLab has no "request changes" concept — post body as a comment.
        self.add_mr_pr_comment_impl(number, body)
    }

    pub(super) fn add_mr_pr_comment_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "note", &num_str, "--message", body])?;
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
            "position": {
                "position_type": "text",
                "new_path": path,
                "new_line": line,
            }
        });
        let api_path = format!("projects/:id/merge_requests/{number}/discussions");
        self.run_with_stdin(
            &["api", &api_path, "--method", "POST", "--input", "-"],
            &json_body.to_string(),
        )?;
        Ok(())
    }
}
