//! GitLab CLI provider — implements [`ForgeProvider`] by invoking `glab`.

use std::path::PathBuf;

use std::collections::HashMap;

use forge_provider::{
    CheckoutResult, Comment, CreateIssueInput, CreateMrPrInput, EditIssuePatch, EditMrPrPatch,
    ForgeAuthStatus, ForgeError, ForgeKind, ForgeProvider, Issue, IssueDetail, IssueFilter,
    IssueState, Label, MergeStrategy, Milestone, MrPr, MrPrDetail, MrPrDiffFile, MrPrFilter,
    ReviewStatus,
};

use crate::auth;
use crate::parsers::{
    GITLAB_FIELDS, parse_gitlab_comment, parse_gitlab_issue_view, parse_gitlab_issues,
    parse_gitlab_labels, parse_gitlab_milestones, parse_gitlab_notes, parse_mr_pr,
};
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

    // ─── Phase 8.2: MR/PR enhancements ─────────────────────────────────

    fn add_mr_pr_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = labels.join(",");
        self.run(&["mr", "update", &num_str, "--label", &joined])?;
        Ok(())
    }

    fn remove_mr_pr_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let num_str = number.to_string();
        let joined = labels.join(",");
        self.run(&["mr", "update", &num_str, "--unlabel", &joined])?;
        Ok(())
    }

    fn add_mr_pr_reviewers(&self, number: u64, reviewers: &[String]) -> Result<(), ForgeError> {
        if reviewers.is_empty() {
            return Ok(());
        }
        // `glab mr update --reviewer` replaces the full reviewer list rather
        // than appending. Fetch current reviewers and merge manually.
        let detail = self.get_mr_pr(number)?;
        let mut set: std::collections::BTreeSet<String> =
            detail.summary.reviewers.into_iter().collect();
        for r in reviewers {
            set.insert(r.clone());
        }
        let joined = set.into_iter().collect::<Vec<_>>().join(",");
        let num_str = number.to_string();
        self.run(&["mr", "update", &num_str, "--reviewer", &joined])?;
        Ok(())
    }

    fn remove_mr_pr_reviewers(&self, number: u64, reviewers: &[String]) -> Result<(), ForgeError> {
        if reviewers.is_empty() {
            return Ok(());
        }
        let detail = self.get_mr_pr(number)?;
        let to_remove: std::collections::HashSet<&str> =
            reviewers.iter().map(|s| s.as_str()).collect();
        let remaining: Vec<String> = detail
            .summary
            .reviewers
            .into_iter()
            .filter(|r| !to_remove.contains(r.as_str()))
            .collect();
        let joined = remaining.join(",");
        let num_str = number.to_string();
        // Empty string clears the reviewer list in glab.
        self.run(&["mr", "update", &num_str, "--reviewer", &joined])?;
        Ok(())
    }

    fn mark_mr_pr_ready(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["mr", "update", &n, "--ready"])?;
        Ok(())
    }

    fn mark_mr_pr_draft(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["mr", "update", &n, "--draft"])?;
        Ok(())
    }

    fn reopen_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["mr", "reopen", &n])?;
        Ok(())
    }

    fn resolve_discussion(&self, number: u64, discussion_id: &str) -> Result<(), ForgeError> {
        let path = format!("projects/:id/merge_requests/{number}/discussions/{discussion_id}");
        self.run(&["api", &path, "--method", "PUT", "-f", "resolved=true"])?;
        Ok(())
    }

    fn unresolve_discussion(&self, number: u64, discussion_id: &str) -> Result<(), ForgeError> {
        let path = format!("projects/:id/merge_requests/{number}/discussions/{discussion_id}");
        self.run(&["api", &path, "--method", "PUT", "-f", "resolved=false"])?;
        Ok(())
    }

    fn checkout_mr_pr(&self, number: u64) -> Result<CheckoutResult, ForgeError> {
        let n = number.to_string();
        let stdout = self.run(&["mr", "checkout", &n])?;
        Ok(parse_glab_checkout_output(&stdout))
    }

    fn list_labels(&self) -> Result<Vec<Label>, ForgeError> {
        let stdout = self.run(&["label", "list", "-F", "json"])?;
        parse_gitlab_labels(&stdout).map_err(Into::into)
    }

    // ─── Phase 8.3: Issues ─────────────────────────────────────────────

    fn list_issues(&self, filter: IssueFilter, limit: u32) -> Result<Vec<Issue>, ForgeError> {
        let args = build_glab_issue_list_args(&filter, limit);
        let ref_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let stdout = self.run(&ref_args)?;
        parse_gitlab_issues(&stdout, &HashMap::new()).map_err(Into::into)
    }

    fn get_issue(&self, number: u64) -> Result<IssueDetail, ForgeError> {
        let num_str = number.to_string();
        let view = self.run(&["issue", "view", &num_str, "-F", "json"])?;
        let (summary, body) =
            parse_gitlab_issue_view(&view, &HashMap::new()).map_err(ForgeError::from)?;
        // Fetch notes via the API. If it fails (e.g. scope mismatch) we still
        // return an IssueDetail with an empty comment list.
        let notes_path = format!("projects/:id/issues/{number}/notes");
        let comments = match self.run(&["api", &notes_path, "--paginate"]) {
            Ok(json) => parse_gitlab_notes(&json).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let mut summary = summary;
        summary.comments_count = comments.len() as u64;
        Ok(IssueDetail {
            summary,
            body,
            comments,
        })
    }

    fn create_issue(&self, input: CreateIssueInput) -> Result<Issue, ForgeError> {
        let args = build_glab_create_issue_args(&input);
        let ref_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let out = self.run(&ref_args)?;
        let number: u64 = out
            .lines()
            .rev()
            .find_map(|line| line.rsplit('/').next().and_then(|s| s.parse::<u64>().ok()))
            .ok_or_else(|| {
                ForgeError::Cli("could not parse issue iid from create output".into())
            })?;
        let detail = self.get_issue(number)?;
        Ok(detail.summary)
    }

    fn edit_issue(&self, number: u64, patch: EditIssuePatch) -> Result<(), ForgeError> {
        let args = build_glab_edit_issue_args(number, &patch);
        if args.len() == 3 {
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
        self.run(&["issue", "note", &n, "--message", body])?;
        Ok(())
    }

    fn add_issue_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = labels.join(",");
        self.run(&["issue", "update", &n, "--label", &joined])?;
        Ok(())
    }

    fn remove_issue_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = labels.join(",");
        self.run(&["issue", "update", &n, "--unlabel", &joined])?;
        Ok(())
    }

    fn add_issue_assignees(&self, number: u64, assignees: &[String]) -> Result<(), ForgeError> {
        if assignees.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = assignees.join(",");
        self.run(&["issue", "update", &n, "--assignee", &joined])?;
        Ok(())
    }

    fn remove_issue_assignees(&self, number: u64, assignees: &[String]) -> Result<(), ForgeError> {
        if assignees.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = assignees.join(",");
        self.run(&["issue", "update", &n, "--unassign", &joined])?;
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
                self.run(&["issue", "update", &n, "--milestone", &m])?;
            }
            None => {
                // glab convention to clear the milestone.
                self.run(&["issue", "update", &n, "--milestone", ""])?;
            }
        }
        Ok(())
    }

    fn list_milestones(&self) -> Result<Vec<Milestone>, ForgeError> {
        let stdout = self.run(&["api", "projects/:id/milestones", "--paginate"])?;
        parse_gitlab_milestones(&stdout).map_err(Into::into)
    }
}

