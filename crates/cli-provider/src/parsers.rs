//! Shared JSON parsing utilities for GitHub and GitLab CLI output.
//!
//! Both providers parse JSON into the same unified types but with different
//! field names. These utilities extract the common parsing logic.

use std::collections::HashMap;

use serde_json::Value;

use forge_provider::{
    Comment, Issue, IssueDetail, IssueState, Label, Milestone, MilestoneState, MrPr, MrPrState,
};

use crate::error::CliError;

/// Field name mapping for provider-specific JSON keys.
pub struct MrPrFieldMap {
    /// Field name for the MR/PR number (GitHub: "number", GitLab: "iid").
    pub number: &'static str,
    /// Field name for author username (GitHub: "login", GitLab: "username").
    pub author_field: &'static str,
    /// Author is nested under this key (both: "author").
    pub author_parent: &'static str,
    /// Field name for source branch (GitHub: "headRefName", GitLab: "source_branch").
    pub source_branch: &'static str,
    /// Field name for target branch (GitHub: "baseRefName", GitLab: "target_branch").
    pub target_branch: &'static str,
    /// Field name for URL (GitHub: "url", GitLab: "web_url").
    pub url: &'static str,
    /// Field name for draft flag (GitHub: "isDraft", GitLab: "draft").
    pub draft: &'static str,
    /// Additional draft field (GitLab: "work_in_progress", GitHub: none).
    pub draft_alt: Option<&'static str>,
    /// Labels are strings (GitLab) or objects with "name" field (GitHub).
    pub labels_are_strings: bool,
    /// Field name for reviewers array (GitHub: "reviewRequests", GitLab: "reviewers").
    pub reviewers: &'static str,
    /// Field name for reviewer username (GitHub: "login", GitLab: "username").
    pub reviewer_field: &'static str,
    /// Field name for created_at (GitHub: "createdAt", GitLab: "created_at").
    pub created_at: &'static str,
    /// Field name for updated_at (GitHub: "updatedAt", GitLab: "updated_at").
    pub updated_at: &'static str,
    /// State value mapping.
    pub state_open: &'static str,
    /// State value for closed.
    pub state_closed: &'static str,
    /// State value for merged.
    pub state_merged: &'static str,
}

/// GitHub field mapping.
pub const GITHUB_FIELDS: MrPrFieldMap = MrPrFieldMap {
    number: "number",
    author_field: "login",
    author_parent: "author",
    source_branch: "headRefName",
    target_branch: "baseRefName",
    url: "url",
    draft: "isDraft",
    draft_alt: None,
    labels_are_strings: false,
    reviewers: "reviewRequests",
    reviewer_field: "login",
    created_at: "createdAt",
    updated_at: "updatedAt",
    state_open: "OPEN",
    state_closed: "CLOSED",
    state_merged: "MERGED",
};

/// GitLab field mapping.
pub const GITLAB_FIELDS: MrPrFieldMap = MrPrFieldMap {
    number: "iid",
    author_field: "username",
    author_parent: "author",
    source_branch: "source_branch",
    target_branch: "target_branch",
    url: "web_url",
    draft: "draft",
    draft_alt: Some("work_in_progress"),
    labels_are_strings: true,
    reviewers: "reviewers",
    reviewer_field: "username",
    created_at: "created_at",
    updated_at: "updated_at",
    state_open: "opened",
    state_closed: "closed",
    state_merged: "merged",
};

/// Parse a JSON value into an `MrPr` using the given field mapping.
pub fn parse_mr_pr(item: &Value, fields: &MrPrFieldMap) -> MrPr {
    MrPr {
        number: item[fields.number].as_u64().unwrap_or(0),
        title: item["title"].as_str().unwrap_or("").to_string(),
        state: parse_state(item["state"].as_str().unwrap_or(""), fields),
        author: item[fields.author_parent][fields.author_field]
            .as_str()
            .unwrap_or("")
            .to_string(),
        source_branch: item[fields.source_branch]
            .as_str()
            .unwrap_or("")
            .to_string(),
        target_branch: item[fields.target_branch]
            .as_str()
            .unwrap_or("")
            .to_string(),
        url: item[fields.url].as_str().unwrap_or("").to_string(),
        draft: item[fields.draft].as_bool().unwrap_or(false)
            || fields
                .draft_alt
                .is_some_and(|alt| item[alt].as_bool().unwrap_or(false)),
        labels: parse_labels(&item["labels"], fields.labels_are_strings),
        reviewers: parse_string_array(&item[fields.reviewers], fields.reviewer_field),
        created_at: item[fields.created_at].as_str().unwrap_or("").to_string(),
        updated_at: item[fields.updated_at].as_str().unwrap_or("").to_string(),
        additions: item["additions"].as_u64(),
        deletions: item["deletions"].as_u64(),
        changed_files: item["changedFiles"].as_u64(),
    }
}

