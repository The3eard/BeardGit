//! Review operations: approve, request changes, comment (general + inline).

use crate::CliProvider;
use crate::error::CliError;
use provider::ProviderKind;

impl CliProvider {
    /// Approve a MR/PR.
    ///
    /// - GitHub: `gh pr review <number> --approve`
    /// - GitLab: `glab mr approve <number>`
    pub fn approve_mr_pr(&self, number: u64) -> Result<(), CliError> {
        let num_str = number.to_string();
        match self.kind {
            ProviderKind::GitHub => {
                self.run(&["pr", "review", &num_str, "--approve"])?;
                Ok(())
            }
            ProviderKind::GitLab => {
                self.run(&["mr", "approve", &num_str])?;
                Ok(())
            }
        }
    }

    /// Request changes on a MR/PR.
    ///
    /// GitHub: submits a "request changes" review with the given body.
    /// GitLab: posts a comment (GitLab has no direct "request changes" concept).
    pub fn request_changes(&self, number: u64, body: &str) -> Result<(), CliError> {
        let num_str = number.to_string();
        match self.kind {
            ProviderKind::GitHub => {
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
            ProviderKind::GitLab => {
                // GitLab doesn't have a "request changes" concept; post as comment
                self.add_comment(number, body)
            }
        }
    }

    /// Add a general comment to a MR/PR.
    ///
    /// - GitHub: `gh pr comment <number> --body <body>`
    /// - GitLab: `glab mr note <number> --message <body>`
    pub fn add_comment(&self, number: u64, body: &str) -> Result<(), CliError> {
        let num_str = number.to_string();
        match self.kind {
            ProviderKind::GitHub => {
                self.run(&["pr", "comment", &num_str, "--body", body])?;
                Ok(())
            }
            ProviderKind::GitLab => {
                self.run(&["mr", "note", &num_str, "--message", body])?;
                Ok(())
            }
        }
    }

    /// Add an inline comment on a specific file and line.
    ///
    /// GitHub: uses `gh api` to create a review comment on the pull request diff.
    /// GitLab: uses `glab api` to create a diff note (discussion) on the MR.
    pub fn add_inline_comment(
        &self,
        number: u64,
        path: &str,
        line: u64,
        body: &str,
    ) -> Result<(), CliError> {
        match self.kind {
            ProviderKind::GitHub => {
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
            ProviderKind::GitLab => {
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
    }
}
