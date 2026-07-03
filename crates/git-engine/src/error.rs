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
    /// A signed commit/amend could not be produced — the `git commit`
    /// invocation failed while signing was enabled (bad key path, locked
    /// gpg-agent, missing signing program, …). Carries the git stderr so
    /// the UI can show the actual reason instead of a generic error.
    #[error("signing failed: {0}")]
    SigningFailed(String),
    /// An I/O error occurred (e.g. spawning the git CLI process).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// The blob at the requested path is binary (contains a NUL byte in
    /// the first 8 KB). Callers should render a placeholder instead of a
    /// diff. Not a failure per se; a structured signal.
    #[error("binary file")]
    Binary,
    /// The blob at the requested path is larger than the per-file cap
    /// for the current operation. Callers should render a placeholder
    /// instead of attempting to load + diff the content. Not a failure
    /// per se; a structured signal. `size` is the byte size of the blob.
    #[error("file too large ({size} bytes)")]
    FileTooLarge {
        /// Byte size of the blob.
        size: usize,
    },
    /// A repo-relative path supplied by a caller failed validation. Raised
    /// by helpers that refuse absolute paths, paths containing `..`
    /// segments, or paths that would resolve outside the repository's
    /// working tree.
    #[error("invalid path: {0}")]
    InvalidPath(String),
    /// A caller-supplied argument was invalid (e.g. an unrecognised reset
    /// mode or an out-of-bounds hunk index). Distinct from
    /// [`GitError::CliError`], which is reserved for a non-zero exit of the
    /// system `git` binary.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// A checkout was refused because it would overwrite uncommitted changes
    /// in the working tree (libgit2 reports a checkout conflict). Distinct
    /// from [`GitError::Git`] because the user acts on it differently —
    /// commit or stash first — so it carries a stable code across IPC.
    #[error("checkout would overwrite local changes: {0}")]
    WouldLoseChanges(String),
    /// A safe branch delete (`git branch -d`) was refused because the branch
    /// holds commits not reachable from HEAD. Distinct from
    /// [`GitError::CliError`] because the user acts on it differently —
    /// re-run with force to discard them — so it carries a stable code.
    #[error("branch is not fully merged: {0}")]
    NotFullyMerged(String),
}
