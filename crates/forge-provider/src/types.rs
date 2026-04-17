//! Shared types returned by [`ForgeProvider`][crate::ForgeProvider] operations.
//!
//! All types here are serde-serializable and stable across the IPC boundary.
//! The snake_case serde representation is chosen so that TypeScript consumers
//! can use identical field names.

use serde::{Deserialize, Serialize};

// ─── Identity ───────────────────────────────────────────────────────────────

/// Which forge a provider instance speaks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeKind {
    /// `github.com` or GitHub Enterprise.
    GitHub,
    /// `gitlab.com` or self-hosted GitLab.
    GitLab,
}

/// High-level authentication signal for a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeAuthStatus {
    /// Authenticated — operations should succeed.
    Authenticated {
        /// Username of the authenticated user when known.
        username: Option<String>,
    },
    /// Not authenticated (no token, no CLI login).
    NotAuthenticated,
    /// Could not determine status (e.g. CLI binary missing).
    Unknown,
}

// ─── MR/PR ──────────────────────────────────────────────────────────────────

/// State of a merge request or pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MrPrState {
    /// The MR/PR is currently open.
    Open,
    /// The MR/PR has been closed without merging.
    Closed,
    /// The MR/PR has been merged.
    Merged,
}

/// Summary of a merge request or pull request (list view).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrPr {
    /// Numeric ID (iid for GitLab, number for GitHub).
    pub number: u64,
    /// Title of the MR/PR.
    pub title: String,
    /// Current state.
    pub state: MrPrState,
    /// Author username.
    pub author: String,
    /// Source branch name.
    pub source_branch: String,
    /// Target branch name.
    pub target_branch: String,
    /// Web URL to view in browser.
    pub url: String,
    /// Whether this is a draft/WIP.
    pub draft: bool,
    /// Labels assigned to the MR/PR.
    pub labels: Vec<String>,
    /// Assigned reviewers (usernames).
    pub reviewers: Vec<String>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last updated timestamp.
    pub updated_at: String,
    /// Number of additions (if available).
    pub additions: Option<u64>,
    /// Number of deletions (if available).
    pub deletions: Option<u64>,
    /// Number of changed files (if available).
    pub changed_files: Option<u64>,
}

/// Review status of a MR/PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    /// No reviews yet.
    Pending,
    /// At least one approval, no rejections.
    Approved,
    /// At least one request for changes.
    ChangesRequested,
    /// Reviews are mixed.
    Commented,
}

/// Detailed information about a single MR/PR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrPrDetail {
    /// Summary fields (same as list).
    pub summary: MrPr,
    /// Markdown body/description.
    pub body: String,
    /// Comments (general + inline).
    pub comments: Vec<Comment>,
    /// Aggregated review status.
    pub review_status: ReviewStatus,
    /// Whether the MR/PR can be merged (no conflicts, checks pass).
    pub mergeable: Option<bool>,
}

/// A comment on a MR/PR, issue, or release (general or inline).
///
/// Renamed from `MrPrComment` in 8.1 so it can be shared across all forge
/// resources that have comment threads (issues in 8.3, releases in 8.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Unique comment ID.
    pub id: u64,
    /// Author username.
    pub author: String,
    /// Markdown body of the comment.
    pub body: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// File path for inline comments, `None` for general comments.
    pub path: Option<String>,
    /// Line number for inline comments, `None` for general comments.
    pub line: Option<u64>,
    /// Whether this is part of a review (not a standalone comment).
    pub is_review: bool,
    /// GitLab-only: whether the comment (discussion) is marked resolvable.
    ///
    /// `None` on GitHub — GitHub has no equivalent concept of resolvable
    /// discussions exposed via the CLI.
    #[serde(default)]
    pub resolvable: Option<bool>,
    /// GitLab-only: whether the comment (discussion) is currently resolved.
    ///
    /// `None` on GitHub.
    #[serde(default)]
    pub resolved: Option<bool>,
    /// GitLab-only: discussion ID used by resolve/unresolve API calls.
    ///
    /// `None` on GitHub.
    #[serde(default)]
    pub discussion_id: Option<String>,
}

/// A file changed in a MR/PR diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrPrDiffFile {
    /// File path.
    pub path: String,
    /// Previous path (for renames).
    pub old_path: Option<String>,
    /// Change status: "added", "modified", "deleted", "renamed".
    pub status: String,
    /// Number of additions.
    pub additions: u64,
    /// Number of deletions.
    pub deletions: u64,
    /// Raw unified diff text for this file.
    pub patch: Option<String>,
}

/// Merge strategy for completing a MR/PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Standard merge commit.
    Merge,
    /// Squash all commits into one.
    Squash,
    /// Rebase commits onto target branch.
    Rebase,
}

/// Filter for [`crate::ForgeProvider::list_mr_prs`].
///
/// Added in 8.1 to replace the bare `(Option<MrPrState>, u32)` tuple the
/// current CLI impl uses. Future sub-phases will extend it with author,
/// label, and text filters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MrPrFilter {
    /// Restrict to a single state (Open/Closed/Merged); `None` means any.
    pub state: Option<MrPrState>,
}

