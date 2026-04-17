//! Provider-agnostic error type returned by [`crate::CiProvider`] methods.
//!
//! Wraps HTTP transport errors, API-level errors (non-2xx responses), JSON
//! deserialization failures, rate limiting, and unsupported operations.
//! Provider implementations convert their internal errors into this type at
//! the trait boundary.

/// Errors returned by provider operations.
///
/// Wraps HTTP transport errors, API-level errors (non-2xx responses), and
/// JSON deserialization failures. Provider implementations convert their
/// internal errors into this type.
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    /// HTTP transport-level error (timeout, DNS, TLS).
    #[error("HTTP error: {0}")]
    Http(String),
    /// Provider API returned a non-2xx status code.
    #[error("API error {status}: {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Response body or error message.
        message: String,
    },
    /// Failed to deserialize the response body.
    #[error("JSON error: {0}")]
    Json(String),
    /// Rate limit exceeded (GitHub: 5,000 req/hour).
    #[error("Rate limited — retry after {retry_after_secs}s")]
    RateLimited {
        /// Seconds until the rate limit resets.
        retry_after_secs: u64,
    },
    /// Operation is not supported by this provider (e.g. GitLab has no draft releases).
    #[error("operation not supported by this provider")]
    NotSupported,
}
