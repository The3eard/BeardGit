//! Error types for the cli-provider crate.

use thiserror::Error;

/// Errors returned by CLI provider operations.
#[derive(Error, Debug)]
pub enum CliError {
    /// The CLI binary was not found at the expected path.
    #[error("CLI binary not found: {0}")]
    BinaryNotFound(String),
    /// The CLI command exited with a non-zero status.
    #[error("CLI command failed: {0}")]
    CommandFailed(String),
    /// Failed to parse CLI JSON output.
    #[error("JSON parse error: {0}")]
    JsonError(String),
    /// An I/O error occurred spawning the CLI process.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Authentication has not been completed.
    #[error("Not authenticated: {0}")]
    NotAuthenticated(String),
}
