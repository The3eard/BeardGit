//! Issues for the GitLab CLI provider.
//!
//! Covers list / get / create / edit / close / reopen / comment / labels /
//! assignees / milestones. Keeps the `glab issue *` argv-builder helpers
//! colocated with the feature.

use std::collections::HashMap;

use forge_provider::{
    CreateIssueInput, EditIssuePatch, ForgeError, Issue, IssueDetail, IssueFilter, IssueState,
    Milestone,
};

use super::GitLabCli;
use crate::parsers::{
    parse_gitlab_issue_view, parse_gitlab_issues, parse_gitlab_milestones, parse_gitlab_notes,
};

impl GitLabCli {
    /// Return a snapshot of the repository label cache, populating it on
    /// first access. Returns an empty map on failure — colouring is a
    /// best-effort UX concern, not load-bearing.
    fn get_label_cache(&self) -> HashMap<String, forge_provider::Label> {
        if let Ok(guard) = self.label_cache.lock()
            && let Some(cache) = guard.as_ref()
        {
            return cache.clone();
        }
        let labels = self.list_labels_impl().unwrap_or_default();
        let map: HashMap<String, forge_provider::Label> =
            labels.into_iter().map(|l| (l.name.clone(), l)).collect();
        if let Ok(mut guard) = self.label_cache.lock() {
            *guard = Some(map.clone());
        }
        map
    }

    pub(super) fn list_issues_impl(
        &self,
        filter: IssueFilter,
        limit: u32,
    ) -> Result<Vec<Issue>, ForgeError> {
        let args = build_glab_issue_list_args(&filter, limit);
        let ref_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let stdout = self.run(&ref_args)?;
        parse_gitlab_issues(&stdout, &self.get_label_cache()).map_err(Into::into)
    }

    pub(super) fn get_issue_impl(&self, number: u64) -> Result<IssueDetail, ForgeError> {
        let num_str = number.to_string();
        let view = self.run(&["issue", "view", &num_str, "-F", "json"])?;
        let cache = self.get_label_cache();
        let (summary, body) = parse_gitlab_issue_view(&view, &cache).map_err(ForgeError::from)?;
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

    pub(super) fn create_issue_impl(&self, input: CreateIssueInput) -> Result<Issue, ForgeError> {
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
        let detail = self.get_issue_impl(number)?;
        Ok(detail.summary)
    }

    pub(super) fn edit_issue_impl(
        &self,
        number: u64,
        patch: EditIssuePatch,
    ) -> Result<(), ForgeError> {
        let args = build_glab_edit_issue_args(number, &patch);
        if args.len() == 3 {
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
        self.run(&["issue", "note", &n, "--message", body])?;
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
        self.run(&["issue", "update", &n, "--label", &joined])?;
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
        self.run(&["issue", "update", &n, "--unlabel", &joined])?;
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
        self.run(&["issue", "update", &n, "--assignee", &joined])?;
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
        self.run(&["issue", "update", &n, "--unassign", &joined])?;
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
                self.run(&["issue", "update", &n, "--milestone", &m])?;
            }
            None => {
                // glab convention to clear the milestone.
                self.run(&["issue", "update", &n, "--milestone", ""])?;
            }
        }
        Ok(())
    }

    pub(super) fn list_milestones_impl(&self) -> Result<Vec<Milestone>, ForgeError> {
        let stdout = self.run(&["api", "projects/:id/milestones", "--paginate"])?;
        parse_gitlab_milestones(&stdout).map_err(Into::into)
    }
}

// ─── argv builders ──────────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;

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