/// Parse a state string into `MrPrState` using the field map.
fn parse_state(raw: &str, fields: &MrPrFieldMap) -> MrPrState {
    if raw == fields.state_open {
        MrPrState::Open
    } else if raw == fields.state_closed {
        MrPrState::Closed
    } else if raw == fields.state_merged {
        MrPrState::Merged
    } else {
        MrPrState::Open
    }
}

/// Parse labels from JSON.
///
/// GitHub: array of objects with "name" field.
/// GitLab: array of plain strings.
fn parse_labels(value: &Value, are_strings: bool) -> Vec<String> {
    value
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| {
                    if are_strings {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v["name"].as_str().map(|s| s.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse an array of objects, extracting a string field from each.
fn parse_string_array(value: &Value, field: &str) -> Vec<String> {
    value
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v[field].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Parse a JSON comment into `Comment` for GitHub.
///
/// GitHub has no equivalent of GitLab's resolvable discussion threads, so
/// `resolvable`, `resolved`, and `discussion_id` are always `None`.
pub fn parse_github_comment(c: &Value) -> Comment {
    Comment {
        id: c["id"].as_u64().unwrap_or(0),
        author: c["author"]["login"].as_str().unwrap_or("").to_string(),
        body: c["body"].as_str().unwrap_or("").to_string(),
        created_at: c["createdAt"].as_str().unwrap_or("").to_string(),
        path: None,
        line: None,
        is_review: false,
        resolvable: None,
        resolved: None,
        discussion_id: None,
    }
}

/// Parse a JSON note into `Comment` for GitLab.
///
/// Populates `resolvable` and `resolved` from the note JSON if present.
/// `discussion_id` is set to `None` here; the caller (GitLab detail
/// fetcher) fills it in when iterating discussions.
pub fn parse_gitlab_comment(c: &Value) -> Comment {
    Comment {
        id: c["id"].as_u64().unwrap_or(0),
        author: c["author"]["username"].as_str().unwrap_or("").to_string(),
        body: c["body"].as_str().unwrap_or("").to_string(),
        created_at: c["created_at"].as_str().unwrap_or("").to_string(),
        path: c["position"]["new_path"].as_str().map(|s| s.to_string()),
        line: c["position"]["new_line"].as_u64(),
        is_review: c["type"].as_str() == Some("DiffNote"),
        resolvable: c["resolvable"].as_bool(),
        resolved: c["resolved"].as_bool(),
        discussion_id: None,
    }
}

// ─── Issues (Phase 8.3) ─────────────────────────────────────────────────────

/// Parse a GitHub label object `{ name, color, description }`.
pub fn parse_github_label(v: &Value) -> Label {
    Label {
        name: v["name"].as_str().unwrap_or("").to_string(),
        color: v["color"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_start_matches('#').to_string()),
        description: v["description"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    }
}

/// Parse GitHub labels array from issue JSON.
fn parse_github_labels_array(v: &Value) -> Vec<Label> {
    v.as_array()
        .map(|a| a.iter().map(parse_github_label).collect())
        .unwrap_or_default()
}

/// Map a GitHub issue state string (`"OPEN"`/`"CLOSED"`) to [`IssueState`].
fn parse_gh_issue_state(raw: &str) -> IssueState {
    if raw.eq_ignore_ascii_case("closed") {
        IssueState::Closed
    } else {
        IssueState::Open
    }
}

/// Map a GitHub milestone state string to [`MilestoneState`].
fn parse_gh_milestone_state(raw: &str) -> MilestoneState {
    if raw.eq_ignore_ascii_case("closed") {
        MilestoneState::Closed
    } else {
        MilestoneState::Open
    }
}

/// Parse a GitHub milestone object, using `number` as the id.
fn parse_github_milestone(v: &Value) -> Option<Milestone> {
    if v.is_null() {
        return None;
    }
    let id = v["number"].as_u64()?;
    Some(Milestone {
        id,
        title: v["title"].as_str().unwrap_or("").to_string(),
        state: parse_gh_milestone_state(v["state"].as_str().unwrap_or("")),
        due_on: v["dueOn"]
            .as_str()
            .or_else(|| v["due_on"].as_str())
            .map(|s| s.to_string()),
    })
}

/// Extract assignee usernames from a GitHub assignees array.
fn parse_github_assignees(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x["login"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Parse a single GitHub issue JSON value into [`Issue`].
fn parse_github_issue(item: &Value) -> Issue {
    let comments_count = item["comments"]
        .as_u64()
        .or_else(|| item["comments"].as_array().map(|a| a.len() as u64))
        .unwrap_or(0);
    Issue {
        number: item["number"].as_u64().unwrap_or(0),
        title: item["title"].as_str().unwrap_or("").to_string(),
        state: parse_gh_issue_state(item["state"].as_str().unwrap_or("")),
        author: item["author"]["login"].as_str().unwrap_or("").to_string(),
        labels: parse_github_labels_array(&item["labels"]),
        assignees: parse_github_assignees(&item["assignees"]),
        milestone: parse_github_milestone(&item["milestone"]),
        comments_count,
        created_at: item["createdAt"].as_str().unwrap_or("").to_string(),
        updated_at: item["updatedAt"].as_str().unwrap_or("").to_string(),
        url: item["url"].as_str().unwrap_or("").to_string(),
    }
}

/// Parse a GitHub issue-list JSON payload into `Vec<Issue>`.
pub fn parse_github_issues(json: &str) -> Result<Vec<Issue>, CliError> {
    let raw: Vec<Value> =
        serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    Ok(raw.iter().map(parse_github_issue).collect())
}

/// Parse a GitHub issue-detail JSON payload into [`IssueDetail`].
///
/// The detail view returns `comments` as an array of note objects rather than
/// a bare count; `comments_count` on the summary is derived from the array
/// length.
pub fn parse_github_issue_detail(json: &str) -> Result<IssueDetail, CliError> {
    let raw: Value = serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    let comments: Vec<Comment> = raw["comments"]
        .as_array()
        .map(|a| a.iter().map(parse_github_comment).collect())
        .unwrap_or_default();
    let mut summary = parse_github_issue(&raw);
    summary.comments_count = comments.len() as u64;
    let body = raw["body"].as_str().unwrap_or("").to_string();
    Ok(IssueDetail {
        summary,
        body,
        comments,
    })
}

/// Parse a GitHub label-list JSON payload.
pub fn parse_github_labels(json: &str) -> Result<Vec<Label>, CliError> {
    let raw: Vec<Value> =
        serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    Ok(raw.iter().map(parse_github_label).collect())
}

/// Parse a GitHub milestone-list JSON payload (from `gh api .../milestones`).
///
/// Uses the snake_case `number`/`due_on` keys that the raw REST API returns.
pub fn parse_github_milestones(json: &str) -> Result<Vec<Milestone>, CliError> {
    let raw: Vec<Value> =
        serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    Ok(raw
        .iter()
        .filter_map(|v| {
            let id = v["number"].as_u64()?;
            Some(Milestone {
                id,
                title: v["title"].as_str().unwrap_or("").to_string(),
                state: parse_gh_milestone_state(v["state"].as_str().unwrap_or("")),
                due_on: v["due_on"].as_str().map(|s| s.to_string()),
            })
        })
        .collect())
}

// ─── GitLab issue parsers ──────────────────────────────────────────────────

/// Map a GitLab issue state string to [`IssueState`].
fn parse_glab_issue_state(raw: &str) -> IssueState {
    if raw.eq_ignore_ascii_case("closed") {
        IssueState::Closed
    } else {
        // "opened" -> Open, anything else -> Open.
        IssueState::Open
    }
}

/// Map a GitLab milestone state string (`"active"`/`"closed"`).
fn parse_glab_milestone_state(raw: &str) -> MilestoneState {
    if raw.eq_ignore_ascii_case("closed") {
        MilestoneState::Closed
    } else {
        MilestoneState::Open
    }
}

/// Expand bare GitLab label strings into [`Label`] structs using an optional
/// cache, falling back to a default neutral color when a label is unknown.
fn expand_gitlab_labels(raw: &Value, cache: &HashMap<String, Label>) -> Vec<Label> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|name| {
                    cache.get(name).cloned().unwrap_or(Label {
                        name: name.to_string(),
                        color: Some("cccccc".to_string()),
                        description: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse a GitLab milestone object, using `iid` as the id when present.
fn parse_gitlab_milestone(v: &Value) -> Option<Milestone> {
    if v.is_null() {
        return None;
    }
    let id = v["iid"].as_u64().or_else(|| v["id"].as_u64())?;
    Some(Milestone {
        id,
        title: v["title"].as_str().unwrap_or("").to_string(),
        state: parse_glab_milestone_state(v["state"].as_str().unwrap_or("")),
        due_on: v["due_date"]
            .as_str()
            .or_else(|| v["due_on"].as_str())
            .map(|s| s.to_string()),
    })
}

/// Extract assignee usernames from a GitLab assignees array.
fn parse_gitlab_assignees(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x["username"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Parse a single GitLab issue JSON value into [`Issue`].
fn parse_gitlab_issue(item: &Value, cache: &HashMap<String, Label>) -> Issue {
    Issue {
        number: item["iid"].as_u64().unwrap_or(0),
        title: item["title"].as_str().unwrap_or("").to_string(),
        state: parse_glab_issue_state(item["state"].as_str().unwrap_or("")),
        author: item["author"]["username"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        labels: expand_gitlab_labels(&item["labels"], cache),
        assignees: parse_gitlab_assignees(&item["assignees"]),
        milestone: parse_gitlab_milestone(&item["milestone"]),
        comments_count: item["user_notes_count"].as_u64().unwrap_or(0),
        created_at: item["created_at"].as_str().unwrap_or("").to_string(),
        updated_at: item["updated_at"].as_str().unwrap_or("").to_string(),
        url: item["web_url"].as_str().unwrap_or("").to_string(),
    }
}

/// Parse a GitLab issue-list JSON payload.
///
/// `label_cache` is consulted to enrich bare label strings with color +
/// description; unknown labels fall through to a neutral default color.
pub fn parse_gitlab_issues(
    json: &str,
    label_cache: &HashMap<String, Label>,
) -> Result<Vec<Issue>, CliError> {
    let raw: Vec<Value> =
        serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    Ok(raw
        .iter()
        .map(|item| parse_gitlab_issue(item, label_cache))
        .collect())
}

/// Parse a GitLab issue-view JSON payload into the summary portion.
pub fn parse_gitlab_issue_view(
    json: &str,
    label_cache: &HashMap<String, Label>,
) -> Result<(Issue, String), CliError> {
    let raw: Value = serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    let summary = parse_gitlab_issue(&raw, label_cache);
    let body = raw["description"].as_str().unwrap_or("").to_string();
    Ok((summary, body))
}

/// Parse a GitLab note-list JSON payload (from `glab api projects/:id/issues/N/notes`).
pub fn parse_gitlab_notes(json: &str) -> Result<Vec<Comment>, CliError> {
    let raw: Vec<Value> =
        serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    // Skip system notes (e.g. "closed", "assigned ...") — the UI only cares
    // about human-authored comments.
    Ok(raw
        .iter()
        .filter(|n| !n["system"].as_bool().unwrap_or(false))
        .map(parse_gitlab_comment)
        .collect())
}

/// Parse a GitLab label-list JSON payload (colors include `#`, strip it).
pub fn parse_gitlab_labels(json: &str) -> Result<Vec<Label>, CliError> {
    let raw: Vec<Value> =
        serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    Ok(raw
        .iter()
        .map(|v| Label {
            name: v["name"].as_str().unwrap_or("").to_string(),
            color: v["color"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|s| s.trim_start_matches('#').to_string()),
            description: v["description"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
        })
        .collect())
}

/// Parse a GitLab milestone-list JSON payload.
///
/// Uses `iid` as the id for consistency with `list_issues` (see Plan notes).
pub fn parse_gitlab_milestones(json: &str) -> Result<Vec<Milestone>, CliError> {
    let raw: Vec<Value> =
        serde_json::from_str(json).map_err(|e| CliError::JsonError(e.to_string()))?;
    Ok(raw
        .iter()
        .filter_map(|v| {
            let id = v["iid"].as_u64().or_else(|| v["id"].as_u64())?;
            Some(Milestone {
                id,
                title: v["title"].as_str().unwrap_or("").to_string(),
                state: parse_glab_milestone_state(v["state"].as_str().unwrap_or("")),
                due_on: v["due_date"].as_str().map(|s| s.to_string()),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_github_mr_pr() {
        let item = json!({
            "number": 42,
            "title": "Fix bug",
            "state": "OPEN",
            "author": { "login": "alice" },
            "headRefName": "fix/bug",
            "baseRefName": "main",
            "url": "https://github.com/org/repo/pull/42",
            "isDraft": false,
            "labels": [{ "name": "bug" }],
            "reviewRequests": [{ "login": "bob" }],
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z",
            "additions": 10,
            "deletions": 5,
            "changedFiles": 3
        });

        let mr = parse_mr_pr(&item, &GITHUB_FIELDS);
        assert_eq!(mr.number, 42);
        assert_eq!(mr.title, "Fix bug");
        assert_eq!(mr.state, MrPrState::Open);
        assert_eq!(mr.author, "alice");
        assert_eq!(mr.source_branch, "fix/bug");
        assert_eq!(mr.target_branch, "main");
        assert!(!mr.draft);
        assert_eq!(mr.labels, vec!["bug"]);
        assert_eq!(mr.reviewers, vec!["bob"]);
        assert_eq!(mr.additions, Some(10));
        assert_eq!(mr.deletions, Some(5));
    }

    #[test]
    fn test_parse_gitlab_mr_pr() {
        let item = json!({
            "iid": 7,
            "title": "Add feature",
            "state": "opened",
            "author": { "username": "carlos" },
            "source_branch": "feature/x",
            "target_branch": "main",
            "web_url": "https://gitlab.com/org/repo/-/merge_requests/7",
            "draft": false,
            "work_in_progress": false,
            "labels": ["enhancement", "frontend"],
            "reviewers": [{ "username": "diana" }],
            "created_at": "2026-03-15T00:00:00Z",
            "updated_at": "2026-03-16T00:00:00Z"
        });

        let mr = parse_mr_pr(&item, &GITLAB_FIELDS);
        assert_eq!(mr.number, 7);
        assert_eq!(mr.title, "Add feature");
        assert_eq!(mr.state, MrPrState::Open);
        assert_eq!(mr.author, "carlos");
        assert_eq!(mr.source_branch, "feature/x");
        assert_eq!(mr.labels, vec!["enhancement", "frontend"]);
        assert_eq!(mr.reviewers, vec!["diana"]);
        assert!(mr.additions.is_none());
    }

    #[test]
    fn test_parse_state_mapping() {
        assert_eq!(parse_state("OPEN", &GITHUB_FIELDS), MrPrState::Open);
        assert_eq!(parse_state("CLOSED", &GITHUB_FIELDS), MrPrState::Closed);
        assert_eq!(parse_state("MERGED", &GITHUB_FIELDS), MrPrState::Merged);
        assert_eq!(parse_state("opened", &GITLAB_FIELDS), MrPrState::Open);
        assert_eq!(parse_state("closed", &GITLAB_FIELDS), MrPrState::Closed);
        assert_eq!(parse_state("merged", &GITLAB_FIELDS), MrPrState::Merged);
        // Unknown defaults to Open
        assert_eq!(parse_state("unknown", &GITHUB_FIELDS), MrPrState::Open);
    }

    #[test]
    fn test_parse_labels_github() {
        let labels = json!([{ "name": "bug" }, { "name": "critical" }]);
        assert_eq!(parse_labels(&labels, false), vec!["bug", "critical"]);
    }

    #[test]
    fn test_parse_labels_gitlab() {
        let labels = json!(["bug", "critical"]);
        assert_eq!(parse_labels(&labels, true), vec!["bug", "critical"]);
    }

    #[test]
    fn test_parse_labels_null() {
        assert_eq!(parse_labels(&json!(null), false), Vec::<String>::new());
    }

    #[test]
    fn test_parse_github_comment() {
        let comment = json!({
            "id": 100,
            "author": { "login": "alice" },
            "body": "Looks good",
            "createdAt": "2026-01-01T12:00:00Z"
        });
        let c = parse_github_comment(&comment);
        assert_eq!(c.id, 100);
        assert_eq!(c.author, "alice");
        assert_eq!(c.body, "Looks good");
        assert!(c.path.is_none());
        assert!(!c.is_review);
    }

    #[test]
    fn test_parse_gitlab_comment_inline() {
        let comment = json!({
            "id": 200,
            "author": { "username": "bob" },
            "body": "Fix this line",
            "created_at": "2026-02-01T12:00:00Z",
            "type": "DiffNote",
            "position": {
                "new_path": "src/main.rs",
                "new_line": 42
            }
        });
        let c = parse_gitlab_comment(&comment);
        assert_eq!(c.id, 200);
        assert_eq!(c.author, "bob");
        assert_eq!(c.path, Some("src/main.rs".to_string()));
        assert_eq!(c.line, Some(42));
        assert!(c.is_review);
    }

    #[test]
    fn test_parse_gitlab_comment_resolvable() {
        let note = json!({
            "id": 42,
            "author": { "username": "alice" },
            "body": "please fix",
            "created_at": "2026-04-16T10:00:00Z",
            "type": "DiffNote",
            "resolvable": true,
            "resolved": false,
            "position": { "new_path": "src/lib.rs", "new_line": 10 }
        });
        let c = parse_gitlab_comment(&note);
        assert_eq!(c.resolvable, Some(true));
        assert_eq!(c.resolved, Some(false));
        // discussion_id is populated by the caller, not the parser.
        assert!(c.discussion_id.is_none());
    }

    #[test]
    fn test_parse_gitlab_comment_no_resolvable_fields() {
        let note = json!({
            "id": 42,
            "author": { "username": "alice" },
            "body": "hi",
            "created_at": "2026-04-16T10:00:00Z"
        });
        let c = parse_gitlab_comment(&note);
        assert!(c.resolvable.is_none());
        assert!(c.resolved.is_none());
    }

    #[test]
    fn test_parse_github_comment_no_resolvable() {
        let item = json!({
            "id": 1,
            "author": { "login": "bob" },
            "body": "looks good",
            "createdAt": "2026-04-16T10:00:00Z"
        });
        let c = parse_github_comment(&item);
        assert!(c.resolvable.is_none());
        assert!(c.resolved.is_none());
        assert!(c.discussion_id.is_none());
    }

    #[test]
    fn test_gitlab_draft_with_wip() {
        let item = json!({
            "iid": 1,
            "title": "WIP: thing",
            "state": "opened",
            "author": { "username": "x" },
            "source_branch": "a",
            "target_branch": "b",
            "web_url": "",
            "draft": false,
            "work_in_progress": true,
            "labels": [],
            "reviewers": [],
            "created_at": "",
            "updated_at": ""
        });
        let mr = parse_mr_pr(&item, &GITLAB_FIELDS);
        assert!(mr.draft);
    }

    // ─── Issues (Phase 8.3) ────────────────────────────────────────────

    fn read_fixture(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(path).expect("fixture present")
    }

    #[test]
    fn parse_github_issues_from_fixture() {
        let json = read_fixture("gh_issue_list.json");
        let issues = parse_github_issues(&json).expect("parse");
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].number, 42);
        assert_eq!(issues[0].state, IssueState::Open);
        assert_eq!(issues[0].author, "alice");
        assert_eq!(issues[0].assignees, vec!["bob".to_string()]);
        assert_eq!(issues[0].milestone.as_ref().unwrap().title, "v1.0");
        assert_eq!(issues[0].labels[0].name, "bug");
        assert_eq!(issues[1].state, IssueState::Closed);
        assert!(issues[1].milestone.is_none());
    }

    #[test]
    fn parse_github_issue_detail_from_fixture() {
        let json = read_fixture("gh_issue_detail.json");
        let detail = parse_github_issue_detail(&json).expect("parse");
        assert_eq!(detail.summary.number, 42);
        assert_eq!(detail.summary.state, IssueState::Open);
        assert_eq!(detail.comments.len(), 1);
        assert_eq!(detail.summary.comments_count, 1);
        assert!(detail.body.starts_with("Steps"));
        assert_eq!(detail.comments[0].author, "dave");
    }

    #[test]
    fn parse_github_labels_from_fixture() {
        let json = read_fixture("gh_label_list.json");
        let labels = parse_github_labels(&json).expect("parse");
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].name, "bug");
        assert_eq!(labels[0].color.as_deref(), Some("d73a4a"));
        // Empty description is normalised to None.
        assert!(labels[1].description.is_none());
    }

    #[test]
    fn parse_github_milestones_from_fixture() {
        let json = read_fixture("gh_milestones.json");
        let ms = parse_github_milestones(&json).expect("parse");
        assert_eq!(ms.len(), 2);
        assert_eq!(ms[0].id, 1);
        assert_eq!(ms[0].state, MilestoneState::Open);
        assert_eq!(ms[1].state, MilestoneState::Closed);
        assert!(ms[0].due_on.is_some());
        assert!(ms[1].due_on.is_none());
    }

    #[test]
    fn parse_gitlab_issues_from_fixture_with_empty_cache() {
        let json = read_fixture("glab_issue_list.json");
        let cache = HashMap::new();
        let issues = parse_gitlab_issues(&json, &cache).expect("parse");
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].number, 42);
        assert_eq!(issues[0].state, IssueState::Open);
        assert_eq!(issues[0].labels[0].name, "bug");
        // Unknown label falls back to the neutral default color.
        assert_eq!(issues[0].labels[0].color.as_deref(), Some("cccccc"));
        assert_eq!(issues[0].assignees, vec!["bob".to_string()]);
        let m = issues[0].milestone.as_ref().unwrap();
        assert_eq!(m.id, 3);
        assert_eq!(m.state, MilestoneState::Open);
        assert_eq!(issues[1].state, IssueState::Closed);
    }

    #[test]
    fn parse_gitlab_issues_uses_label_cache() {
        let json = read_fixture("glab_issue_list.json");
        let mut cache = HashMap::new();
        cache.insert(
            "bug".to_string(),
            Label {
                name: "bug".into(),
                color: Some("d73a4a".into()),
                description: Some("broken".into()),
            },
        );
        let issues = parse_gitlab_issues(&json, &cache).expect("parse");
        assert_eq!(issues[0].labels[0].color.as_deref(), Some("d73a4a"));
        assert_eq!(issues[0].labels[0].description.as_deref(), Some("broken"));
    }

    #[test]
    fn parse_gitlab_issue_view_and_notes() {
        let view_json = read_fixture("glab_issue_detail.json");
        let notes_json = read_fixture("glab_issue_notes.json");
        let cache = HashMap::new();
        let (summary, body) = parse_gitlab_issue_view(&view_json, &cache).expect("view");
        assert_eq!(summary.number, 42);
        assert!(body.contains("Steps"));
        let comments = parse_gitlab_notes(&notes_json).expect("notes");
        // System notes are filtered out.
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].author, "dave");
    }

    #[test]
    fn parse_gitlab_labels_from_fixture_strips_hash() {
        let json = read_fixture("glab_label_list.json");
        let labels = parse_gitlab_labels(&json).expect("parse");
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].color.as_deref(), Some("d73a4a"));
    }

    #[test]
    fn parse_gitlab_milestones_from_fixture() {
        let json = read_fixture("glab_milestones.json");
        let ms = parse_gitlab_milestones(&json).expect("parse");
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].id, 1);
        assert_eq!(ms[0].state, MilestoneState::Open);
        assert_eq!(ms[0].due_on.as_deref(), Some("2026-05-01"));
    }
}
