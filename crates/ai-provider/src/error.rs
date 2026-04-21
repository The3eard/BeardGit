//! Error types for AI provider operations.

/// Errors that can occur during AI provider operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AiError {
    /// The AI tool binary was not found on PATH.
    #[error("AI tool binary not found: {0}")]
    BinaryNotFound(String),

    /// Failed to construct a CLI command.
    #[error("failed to build command: {0}")]
    CommandBuild(String),

    /// An I/O error occurred (reading session files, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse output or data files.
    #[error("failed to parse: {0}")]
    Parse(String),

    /// The requested feature is not supported by this provider.
    #[error("feature not supported by this provider")]
    NotSupported,

    /// The provider's credentials are missing or expired.
    #[error("AI provider authentication expired: {0}")]
    AuthExpired(String),

    /// The provider reported a rate-limit / quota error.
    #[error("AI provider rate limited: {0}")]
    RateLimited(String),

    /// An unclassified error returned by the provider (raw stderr).
    #[error("AI provider error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = AiError::BinaryNotFound("claude".into());
        assert_eq!(err.to_string(), "AI tool binary not found: claude");

        let err = AiError::NotSupported;
        assert_eq!(err.to_string(), "feature not supported by this provider");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: AiError = io_err.into();
        assert!(matches!(err, AiError::Io(_)));
    }
}
