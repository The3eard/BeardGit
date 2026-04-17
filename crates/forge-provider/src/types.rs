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
}
