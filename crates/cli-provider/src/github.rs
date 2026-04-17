//! GitHub CLI provider — implements [`ForgeProvider`] by invoking `gh`.

use std::path::PathBuf;

use forge_provider::{
    CheckoutResult, Comment, CreateIssueInput, CreateMrPrInput, CreateReleaseInput, EditIssuePatch,
    EditMrPrPatch, EditReleasePatch, ForgeAuthStatus, ForgeError, ForgeKind, ForgeProvider, Issue,
    IssueDetail, IssueFilter, IssueState, Label, MergeStrategy, Milestone, MrPr, MrPrDetail,
    MrPrDiffFile, MrPrFilter, Release, ReleaseAsset, ReleaseDetail, ReviewStatus,
};

use crate::auth;
use crate::parsers::{
    GITHUB_FIELDS, parse_github_comment, parse_github_issue_detail, parse_github_issues,
    parse_github_labels, parse_github_milestones, parse_mr_pr,
};
use crate::releases::{
    build_gh_create_args, build_gh_edit_args, build_gh_upload_args, parse_gh_release_detail,
    parse_gh_releases,
};
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

    // ─── Phase 8.3: Issues ─────────────────────────────────────────────

    fn list_issues(&self, filter: IssueFilter, limit: u32) -> Result<Vec<Issue>, ForgeError> {
        let args = build_gh_issue_list_args(&filter, limit);
        let ref_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let stdout = self.run(&ref_args)?;
        parse_github_issues(&stdout).map_err(Into::into)
    }

    fn get_issue(&self, number: u64) -> Result<IssueDetail, ForgeError> {
        let num_str = number.to_string();
        let stdout = self.run(&[
            "issue",
            "view",
            &num_str,
            "--json",
            "number,title,state,author,labels,assignees,milestone,comments,body,createdAt,updatedAt,url",
        ])?;
        parse_github_issue_detail(&stdout).map_err(Into::into)
    }

    fn create_issue(&self, input: CreateIssueInput) -> Result<Issue, ForgeError> {
        let args = build_gh_create_issue_args(&input);
        let ref_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let out = self.run(&ref_args)?;
        let url = out.trim().lines().last().unwrap_or("").trim().to_string();
        let number: u64 = url
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ForgeError::Cli(format!("could not parse issue number from `{url}`")))?;
        // Re-fetch the summary by detail view so we get a complete Issue.
        let detail = self.get_issue(number)?;
        Ok(detail.summary)
    }

    fn edit_issue(&self, number: u64, patch: EditIssuePatch) -> Result<(), ForgeError> {
        let args = build_gh_edit_issue_args(number, &patch);
        if args.len() == 3 {
            // No-op patch — avoid an unnecessary CLI call.
            return Ok(());
        }
        let ref_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.run(&ref_args)?;
        Ok(())
    }

    fn close_issue(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["issue", "close", &n])?;
        Ok(())
    }

    fn reopen_issue(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["issue", "reopen", &n])?;
        Ok(())
    }

    fn add_issue_comment(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["issue", "comment", &n, "--body", body])?;
        Ok(())
    }

    fn add_issue_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = labels.join(",");
        self.run(&["issue", "edit", &n, "--add-label", &joined])?;
        Ok(())
    }

    fn remove_issue_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = labels.join(",");
        self.run(&["issue", "edit", &n, "--remove-label", &joined])?;
        Ok(())
    }

    fn add_issue_assignees(&self, number: u64, assignees: &[String]) -> Result<(), ForgeError> {
        if assignees.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = assignees.join(",");
        self.run(&["issue", "edit", &n, "--add-assignee", &joined])?;
        Ok(())
    }

    fn remove_issue_assignees(&self, number: u64, assignees: &[String]) -> Result<(), ForgeError> {
        if assignees.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = assignees.join(",");
        self.run(&["issue", "edit", &n, "--remove-assignee", &joined])?;
        Ok(())
    }

    fn set_issue_milestone(
        &self,
        number: u64,
        milestone_id: Option<u64>,
    ) -> Result<(), ForgeError> {
        let n = number.to_string();
        match milestone_id {
            Some(id) => {
                let m = id.to_string();
                self.run(&["issue", "edit", &n, "--milestone", &m])?;
            }
            None => {
                self.run(&["issue", "edit", &n, "--remove-milestone"])?;
            }
        }
        Ok(())
    }

    fn list_milestones(&self) -> Result<Vec<Milestone>, ForgeError> {
        let stdout = self.run(&[
            "api",
            "repos/{owner}/{repo}/milestones",
            "--paginate",
            "-F",
            "state=all",
        ])?;
        parse_github_milestones(&stdout).map_err(Into::into)
    }

    // ─── Phase 8.5: Releases ───────────────────────────────────────────

    fn list_releases(&self, limit: u32) -> Result<Vec<Release>, ForgeError> {
        let limit_str = limit.to_string();
        let stdout = self.run(&[
            "release",
            "list",
            "-L",
            &limit_str,
            "--json",
            "tagName,name,isDraft,isPrerelease,publishedAt,createdAt,author,url",
        ])?;
        parse_gh_releases(&stdout).map_err(|e| ForgeError::Cli(e.to_string()))
    }

    fn get_release(&self, tag: &str) -> Result<ReleaseDetail, ForgeError> {
        let stdout = self.run(&[
            "release",
            "view",
            tag,
            "--json",
            "tagName,name,isDraft,isPrerelease,publishedAt,createdAt,author,url,body,assets",
        ])?;
        parse_gh_release_detail(&stdout).map_err(|e| ForgeError::Cli(e.to_string()))
    }

    fn list_release_assets(&self, tag: &str) -> Result<Vec<ReleaseAsset>, ForgeError> {
        Ok(self.get_release(tag)?.assets)
    }

    fn create_release(&self, input: CreateReleaseInput) -> Result<Release, ForgeError> {
        let args = build_gh_create_args(&input);
        let ref_args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&ref_args)?;
        // `gh release create` prints the release URL, not JSON. Re-fetch the
        // detail view to build a full summary.
        Ok(self.get_release(&input.tag)?.summary)
    }

    fn edit_release(&self, tag: &str, patch: EditReleasePatch) -> Result<(), ForgeError> {
        let args = build_gh_edit_args(tag, &patch);
        if args.len() == 3 {
            // No-op patch — avoid an unnecessary CLI call.
            return Ok(());
        }
        let ref_args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&ref_args)?;
        Ok(())
    }

    fn delete_release(&self, tag: &str) -> Result<(), ForgeError> {
        self.run(&["release", "delete", tag, "--yes"])?;
        Ok(())
    }

    fn publish_release(&self, tag: &str) -> Result<(), ForgeError> {
        self.run(&["release", "edit", tag, "--draft=false"])?;
        Ok(())
    }

    fn upload_release_asset(
        &self,
        tag: &str,
        path: &std::path::Path,
        label: Option<&str>,
    ) -> Result<ReleaseAsset, ForgeError> {
        let path_str = path.to_string_lossy().to_string();
        let args = build_gh_upload_args(tag, &path_str, label);
        let ref_args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&ref_args)?;
        // `gh release upload` prints no JSON — re-fetch detail and locate the
        // newly uploaded asset by file name.
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let detail = self.get_release(tag)?;
        detail
            .assets
            .into_iter()
            .find(|a| a.name == name)
            .ok_or_else(|| ForgeError::NotFound(format!("asset {name} after upload")))
    }

    fn delete_release_asset(&self, tag: &str, asset_id: u64) -> Result<(), ForgeError> {
        // `gh release delete-asset` identifies assets by name, not ID. Look up
        // the name from the detail view first.
        let detail = self.get_release(tag)?;
        let name = detail
            .assets
            .iter()
            .find(|a| a.id == asset_id)
            .map(|a| a.name.clone())
            .ok_or_else(|| ForgeError::NotFound(format!("asset id {asset_id}")))?;
        self.run(&["release", "delete-asset", tag, &name, "--yes"])?;
        Ok(())
    }
}

