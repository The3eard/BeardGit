//! Shared JSON parsing utilities for GitHub and GitLab CLI output.
//!
//! Both providers parse JSON into the same unified types but with different
//! field names. These utilities extract the common parsing logic.

use serde_json::Value;

use forge_provider::{Comment, MrPr, MrPrState};

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
}
