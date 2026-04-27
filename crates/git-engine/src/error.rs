//! Error types for the git-engine crate.
//!
//! All public APIs in this crate return [`GitError`] on failure, which unifies
//! errors from `libgit2`, the filesystem, and repository discovery.

use thiserror::Error;

/// Unified error type for all git-engine operations.
#[derive(Error, Debug)]
pub enum GitError {
    /// A `libgit2` operation failed.
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    /// No git repository was found at or above the given path.
    #[error("Repository not found at {0}")]
    RepoNotFound(String),
    /// A git CLI command exited with a non-zero status.
    #[error("CLI error: {0}")]
    CliError(String),
    /// An I/O error occurred (e.g. spawning the git CLI process).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// The blob at the requested path is binary (contains a NUL byte in
    /// the first 8 KB). Callers should render a placeholder instead of a
    /// diff. Not a failure per se; a structured signal.
    #[error("binary file")]
    Binary,
}