/// Build the argv for `gh issue list` from an [`IssueFilter`] + limit.
pub(crate) fn build_gh_issue_list_args(filter: &IssueFilter, limit: u32) -> Vec<String> {
    let fields =
        "number,title,state,author,labels,assignees,milestone,comments,createdAt,updatedAt,url";
    let mut args: Vec<String> = vec![
        "issue".into(),
        "list".into(),
        "--json".into(),
        fields.into(),
        "--limit".into(),
        limit.to_string(),
    ];
    match filter.state {
        Some(IssueState::Open) => {
            args.push("--state".into());
            args.push("open".into());
        }
        Some(IssueState::Closed) => {
            args.push("--state".into());
            args.push("closed".into());
        }
        None => {
            args.push("--state".into());
            args.push("all".into());
        }
    }
    if let Some(a) = &filter.author {
        args.push("--author".into());
        args.push(a.clone());
    }
    if let Some(a) = &filter.assignee {
        args.push("--assignee".into());
        args.push(a.clone());
    }
    if let Some(l) = &filter.label {
        args.push("--label".into());
        args.push(l.clone());
    }
    if let Some(m) = filter.milestone {
        args.push("--milestone".into());
        args.push(m.to_string());
    }
    if let Some(t) = &filter.text {
        args.push("--search".into());
        args.push(t.clone());
    }
    args
}

