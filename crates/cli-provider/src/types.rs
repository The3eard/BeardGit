//! Shared MR/PR types returned to the frontend.

use serde::{Deserialize, Serialize};

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
    pub comments: Vec<MrPrComment>,
    /// Aggregated review status.
    pub review_status: ReviewStatus,
    /// Whether the MR/PR can be merged (no conflicts, checks pass).
    pub mergeable: Option<bool>,
}

/// A comment on a MR/PR (general or inline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrPrComment {
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
