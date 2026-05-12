//! Forge provider trait and shared types.
//!
//! Defines the [`ForgeProvider`] trait — the contract for code-forge backends
//! (GitHub, GitLab, and future forges such as Gitea, Forgejo, or Bitbucket).
//! This crate is Tauri-free and sync-only; implementations live in sibling
//! crates (e.g. `cli-provider`) and execute on `spawn_blocking` threads in
//! `app-core`.
//!
//! ## Trait-crate purity
//!
//! This crate must stay free of runtime dependencies (no `reqwest`, `tokio`
//! runtimes, `tauri`, `hyper`, or similar). CI contains a grep-based guard
//! that fails the build if one of those imports leaks in. The `mock` feature
//! enables a stand-in [`ForgeProvider`] implementation for integration tests.
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
    ///
    /// `base_sha` / `head_sha` are required by GitLab's discussion position
    /// payload; GitHub ignores them.
    fn add_mr_pr_inline_comment(
        &self,
        number: u64,
        path: &str,
        line: u64,
        body: &str,
        base_sha: &str,
        head_sha: &str,
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

    /// 8.2 — mark a discussion thread as resolved.
    ///
    /// GitLab-only; GitHub returns [`ForgeError::NotSupported`] because
    /// there is no equivalent CLI-exposed resolve operation.
    fn resolve_discussion(&self, _number: u64, _discussion_id: &str) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — mark a discussion thread as unresolved.
    ///
    /// GitLab-only; GitHub returns [`ForgeError::NotSupported`].
    fn unresolve_discussion(&self, _number: u64, _discussion_id: &str) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// Reply to an existing review-comment thread on a MR/PR.
    ///
    /// `thread_id` is forge-specific and matches the `discussion_id` populated
    /// on the inline [`Comment`] by the corresponding parser:
    /// - GitHub: the root review-comment id (REST `id`, as a decimal string).
    ///   The reply posts to `POST /pulls/{n}/comments/{id}/replies`.
    /// - GitLab: the discussion id (UUID-like string). The reply posts to
    ///   `POST /projects/:id/merge_requests/{n}/discussions/{id}/notes`.
    fn reply_to_review_comment(
        &self,
        _number: u64,
        _thread_id: &str,
        _body: &str,
    ) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — check out a MR/PR branch locally.
    ///
    /// Both CLIs handle fork-remote setup, fetch, and checkout in a single
    /// invocation. This synchronous method captures stdout and parses it
    /// into a [`CheckoutResult`]. Callers that need streaming output
    /// should invoke the CLI through `TaskManager` directly (see the
    /// `checkout_mr_pr_locally` Tauri command).
    fn checkout_mr_pr(&self, _number: u64) -> Result<CheckoutResult, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.2 — list all repository labels (for the label picker UI).
    fn list_labels(&self) -> Result<Vec<Label>, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    // ─── Issues (Phase 8.3) ────────────────────────────────────────────

    /// 8.3 — list issues for the current repo with optional filters.
    fn list_issues(&self, _filter: IssueFilter, _limit: u32) -> Result<Vec<Issue>, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — get full issue detail including body and comments.
    fn get_issue(&self, _number: u64) -> Result<IssueDetail, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — create a new issue.
    fn create_issue(&self, _input: CreateIssueInput) -> Result<Issue, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — edit an existing issue's title and/or body.
    fn edit_issue(&self, _number: u64, _patch: EditIssuePatch) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — close an open issue.
    fn close_issue(&self, _number: u64) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — reopen a closed issue.
    fn reopen_issue(&self, _number: u64) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — post a general comment on an issue.
    fn add_issue_comment(&self, _number: u64, _body: &str) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — add labels to an existing issue.
    fn add_issue_labels(&self, _number: u64, _labels: &[String]) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — remove labels from an existing issue.
    fn remove_issue_labels(&self, _number: u64, _labels: &[String]) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — add assignees to an existing issue.
    fn add_issue_assignees(&self, _number: u64, _assignees: &[String]) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — remove assignees from an existing issue.
    fn remove_issue_assignees(
        &self,
        _number: u64,
        _assignees: &[String],
    ) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — set (or clear) the milestone on an issue.
    ///
    /// Pass `None` to clear the milestone.
    fn set_issue_milestone(
        &self,
        _number: u64,
        _milestone_id: Option<u64>,
    ) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.3 — list all milestones for the current repo (for picker UIs).
    fn list_milestones(&self) -> Result<Vec<Milestone>, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    // ─── Releases (Phase 8.5) ──────────────────────────────────────────

    /// 8.5 — list releases for the current repo, newest first.
    fn list_releases(&self, _limit: u32) -> Result<Vec<Release>, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.5 — get full release detail (summary, notes body, assets).
    fn get_release(&self, _tag: &str) -> Result<ReleaseDetail, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.5 — create a new release from a [`CreateReleaseInput`].
    fn create_release(&self, _input: CreateReleaseInput) -> Result<Release, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.5 — edit the title/body/flags of an existing release.
    fn edit_release(&self, _tag: &str, _patch: EditReleasePatch) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.5 — delete a release. The underlying tag is not removed.
    fn delete_release(&self, _tag: &str) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.5 — publish a draft release (GitHub only; GitLab returns
    /// [`ForgeError::NotSupported`]).
    fn publish_release(&self, _tag: &str) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.5 — list the binary assets attached to a release.
    fn list_release_assets(&self, _tag: &str) -> Result<Vec<ReleaseAsset>, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.5 — upload a file as a release asset. Returns the newly-created
    /// [`ReleaseAsset`] record.
    fn upload_release_asset(
        &self,
        _tag: &str,
        _path: &std::path::Path,
        _label: Option<&str>,
    ) -> Result<ReleaseAsset, ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// 8.5 — delete a single release asset by its provider-specific ID.
    fn delete_release_asset(&self, _tag: &str, _asset_id: u64) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    // ─── Repository management ─────────────────────────────────────────

    /// Create a new repository in the authenticated user's namespace.
    ///
    /// Used by the "init repo on open" flow when the user opens a local
    /// project that has no remote yet. Default impl returns
    /// [`ForgeError::NotSupported`]; concrete CLI providers override this
    /// with `gh repo create` / `glab repo create`.
    fn create_repo(&self, _input: CreateRepoInput) -> Result<RepoCreated, ForgeError> {
        Err(ForgeError::NotSupported)
    }
}