/// Build argv for `glab issue list` from an [`IssueFilter`] + limit.
pub(crate) fn build_glab_issue_list_args(filter: &IssueFilter, limit: u32) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "issue".into(),
        "list".into(),
        "--per-page".into(),
        limit.to_string(),
        "-F".into(),
        "json".into(),
    ];
    match filter.state {
        Some(IssueState::Open) => args.push("--opened".into()),
        Some(IssueState::Closed) => args.push("--closed".into()),
        None => args.push("--all".into()),
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

/// Build argv for `glab issue create` from a [`CreateIssueInput`].
pub(crate) fn build_glab_create_issue_args(input: &CreateIssueInput) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "issue".into(),
        "create".into(),
        "--title".into(),
        input.title.clone(),
        "--description".into(),
        input.body.clone(),
        "--no-editor".into(),
    ];
    if !input.labels.is_empty() {
        args.push("--label".into());
        args.push(input.labels.join(","));
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

/// Build argv for `glab issue update` from a patch. A returned vec of length
/// 3 (just `issue update N`) should be treated as a no-op by the caller.
pub(crate) fn build_glab_edit_issue_args(number: u64, patch: &EditIssuePatch) -> Vec<String> {
    let mut args: Vec<String> = vec!["issue".into(), "update".into(), number.to_string()];
    if let Some(t) = &patch.title {
        args.push("--title".into());
        args.push(t.clone());
    }
    if let Some(b) = &patch.body {
        args.push("--description".into());
        args.push(b.clone());
    }
    args
}

/// Parse stdout from `glab mr checkout N` into a [`CheckoutResult`].
///
/// Pure heuristic parser — looks for glab's "Checking out branch 'X' from
/// merge request !N" line and its "Adding remote <name>" line.
fn parse_glab_checkout_output(stdout: &str) -> CheckoutResult {
    let mut branch_name = String::new();
    let mut remote_added: Option<String> = None;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("Checking out branch '")
            && let Some(end) = rest.find('\'')
        {
            branch_name = rest[..end].to_string();
        }
        if let Some(rest) = line.strip_prefix("Adding remote ") {
            remote_added = Some(rest.trim().to_string());
        }
    }
    let is_fork = remote_added.is_some();
    CheckoutResult {
        branch_name,
        is_fork,
        remote_added,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_glab_checkout_simple_output() {
        let out = "Checking out branch 'feature-y' from merge request !7\n";
        let r = parse_glab_checkout_output(out);
        assert_eq!(r.branch_name, "feature-y");
        assert!(!r.is_fork);
        assert!(r.remote_added.is_none());
    }

    #[test]
    fn parse_glab_checkout_fork_adds_remote() {
        let out =
            "Adding remote fork-user\nChecking out branch 'fork-feature' from merge request !7\n";
        let r = parse_glab_checkout_output(out);
        assert_eq!(r.branch_name, "fork-feature");
        assert!(r.is_fork);
        assert_eq!(r.remote_added.as_deref(), Some("fork-user"));
    }

    #[test]
    fn parse_glab_checkout_empty_stdout() {
        let r = parse_glab_checkout_output("");
        assert_eq!(r.branch_name, "");
        assert!(!r.is_fork);
        assert!(r.remote_added.is_none());
    }

    // ─── Issue command builders (Phase 8.3) ────────────────────────────

    #[test]
    fn build_glab_issue_list_args_default_uses_all_flag() {
        let f = IssueFilter::default();
        let args = build_glab_issue_list_args(&f, 50);
        assert!(args.contains(&"--all".to_string()));
    }

    #[test]
    fn build_glab_issue_list_args_open_uses_opened_flag() {
        let f = IssueFilter {
            state: Some(IssueState::Open),
            ..Default::default()
        };
        let args = build_glab_issue_list_args(&f, 50);
        assert!(args.contains(&"--opened".to_string()));
    }

    #[test]
    fn build_glab_issue_list_args_closed_uses_closed_flag() {
        let f = IssueFilter {
            state: Some(IssueState::Closed),
            ..Default::default()
        };
        let args = build_glab_issue_list_args(&f, 50);
        assert!(args.contains(&"--closed".to_string()));
    }

    #[test]
    fn build_glab_create_issue_args_uses_description_flag() {
        let input = CreateIssueInput {
            title: "T".into(),
            body: "B".into(),
            labels: vec!["bug".into(), "docs".into()],
            assignees: vec!["alice".into()],
            milestone: Some(5),
        };
        let args = build_glab_create_issue_args(&input);
        assert!(args.contains(&"--title".to_string()));
        assert!(args.contains(&"--description".to_string()));
        // Labels are comma-joined as a single value (glab convention).
        assert!(args.windows(2).any(|w| w == ["--label", "bug,docs"]));
        assert!(args.windows(2).any(|w| w == ["--assignee", "alice"]));
        assert!(args.windows(2).any(|w| w == ["--milestone", "5"]));
    }

    #[test]
    fn build_glab_edit_issue_args_empty_patch_is_noop() {
        let args = build_glab_edit_issue_args(1, &EditIssuePatch::default());
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn build_glab_edit_issue_args_title_only() {
        let patch = EditIssuePatch {
            title: Some("new".into()),
            body: None,
        };
        let args = build_glab_edit_issue_args(7, &patch);
        assert!(args.windows(2).any(|w| w == ["--title", "new"]));
        assert!(!args.contains(&"--description".to_string()));
    }
}
