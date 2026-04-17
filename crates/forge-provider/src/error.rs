//! Error type for forge operations.

use std::io;

use thiserror::Error;

/// Errors returned by any [`ForgeProvider`][crate::ForgeProvider] operation.
///
/// Implementations convert their internal error types into this enum at the
/// trait boundary so callers have a uniform error surface.
#[derive(Error, Debug)]
pub enum ForgeError {
    /// The provider does not support this operation.
    ///
    /// Returned by default-implemented trait methods and by provider-specific
    /// methods when the underlying forge lacks the capability (e.g. GitLab has
    /// no concept of a release draft, so `publish_release` returns this).
    #[error("operation not supported by this provider")]
    NotSupported,
    /// The caller is not authenticated (no token, expired token, revoked).
    #[error("not authenticated: {0}")]
    NotAuthenticated(String),
    /// The requested resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The forge API returned a non-success HTTP status.
    #[error("forge API error: status={status} message={message}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Human-readable message from the API response body.
        message: String,
    },
    /// A CLI subprocess failed (non-zero exit, binary missing, JSON parse error).
    #[error("CLI error: {0}")]
    Cli(String),
    /// Filesystem or process I/O failed while talking to the CLI.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_not_supported() {
        let e = ForgeError::NotSupported;
        assert_eq!(e.to_string(), "operation not supported by this provider");
    }

    #[test]
    fn display_api_error_formats_status_and_message() {
        let e = ForgeError::ApiError {
            status: 404,
            message: "missing".into(),
        };
        assert_eq!(e.to_string(), "forge API error: status=404 message=missing");
    }

    #[test]
    fn io_error_converts_via_from() {
        let io_err = io::Error::other("boom");
        let converted: ForgeError = io_err.into();
        assert!(matches!(converted, ForgeError::Io(_)));
    }
}
