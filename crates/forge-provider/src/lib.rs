//! Forge provider trait and shared types.
//!
//! Defines the [`ForgeProvider`] trait — the contract for code-forge backends
//! (GitHub, GitLab, and future forges such as Gitea, Forgejo, or Bitbucket).
//! This crate is Tauri-free and sync-only; implementations live in sibling
//! crates (e.g. `cli-provider`) and execute on `spawn_blocking` threads in
//! `app-core`.
//!
//! # Design
//!
//! The trait is **fat**: it covers the union of capabilities across all
//! forges that BeardGit will ever target. Operations that are universally
//! available (MR/PR list/create/merge/…) are required methods. Operations
//! that are added in later sub-phases (issues in 8.3, releases in 8.5,
//! label/reviewer edits in 8.2, etc.) have default implementations that
//! return [`ForgeError::NotSupported`]. Concrete providers override only
//! the methods they support.
//!
//! Sub-phases 8.2 through 8.5 will *fill in* the `NotSupported` defaults
//! on `GitHubCli` and `GitLabCli`; this phase only lays the foundation.

pub mod error;
#[cfg(any(test, feature = "mock"))]
pub mod mock;
pub mod types;

pub use error::ForgeError;
pub use types::*;

/// Trait implemented by every forge backend (GitHub, GitLab, future forges).
///
/// Implementations are `Send + Sync` so they can be held as `Arc<dyn ForgeProvider>`
/// and cloned cheaply into `spawn_blocking` closures in `app-core`.
pub trait ForgeProvider: Send + Sync {
    // ─── Identity ───

    /// Which forge this provider instance represents.
    fn kind(&self) -> ForgeKind;

    /// High-level authentication status (installed + logged in + username).
    fn auth_status(&self) -> ForgeAuthStatus;

    // ─── MR/PR — required for all forge implementations ───

    /// List merge requests / pull requests for the current repo, newest first.
    fn list_mr_prs(&self, filter: MrPrFilter, limit: u32) -> Result<Vec<MrPr>, ForgeError>;

    /// Fetch detailed info about a single MR/PR including comments.
    fn get_mr_pr(&self, number: u64) -> Result<MrPrDetail, ForgeError>;

    /// Get the list of changed files in a MR/PR diff.
    fn get_mr_pr_diff(&self, number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError>;

    /// Open a new MR/PR with the given source/target branch and metadata.
    fn create_mr_pr(&self, input: CreateMrPrInput) -> Result<MrPr, ForgeError>;

    /// Edit the title and/or body of an existing MR/PR.
    fn edit_mr_pr(&self, number: u64, patch: EditMrPrPatch) -> Result<(), ForgeError>;

    /// Merge an open MR/PR using the given strategy.
    fn merge_mr_pr(&self, number: u64, strategy: MergeStrategy) -> Result<(), ForgeError>;

    /// Close an open MR/PR without merging.
    fn close_mr_pr(&self, number: u64) -> Result<(), ForgeError>;

    /// Approve an open MR/PR.
    fn approve_mr_pr(&self, number: u64) -> Result<(), ForgeError>;

    /// Request changes on an open MR/PR with a review body.
    ///
    /// GitHub submits a formal "request changes" review; GitLab posts the
    /// body as a comment (GitLab has no native "request changes" concept).
    fn request_changes(&self, number: u64, body: &str) -> Result<(), ForgeError>;

    /// Post a general comment on a MR/PR.
    fn add_mr_pr_comment(&self, number: u64, body: &str) -> Result<(), ForgeError>;

    /// Post an inline review comment on a specific file + line of a MR/PR diff.
    fn add_mr_pr_inline_comment(
        &self,
        number: u64,
        path: &str,
        line: u64,
        body: &str,
    ) -> Result<(), ForgeError>;

    // ─── Sub-phase stubs (default = NotSupported) ───
    //
    // These defaults are fillers so downstream consumers can already compile
    // against the fat trait. Each sub-phase (8.2, 8.3, 8.4, 8.5) will override
    // them on `GitHubCli` / `GitLabCli`.

    /// 8.2 — add labels to an existing MR/PR.
    fn add_mr_pr_labels(&self, _number: u64, _labels: &[String]) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — remove labels from an existing MR/PR.
    fn remove_mr_pr_labels(&self, _number: u64, _labels: &[String]) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — add reviewers to an existing MR/PR.
    fn add_mr_pr_reviewers(&self, _number: u64, _reviewers: &[String]) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — remove reviewers from an existing MR/PR.
    fn remove_mr_pr_reviewers(
        &self,
        _number: u64,
        _reviewers: &[String],
    ) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — mark a draft MR/PR as ready for review.
    fn mark_mr_pr_ready(&self, _number: u64) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — convert a ready MR/PR back to draft.
    fn mark_mr_pr_draft(&self, _number: u64) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — reopen a previously closed MR/PR.
    fn reopen_mr_pr(&self, _number: u64) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }
}
