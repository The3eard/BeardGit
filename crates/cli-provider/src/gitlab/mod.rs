//! GitLab CLI provider — implements [`ForgeProvider`] by invoking `glab`.
//!
//! The [`ForgeProvider`] trait impl in this file is purely delegating: each
//! method calls into a feature-scoped submodule (e.g. [`mr_pr`], [`issues`],
//! [`releases`]). The submodules define `impl GitLabCli { pub(super) fn
//! *_impl(…) }` methods where the real logic lives, and keep their own
//! argv-builder / parser helpers colocated.

use std::path::PathBuf;

use forge_provider::{
    CheckoutResult, CreateIssueInput, CreateMrPrInput, CreateReleaseInput, CreateRepoInput,
    EditIssuePatch, EditMrPrPatch, EditReleasePatch, ForgeAuthStatus, ForgeError, ForgeKind,
    ForgeProvider, Issue, IssueDetail, IssueFilter, Label, MergeStrategy, Milestone, MrPr,
    MrPrDetail, MrPrDiffFile, MrPrFilter, Release, ReleaseAsset, ReleaseDetail, RepoCreated,
};

use crate::auth;
use crate::runner;

mod checkout;
mod discussions;
mod issues;
mod labels;
mod lifecycle;
mod mr_pr;
mod releases;
mod repo_create;
mod reviewers;

/// CLI-backed [`ForgeProvider`] for GitLab (using the bundled `glab` binary).
pub struct GitLabCli {
    /// Absolute path to the `glab` binary.
    pub binary_path: PathBuf,
    /// Working directory — the repository root. `glab` auto-detects the remote.
    pub repo_path: PathBuf,
    /// Lazy cache of repository labels for colouring issue labels in the
    /// list/detail views. Populated on first issue fetch.
    pub(super) label_cache:
        std::sync::Mutex<Option<std::collections::HashMap<String, forge_provider::Label>>>,
}

