//! GitHub CLI provider — implements [`ForgeProvider`] by invoking `gh`.

use std::path::PathBuf;

use forge_provider::{
    CheckoutResult, Comment, CreateMrPrInput, EditMrPrPatch, ForgeAuthStatus, ForgeError,
    ForgeKind, ForgeProvider, Label, MergeStrategy, MrPr, MrPrDetail, MrPrDiffFile, MrPrFilter,
    ReviewStatus,
};

use crate::auth;
use crate::parsers::{GITHUB_FIELDS, parse_github_comment, parse_mr_pr};
use crate::runner;

/// CLI-backed [`ForgeProvider`] for GitHub (using the bundled `gh` binary).
pub struct GitHubCli {
    /// Absolute path to the `gh` binary.
    pub binary_path: PathBuf,
    /// Working directory — the repository root. `gh` auto-detects the remote.
    pub repo_path: PathBuf,
}

impl GitHubCli {
    /// Create a new GitHub CLI provider.
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

impl ForgeProvider for GitHubCli {
    fn kind(&self) -> ForgeKind {
        ForgeKind::GitHub
    }

    fn auth_status(&self) -> ForgeAuthStatus {
        let status = auth::check_gh_auth_status(&self.binary_path);
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

    fn get_mr_pr(&self, number: u64) -> Result<MrPrDetail, ForgeError> {
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

    fn get_mr_pr_diff(&self, number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError> {
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

    fn create_mr_pr(&self, input: CreateMrPrInput) -> Result<MrPr, ForgeError> {
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
        self.list_mr_prs(MrPrFilter::default(), 1).and_then(|list| {
            list.into_iter()
                .find(|p| p.number == number)
                .ok_or_else(|| ForgeError::Cli("created PR not found in list".into()))
        })
    }

    fn edit_mr_pr(&self, number: u64, patch: EditMrPrPatch) -> Result<(), ForgeError> {
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

    fn merge_mr_pr(&self, number: u64, strategy: MergeStrategy) -> Result<(), ForgeError> {
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

    fn close_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "close", &num_str])?;
        Ok(())
    }

    fn approve_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "review", &num_str, "--approve"])?;
        Ok(())
    }

    fn request_changes(&self, number: u64, body: &str) -> Result<(), ForgeError> {
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

    fn add_mr_pr_comment(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "comment", &num_str, "--body", body])?;
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

    // ─── Phase 8.2: MR/PR enhancements ─────────────────────────────────

    fn add_mr_pr_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = labels.join(",");
        self.run(&["pr", "edit", &num_str, "--add-label", &joined])?;
        Ok(())
    }

    fn remove_mr_pr_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = labels.join(",");
        self.run(&["pr", "edit", &num_str, "--remove-label", &joined])?;
        Ok(())
    }

    fn add_mr_pr_reviewers(&self, number: u64, reviewers: &[String]) -> Result<(), ForgeError> {
        if reviewers.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = reviewers.join(",");
        self.run(&["pr", "edit", &num_str, "--add-reviewer", &joined])?;
        Ok(())
    }

    fn remove_mr_pr_reviewers(&self, number: u64, reviewers: &[String]) -> Result<(), ForgeError> {
        if reviewers.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = reviewers.join(",");
        self.run(&["pr", "edit", &num_str, "--remove-reviewer", &joined])?;
        Ok(())
    }

    fn mark_mr_pr_ready(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["pr", "ready", &n])?;
        Ok(())
    }

    fn mark_mr_pr_draft(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["pr", "ready", &n, "--undo"])?;
        Ok(())
    }

    fn reopen_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["pr", "reopen", &n])?;
        Ok(())
    }

    fn checkout_mr_pr(&self, number: u64) -> Result<CheckoutResult, ForgeError> {
        let n = number.to_string();
        let stdout = self.run(&["pr", "checkout", &n])?;
        Ok(parse_gh_checkout_output(&stdout))
    }

    fn list_labels(&self) -> Result<Vec<Label>, ForgeError> {
        let raw: Vec<serde_json::Value> =
            self.run_json(&["label", "list", "--json", "name,color,description"])?;
        Ok(raw
            .iter()
            .map(|v| Label {
                name: v["name"].as_str().unwrap_or("").to_string(),
                color: v["color"]
                    .as_str()
                    .map(|s| s.trim_start_matches('#').to_string()),
                description: v["description"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
            })
            .collect())
    }
}

/// Parse stdout from `gh pr checkout N` into a [`CheckoutResult`].
///
/// Pure heuristic parser — looks for the branch name in git's standard
/// "Switched to ..." lines and detects fork remote additions in `gh`'s
/// `Added remote '<name>'` line.
fn parse_gh_checkout_output(stdout: &str) -> CheckoutResult {
    let mut branch_name = String::new();
    let mut remote_added: Option<String> = None;
    for line in stdout.lines() {
        let after_new = line.strip_prefix("Switched to a new branch '");
        let after_existing = line.strip_prefix("Switched to branch '");
        if let Some(rest) = after_new.or(after_existing)
            && let Some(end) = rest.find('\'')
        {
            branch_name = rest[..end].to_string();
        }
        if let Some(rest) = line.strip_prefix("Added remote '")
            && let Some(end) = rest.find('\'')
        {
            remote_added = Some(rest[..end].to_string());
        }
    }
    let is_fork = remote_added.is_some();
    CheckoutResult {
        branch_name,
        is_fork,
        remote_added,
    }
}

/// Map [`MrPrState`][forge_provider::MrPrState] to the string `gh` expects.
fn state_to_gh_str(s: forge_provider::MrPrState) -> &'static str {
    match s {
        forge_provider::MrPrState::Open => "open",
        forge_provider::MrPrState::Closed => "closed",
        forge_provider::MrPrState::Merged => "merged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gh_checkout_simple_output() {
        let out = "From github.com:foo/bar\n   abc..def  pull/42/head -> origin/pull/42/head\nSwitched to a new branch 'feature-x'\n";
        let r = parse_gh_checkout_output(out);
        assert_eq!(r.branch_name, "feature-x");
        assert!(!r.is_fork);
        assert_eq!(r.remote_added, None);
    }

    #[test]
    fn parse_gh_checkout_existing_branch() {
        let out = "Switched to branch 'existing-feature'\n";
        let r = parse_gh_checkout_output(out);
        assert_eq!(r.branch_name, "existing-feature");
        assert!(!r.is_fork);
    }

    #[test]
    fn parse_gh_checkout_fork_adds_remote() {
        let out = "Added remote 'contributor'\nFrom github.com:contributor/bar\nSwitched to a new branch 'contributor-feature'\n";
        let r = parse_gh_checkout_output(out);
        assert_eq!(r.branch_name, "contributor-feature");
        assert!(r.is_fork);
        assert_eq!(r.remote_added.as_deref(), Some("contributor"));
    }

    #[test]
    fn parse_gh_checkout_empty_stdout_yields_empty_branch() {
        let r = parse_gh_checkout_output("");
        assert_eq!(r.branch_name, "");
        assert!(!r.is_fork);
        assert!(r.remote_added.is_none());
    }
}
