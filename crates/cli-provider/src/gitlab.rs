//! GitLab CLI provider — implements [`ForgeProvider`] by invoking `glab`.

use std::path::PathBuf;

use forge_provider::{
    Comment, CreateMrPrInput, EditMrPrPatch, ForgeAuthStatus, ForgeError, ForgeKind, ForgeProvider,
    MergeStrategy, MrPr, MrPrDetail, MrPrDiffFile, MrPrFilter, ReviewStatus,
};

use crate::auth;
use crate::parsers::{GITLAB_FIELDS, parse_gitlab_comment, parse_mr_pr};
use crate::runner;

/// CLI-backed [`ForgeProvider`] for GitLab (using the bundled `glab` binary).
pub struct GitLabCli {
    /// Absolute path to the `glab` binary.
    pub binary_path: PathBuf,
    /// Working directory — the repository root. `glab` auto-detects the remote.
    pub repo_path: PathBuf,
}

impl GitLabCli {
    /// Create a new GitLab CLI provider.
    pub fn new(binary_path: impl Into<PathBuf>, repo_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
            repo_path: repo_path.into(),
        }
    }

    fn run(&self, args: &[&str]) -> Result<String, ForgeError> {
        runner::run(&self.binary_path, &self.repo_path, args).map_err(Into::into)
    }

    fn run_json<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> Result<T, ForgeError> {
        runner::run_json(&self.binary_path, &self.repo_path, args).map_err(Into::into)
    }

    fn run_with_stdin(&self, args: &[&str], stdin_data: &str) -> Result<String, ForgeError> {
        runner::run_with_stdin(&self.binary_path, &self.repo_path, args, stdin_data)
            .map_err(Into::into)
    }
}

impl ForgeProvider for GitLabCli {
    fn kind(&self) -> ForgeKind {
        ForgeKind::GitLab
    }

    fn auth_status(&self) -> ForgeAuthStatus {
        let status = auth::check_glab_auth_status(&self.binary_path);
        if status.authenticated {
            ForgeAuthStatus::Authenticated {
                username: status.username,
            }
        } else if status.error.is_some() {
            ForgeAuthStatus::Unknown
        } else {
            ForgeAuthStatus::NotAuthenticated
        }
    }

    fn list_mr_prs(&self, filter: MrPrFilter, limit: u32) -> Result<Vec<MrPr>, ForgeError> {
        let state_str = filter.state.map(state_to_glab_str);
        let per_page = limit.to_string();
        let mut args = vec!["mr", "list", "--per-page", &per_page];
        if let Some(s) = state_str {
            args.extend(["--state", s]);
        }
        args.extend(["-F", "json"]);
        let stdout = self.run(&args)?;
        let raw: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).map_err(|e| ForgeError::Cli(format!("json: {e}")))?;
        Ok(raw.iter().map(|i| parse_mr_pr(i, &GITLAB_FIELDS)).collect())
    }

    fn get_mr_pr(&self, number: u64) -> Result<MrPrDetail, ForgeError> {
        let num_str = number.to_string();
        let raw: serde_json::Value = self.run_json(&["mr", "view", &num_str, "-F", "json"])?;
        let summary = parse_mr_pr(&raw, &GITLAB_FIELDS);
        let comments: Vec<Comment> = raw["notes"]
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

    fn get_mr_pr_diff(&self, number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError> {
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

    fn create_mr_pr(&self, input: CreateMrPrInput) -> Result<MrPr, ForgeError> {
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
        self.get_mr_pr(number).map(|d| d.summary)
    }

    fn edit_mr_pr(&self, number: u64, patch: EditMrPrPatch) -> Result<(), ForgeError> {
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

    fn merge_mr_pr(&self, number: u64, strategy: MergeStrategy) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        let mut args = vec!["mr", "merge", &num_str];
        if strategy == MergeStrategy::Squash {
            args.push("--squash");
        }
        // glab has no --rebase flag; rebase is configured on the MR itself.
        self.run(&args)?;
        Ok(())
    }

    fn close_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "close", &num_str])?;
        Ok(())
    }

    fn approve_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "approve", &num_str])?;
        Ok(())
    }

    fn request_changes(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        // GitLab has no "request changes" concept — post body as a comment.
        self.add_mr_pr_comment(number, body)
    }

    fn add_mr_pr_comment(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "note", &num_str, "--message", body])?;
        Ok(())
    }

    fn add_mr_pr_inline_comment(
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

/// Map [`MrPrState`][forge_provider::MrPrState] to the string `glab` expects.
fn state_to_glab_str(s: forge_provider::MrPrState) -> &'static str {
    match s {
        forge_provider::MrPrState::Open => "opened",
        forge_provider::MrPrState::Closed => "closed",
        forge_provider::MrPrState::Merged => "merged",
    }
}