/// Input payload for [`crate::ForgeProvider::create_mr_pr`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMrPrInput {
    /// Source branch name.
    pub source: String,
    /// Target branch name.
    pub target: String,
    /// Title for the new MR/PR.
    pub title: String,
    /// Markdown description/body.
    pub body: String,
    /// Whether to open as draft.
    pub draft: bool,
    /// Labels to apply on creation.
    pub labels: Vec<String>,
    /// Reviewers (usernames) to request on creation.
    pub reviewers: Vec<String>,
}

/// Fields to change on an existing MR/PR via [`crate::ForgeProvider::edit_mr_pr`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditMrPrPatch {
    /// New title (leave `None` to keep current title).
    pub title: Option<String>,
    /// New body (leave `None` to keep current body).
    pub body: Option<String>,
    /// Toggle draft state. `Some(true)` → convert to draft, `Some(false)`
    /// → mark ready, `None` → leave unchanged.
    ///
    /// Providers with dedicated draft lifecycle commands (both GitHub
    /// `pr ready` and GitLab `mr update --ready/--draft`) are invoked
    /// through [`crate::ForgeProvider::mark_mr_pr_ready`] and
    /// [`crate::ForgeProvider::mark_mr_pr_draft`]; this field is here
    /// primarily for symmetry with the edit API.
    pub draft: Option<bool>,
}

/// Result of checking out a MR/PR branch locally via
/// [`crate::ForgeProvider::checkout_mr_pr`].
///
/// Enables the frontend to show a useful toast (e.g. "Checked out
/// `feature/foo`; added remote `fork`") and decide whether to refresh the
/// remotes list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutResult {
    /// Name of the local branch that was checked out.
    pub branch_name: String,
    /// Whether the MR/PR source branch lives on a fork (a new remote was
    /// needed to fetch it).
    pub is_fork: bool,
    /// Name of the remote that was added for the fork, if any.
    pub remote_added: Option<String>,
}

/// A repository label (used by both issues and MR/PRs).
///
/// Returned by [`crate::ForgeProvider::list_labels`] for populating the
/// label picker UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    /// Label name (identifier — `add_mr_pr_labels` takes these).
    pub name: String,
    /// Hex color without the leading `#`, if provided.
    pub color: Option<String>,
    /// Optional human-readable description.
    pub description: Option<String>,
}

#[cfg(test)]
mod types_tests {
    use super::*;

    #[test]
    fn checkout_result_serializes_snake_case() {
        let cr = CheckoutResult {
            branch_name: "feature/foo".into(),
            is_fork: true,
            remote_added: Some("fork".into()),
        };
        let json = serde_json::to_string(&cr).unwrap();
        assert!(json.contains("\"branch_name\":\"feature/foo\""));
        assert!(json.contains("\"is_fork\":true"));
        assert!(json.contains("\"remote_added\":\"fork\""));
    }

    #[test]
    fn label_round_trips() {
        let l = Label {
            name: "bug".into(),
            color: Some("ff0000".into()),
            description: Some("Something broken".into()),
        };
        let json = serde_json::to_string(&l).unwrap();
        let back: Label = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "bug");
        assert_eq!(back.color.as_deref(), Some("ff0000"));
        assert_eq!(back.description.as_deref(), Some("Something broken"));
    }

    #[test]
    fn label_without_color_or_description() {
        let l = Label {
            name: "plain".into(),
            color: None,
            description: None,
        };
        let json = serde_json::to_string(&l).unwrap();
        let back: Label = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "plain");
        assert!(back.color.is_none());
        assert!(back.description.is_none());
    }

    #[test]
    fn edit_patch_has_optional_draft() {
        let patch = EditMrPrPatch {
            title: Some("new title".into()),
            body: None,
            draft: Some(false),
        };
        let json = serde_json::to_string(&patch).unwrap();
        assert!(json.contains("\"draft\":false"));
    }

    #[test]
    fn edit_patch_default_draft_none() {
        let patch = EditMrPrPatch::default();
        assert!(patch.draft.is_none());
    }

    #[test]
    fn comment_with_resolvable_fields() {
        let c = Comment {
            id: 1,
            author: "alice".into(),
            body: "please fix".into(),
            created_at: "2026-04-16T10:00:00Z".into(),
            path: None,
            line: None,
            is_review: false,
            resolvable: Some(true),
            resolved: Some(false),
            discussion_id: Some("abc123".into()),
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Comment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.resolvable, Some(true));
        assert_eq!(back.resolved, Some(false));
        assert_eq!(back.discussion_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn comment_without_resolvable_fields_defaults() {
        // Simulates a payload from GitHub which omits the new optional keys.
        let json = r#"{
            "id": 1,
            "author": "bob",
            "body": "looks good",
            "created_at": "2026-04-16T10:00:00Z",
            "path": null,
            "line": null,
            "is_review": false
        }"#;
        let c: Comment = serde_json::from_str(json).unwrap();
        assert!(c.resolvable.is_none());
        assert!(c.resolved.is_none());
        assert!(c.discussion_id.is_none());
    }
}
