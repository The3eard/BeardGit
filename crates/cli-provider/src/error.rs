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

impl From<CliError> for forge_provider::ForgeError {
    fn from(err: CliError) -> Self {
        match err {
            CliError::BinaryNotFound(p) => {
                forge_provider::ForgeError::Cli(format!("binary not found: {p}"))
            }
            CliError::CommandFailed(m) => forge_provider::ForgeError::Cli(m),
            CliError::JsonError(m) => forge_provider::ForgeError::Cli(format!("json: {m}")),
            CliError::Io(e) => forge_provider::ForgeError::Io(e),
            CliError::NotAuthenticated(m) => forge_provider::ForgeError::NotAuthenticated(m),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_provider::ForgeError;

    #[test]
    fn binary_not_found_becomes_cli() {
        let e: ForgeError = CliError::BinaryNotFound("/missing/gh".into()).into();
        match e {
            ForgeError::Cli(m) => assert!(m.contains("/missing/gh")),
            _ => panic!("expected Cli variant"),
        }
    }

    #[test]
    fn not_authenticated_maps_through() {
        let e: ForgeError = CliError::NotAuthenticated("no token".into()).into();
        assert!(matches!(e, ForgeError::NotAuthenticated(_)));
    }

    #[test]
    fn io_error_preserved() {
        let e: ForgeError = CliError::Io(std::io::Error::other("x")).into();
        assert!(matches!(e, ForgeError::Io(_)));
    }
}