impl GitLabCli {
    /// Create a new GitLab CLI provider.
    pub fn new(binary_path: impl Into<PathBuf>, repo_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
            repo_path: repo_path.into(),
            label_cache: std::sync::Mutex::new(None),
        }
    }

    pub(super) fn run(&self, args: &[&str]) -> Result<String, ForgeError> {
        runner::run(&self.binary_path, &self.repo_path, args).map_err(Into::into)
    }

    pub(super) fn run_json<T: serde::de::DeserializeOwned>(
        &self,
        args: &[&str],
    ) -> Result<T, ForgeError> {
        runner::run_json(&self.binary_path, &self.repo_path, args).map_err(Into::into)
    }

    pub(super) fn run_with_stdin(
        &self,
        args: &[&str],
        stdin_data: &str,
    ) -> Result<String, ForgeError> {
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

    // ─── MR / PR ───────────────────────────────────────────────────────

    fn list_mr_prs(&self, filter: MrPrFilter, limit: u32) -> Result<Vec<MrPr>, ForgeError> {
        self.list_mr_prs_impl(filter, limit)
    }

    fn get_mr_pr(&self, number: u64) -> Result<MrPrDetail, ForgeError> {
        self.get_mr_pr_impl(number)
    }

    fn get_mr_pr_diff(&self, number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError> {
        self.get_mr_pr_diff_impl(number)
    }

    fn create_mr_pr(&self, input: CreateMrPrInput) -> Result<MrPr, ForgeError> {
        self.create_mr_pr_impl(input)
    }

    fn edit_mr_pr(&self, number: u64, patch: EditMrPrPatch) -> Result<(), ForgeError> {
        self.edit_mr_pr_impl(number, patch)
    }

    fn merge_mr_pr(&self, number: u64, strategy: MergeStrategy) -> Result<(), ForgeError> {
        self.merge_mr_pr_impl(number, strategy)
    }

    fn close_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        self.close_mr_pr_impl(number)
    }

    fn approve_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        self.approve_mr_pr_impl(number)
    }

    fn request_changes(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        self.request_changes_impl(number, body)
    }

    fn add_mr_pr_comment(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        self.add_mr_pr_comment_impl(number, body)
    }

    fn add_mr_pr_inline_comment(
        &self,
        number: u64,
        path: &str,
        line: u64,
        body: &str,
        base_sha: &str,
        head_sha: &str,
    ) -> Result<(), ForgeError> {
        self.add_mr_pr_inline_comment_impl(number, path, line, body, base_sha, head_sha)
    }

    // ─── Labels ────────────────────────────────────────────────────────

    fn add_mr_pr_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        self.add_mr_pr_labels_impl(number, labels)
    }

    fn remove_mr_pr_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        self.remove_mr_pr_labels_impl(number, labels)
    }

    fn list_labels(&self) -> Result<Vec<Label>, ForgeError> {
        self.list_labels_impl()
    }

    // ─── Reviewers ─────────────────────────────────────────────────────

    fn add_mr_pr_reviewers(&self, number: u64, reviewers: &[String]) -> Result<(), ForgeError> {
        self.add_mr_pr_reviewers_impl(number, reviewers)
    }

    fn remove_mr_pr_reviewers(&self, number: u64, reviewers: &[String]) -> Result<(), ForgeError> {
        self.remove_mr_pr_reviewers_impl(number, reviewers)
    }

    // ─── Lifecycle ─────────────────────────────────────────────────────

    fn mark_mr_pr_ready(&self, number: u64) -> Result<(), ForgeError> {
        self.mark_mr_pr_ready_impl(number)
    }

    fn mark_mr_pr_draft(&self, number: u64) -> Result<(), ForgeError> {
        self.mark_mr_pr_draft_impl(number)
    }

    fn reopen_mr_pr(&self, number: u64) -> Result<(), ForgeError> {
        self.reopen_mr_pr_impl(number)
    }

    // ─── Discussions (GitLab-only) ─────────────────────────────────────

    fn resolve_discussion(&self, number: u64, discussion_id: &str) -> Result<(), ForgeError> {
        self.resolve_discussion_impl(number, discussion_id)
    }

    fn unresolve_discussion(&self, number: u64, discussion_id: &str) -> Result<(), ForgeError> {
        self.unresolve_discussion_impl(number, discussion_id)
    }

    // ─── Checkout ──────────────────────────────────────────────────────

    fn checkout_mr_pr(&self, number: u64) -> Result<CheckoutResult, ForgeError> {
        self.checkout_mr_pr_impl(number)
    }

    // ─── Issues ────────────────────────────────────────────────────────

    fn list_issues(&self, filter: IssueFilter, limit: u32) -> Result<Vec<Issue>, ForgeError> {
        self.list_issues_impl(filter, limit)
    }

    fn get_issue(&self, number: u64) -> Result<IssueDetail, ForgeError> {
        self.get_issue_impl(number)
    }

    fn create_issue(&self, input: CreateIssueInput) -> Result<Issue, ForgeError> {
        self.create_issue_impl(input)
    }

    fn edit_issue(&self, number: u64, patch: EditIssuePatch) -> Result<(), ForgeError> {
        self.edit_issue_impl(number, patch)
    }

    fn close_issue(&self, number: u64) -> Result<(), ForgeError> {
        self.close_issue_impl(number)
    }

    fn reopen_issue(&self, number: u64) -> Result<(), ForgeError> {
        self.reopen_issue_impl(number)
    }

    fn add_issue_comment(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        self.add_issue_comment_impl(number, body)
    }

    fn add_issue_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        self.add_issue_labels_impl(number, labels)
    }

    fn remove_issue_labels(&self, number: u64, labels: &[String]) -> Result<(), ForgeError> {
        self.remove_issue_labels_impl(number, labels)
    }

    fn add_issue_assignees(&self, number: u64, assignees: &[String]) -> Result<(), ForgeError> {
        self.add_issue_assignees_impl(number, assignees)
    }

    fn remove_issue_assignees(&self, number: u64, assignees: &[String]) -> Result<(), ForgeError> {
        self.remove_issue_assignees_impl(number, assignees)
    }

    fn set_issue_milestone(
        &self,
        number: u64,
        milestone_id: Option<u64>,
    ) -> Result<(), ForgeError> {
        self.set_issue_milestone_impl(number, milestone_id)
    }

    fn list_milestones(&self) -> Result<Vec<Milestone>, ForgeError> {
        self.list_milestones_impl()
    }

    // ─── Releases ──────────────────────────────────────────────────────

    fn list_releases(&self, limit: u32) -> Result<Vec<Release>, ForgeError> {
        self.list_releases_impl(limit)
    }

    fn get_release(&self, tag: &str) -> Result<ReleaseDetail, ForgeError> {
        self.get_release_impl(tag)
    }

    fn list_release_assets(&self, tag: &str) -> Result<Vec<ReleaseAsset>, ForgeError> {
        self.list_release_assets_impl(tag)
    }

    fn create_release(&self, input: CreateReleaseInput) -> Result<Release, ForgeError> {
        self.create_release_impl(input)
    }

    fn edit_release(&self, tag: &str, patch: EditReleasePatch) -> Result<(), ForgeError> {
        self.edit_release_impl(tag, patch)
    }

    fn delete_release(&self, tag: &str) -> Result<(), ForgeError> {
        self.delete_release_impl(tag)
    }

    fn publish_release(&self, tag: &str) -> Result<(), ForgeError> {
        self.publish_release_impl(tag)
    }

    fn upload_release_asset(
        &self,
        tag: &str,
        path: &std::path::Path,
        label: Option<&str>,
    ) -> Result<ReleaseAsset, ForgeError> {
        self.upload_release_asset_impl(tag, path, label)
    }

    fn delete_release_asset(&self, tag: &str, asset_id: u64) -> Result<(), ForgeError> {
        self.delete_release_asset_impl(tag, asset_id)
    }

    // ─── Repo ──────────────────────────────────────────────────────────

    fn create_repo(&self, input: CreateRepoInput) -> Result<RepoCreated, ForgeError> {
        self.create_repo_impl(input)
    }
}
