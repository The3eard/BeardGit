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
//! - [`blame`] — per-line blame and file history with rename tracking
//! - [`cli`] — shell-out wrapper for git CLI operations
//! - [`interactive_rebase`] — pre-planned interactive rebase via `GIT_SEQUENCE_EDITOR`
//! - [`worktree`] — list, create, and remove linked worktrees
//! - [`error`] — unified error type

pub mod blame;
pub mod clean;
pub mod cli;
pub mod commits;
pub mod conflict;
pub mod diff;
pub mod error;
pub mod file_content;
pub mod hunk_staging;
pub mod interactive_rebase;
pub mod operations;
pub mod reflog;
pub mod remote;
pub mod repository;
pub mod reset;
pub mod staging;
pub mod worktree;

pub use blame::{BlameLine, FileHistoryEntry};
pub use clean::CleanItem;
pub use cli::{CommitStats, GitCliResult, StashEntry, TagInfo};
pub use commits::CommitInfo;
pub use conflict::{ConflictFileContents, ConflictState, ConflictStatus};
pub use diff::{CommitFileChange, DiffHunkInfo, DiffLineInfo, FileDiff};
pub use error::GitError;
pub use hunk_staging::HunkSelection;
pub use interactive_rebase::{RebaseAction, RebaseCommit};
pub use reflog::ReflogEntry;
pub use repository::{BranchInfo, RepoStatus, Repository, StatusSummary};
pub use staging::FileStatus;
pub use worktree::WorktreeInfo;
