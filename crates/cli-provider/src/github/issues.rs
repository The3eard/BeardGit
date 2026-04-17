//! Issues for the GitHub CLI provider.
//!
//! Covers list / get / create / edit / close / reopen / comment / labels /
//! assignees / milestones. Keeps the `gh issue *` argv-builder helpers
//! colocated with the feature.

use forge_provider::{
    CreateIssueInput, EditIssuePatch, ForgeError, Issue, IssueDetail, IssueFilter, IssueState,
    Milestone,
};

use super::GitHubCli;
use crate::parsers::{parse_github_issue_detail, parse_github_issues, parse_github_milestones};

impl GitHubCli {
    pub(super) fn list_issues_impl(
        &self,
        filter: IssueFilter,
        limit: u32,
    ) -> Result<Vec<Issue>, ForgeError> {
        let args = build_gh_issue_list_args(&filter, limit);
        let ref_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let stdout = self.run(&ref_args)?;
        parse_github_issues(&stdout).map_err(Into::into)
    }

    pub(super) fn get_issue_impl(&self, number: u64) -> Result<IssueDetail, ForgeError> {
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

    pub(super) fn create_issue_impl(&self, input: CreateIssueInput) -> Result<Issue, ForgeError> {
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
        let detail = self.get_issue_impl(number)?;
        Ok(detail.summary)
    }

    pub(super) fn edit_issue_impl(
        &self,
        number: u64,
        patch: EditIssuePatch,
    ) -> Result<(), ForgeError> {
        let args = build_gh_edit_issue_args(number, &patch);
        if args.len() == 3 {
            // No-op patch — avoid an unnecessary CLI call.
            return Ok(());
        }
        let ref_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.run(&ref_args)?;
        Ok(())
    }

    pub(super) fn close_issue_impl(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["issue", "close", &n])?;
        Ok(())
    }

    pub(super) fn reopen_issue_impl(&self, number: u64) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["issue", "reopen", &n])?;
        Ok(())
    }

    pub(super) fn add_issue_comment_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let n = number.to_string();
        self.run(&["issue", "comment", &n, "--body", body])?;
        Ok(())
    }

    pub(super) fn add_issue_labels_impl(
        &self,
        number: u64,
        labels: &[String],
    ) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = labels.join(",");
        self.run(&["issue", "edit", &n, "--add-label", &joined])?;
        Ok(())
    }

    pub(super) fn remove_issue_labels_impl(
        &self,
        number: u64,
        labels: &[String],
    ) -> Result<(), ForgeError> {
        if labels.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = labels.join(",");
        self.run(&["issue", "edit", &n, "--remove-label", &joined])?;
        Ok(())
    }

    pub(super) fn add_issue_assignees_impl(
        &self,
        number: u64,
        assignees: &[String],
    ) -> Result<(), ForgeError> {
        if assignees.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = assignees.join(",");
        self.run(&["issue", "edit", &n, "--add-assignee", &joined])?;
        Ok(())
    }

    pub(super) fn remove_issue_assignees_impl(
        &self,
        number: u64,
        assignees: &[String],
    ) -> Result<(), ForgeError> {
        if assignees.is_empty() {
            return Ok(());
        }
        let n = number.to_string();
        let joined = assignees.join(",");
        self.run(&["issue", "edit", &n, "--remove-assignee", &joined])?;
        Ok(())
    }

    pub(super) fn set_issue_milestone_impl(
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

    pub(super) fn list_milestones_impl(&self) -> Result<Vec<Milestone>, ForgeError> {
        let stdout = self.run(&[
            "api",
            "repos/{owner}/{repo}/milestones",
            "--paginate",
            "-F",
            "state=all",
        ])?;
        parse_github_milestones(&stdout).map_err(Into::into)
    }
}

// ─── argv builders ──────────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;

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