/// Build the argv for `gh issue create` from a [`CreateIssueInput`].
pub(crate) fn build_gh_create_issue_args(input: &CreateIssueInput) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "issue".into(),
        "create".into(),
        "--title".into(),
        input.title.clone(),
        "--body".into(),
        input.body.clone(),
    ];
    for l in &input.labels {
        args.push("--label".into());
        args.push(l.clone());
    }
    for a in &input.assignees {
        args.push("--assignee".into());
        args.push(a.clone());
    }
    if let Some(m) = input.milestone {
        args.push("--milestone".into());
        args.push(m.to_string());
    }
    args
}

/// Build the argv for `gh issue edit` from a patch. If the returned vec has
/// length 3 (just `issue edit N`), the caller should treat it as a no-op.
pub(crate) fn build_gh_edit_issue_args(number: u64, patch: &EditIssuePatch) -> Vec<String> {
    let mut args: Vec<String> = vec!["issue".into(), "edit".into(), number.to_string()];
    if let Some(t) = &patch.title {
        args.push("--title".into());
        args.push(t.clone());
    }
    if let Some(b) = &patch.body {
        args.push("--body".into());
        args.push(b.clone());
    }
    args
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

    // ─── Issue command builders (Phase 8.3) ────────────────────────────

    #[test]
    fn build_gh_issue_list_args_default_uses_all_state() {
        let f = IssueFilter::default();
        let args = build_gh_issue_list_args(&f, 50);
        assert!(args.contains(&"--state".to_string()));
        assert!(args.contains(&"all".to_string()));
    }

    #[test]
    fn build_gh_issue_list_args_with_state_and_filters() {
        let f = IssueFilter {
            state: Some(IssueState::Closed),
            author: Some("alice".into()),
            label: Some("bug".into()),
            ..Default::default()
        };
        let args = build_gh_issue_list_args(&f, 25);
        assert!(args.contains(&"closed".to_string()));
        assert!(args.contains(&"--author".to_string()));
        assert!(args.contains(&"alice".to_string()));
        assert!(args.contains(&"--label".to_string()));
        assert!(args.contains(&"bug".to_string()));
        assert!(args.contains(&"25".to_string()));
    }

    #[test]
    fn build_gh_create_issue_args_produces_expected_order() {
        let input = CreateIssueInput {
            title: "T".into(),
            body: "B".into(),
            labels: vec!["bug".into()],
            assignees: vec!["alice".into()],
            milestone: Some(5),
        };
        let args = build_gh_create_issue_args(&input);
        assert_eq!(args[0], "issue");
        assert_eq!(args[1], "create");
        assert!(args.windows(2).any(|w| w == ["--title", "T"]));
        assert!(args.windows(2).any(|w| w == ["--body", "B"]));
        assert!(args.windows(2).any(|w| w == ["--label", "bug"]));
        assert!(args.windows(2).any(|w| w == ["--assignee", "alice"]));
        assert!(args.windows(2).any(|w| w == ["--milestone", "5"]));
    }

    #[test]
    fn build_gh_create_issue_args_without_milestone_omits_flag() {
        let input = CreateIssueInput {
            title: "t".into(),
            body: "b".into(),
            labels: vec![],
            assignees: vec![],
            milestone: None,
        };
        let args = build_gh_create_issue_args(&input);
        assert!(!args.contains(&"--milestone".to_string()));
    }

    #[test]
    fn build_gh_edit_issue_args_title_only_omits_body() {
        let patch = EditIssuePatch {
            title: Some("new".into()),
            body: None,
        };
        let args = build_gh_edit_issue_args(42, &patch);
        assert!(args.windows(2).any(|w| w == ["--title", "new"]));
        assert!(!args.contains(&"--body".to_string()));
    }

    #[test]
    fn build_gh_edit_issue_args_empty_patch_is_noop_length() {
        let patch = EditIssuePatch::default();
        let args = build_gh_edit_issue_args(1, &patch);
        // ["issue", "edit", "1"] — no fields.
        assert_eq!(args.len(), 3);
    }
}
