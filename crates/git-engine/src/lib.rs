//! Git engine crate for BeardGit.
//!
//! Provides a high-level interface to Git repositories built on top of `libgit2`
//! (`git2` crate) for read-heavy operations and the bundled git CLI for complex
//! write operations (merge, rebase, push, pull, stash, tags).
//!
//! # Modules
//! - [`repository`] — open repositories and inspect branches/status
//! - [`commits`] — walk and filter commit history
//! - [`staging`] — file status, stage/unstage operations
//! - [`operations`] — create commits, manage branches, checkout
//! - [`diff`] — diff working directory, index, and individual commits
//! - [`conflict`] — conflict detection, status, and abort/continue operations
//! - [`file_content`] — raw file content retrieval for CodeMirror diff views
//! - [`workdir_tree`] — list working-directory entries and perform light file CRUD
//! - [`gitignore`] — read, write, and append patterns to `.gitignore`
//! - [`blame`] — per-line blame and file history with rename tracking
//! - [`cli`] — shell-out wrapper for git CLI operations
//! - [`interactive_rebase`] — pre-planned interactive rebase via `GIT_SEQUENCE_EDITOR`
//! - [`worktree`] — list, create, and remove linked worktrees
//! - [`submodule`] — list, init, update, and deinit submodules
//! - [`error`] — unified error type

pub mod bisect;
pub mod blame;
pub mod clean;
pub mod cli;
pub mod commits;
pub mod config;
pub mod conflict;
pub mod diff;
pub mod error;
pub mod file_content;
pub mod gitignore;
pub mod hunk_staging;
pub mod interactive_rebase;
pub mod operations;
pub mod patch;
pub mod reflog;
pub mod remote;
pub mod rename_branch;
pub mod repository;
pub mod reset;
pub mod staging;
pub mod submodule;
pub mod workdir_tree;
pub mod worktree;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use bisect::BisectState;
pub use blame::{BlameLine, FileHistoryEntry};
pub use clean::CleanItem;
pub use cli::{CommitStats, GitCliResult, StashEntry, TagInfo};
pub use commits::{CommitInfo, CommitWalkOptions};
pub use config::{ConfigEntry, ConfigScope};
pub use conflict::{ConflictFileContents, ConflictState, ConflictStatus};
pub use diff::{
    CommitFileChange, DiffHunkInfo, DiffLineInfo, FileDiff, FileDiffStat, MAX_DIFF_RESPONSE_BYTES,
    enforce_response_budget,
};
pub use error::GitError;
pub use hunk_staging::HunkSelection;
pub use interactive_rebase::{RebaseAction, RebaseCommit};
pub use patch::{PatchPreview, PatchStat};
pub use reflog::ReflogEntry;
pub use repository::{BranchInfo, RepoStatus, Repository, StatusSummary};
pub use staging::FileStatus;
pub use submodule::{SubmoduleInfo, SubmoduleStatus};
pub use workdir_tree::WorkdirTreeEntry;
pub use worktree::WorktreeInfo;
