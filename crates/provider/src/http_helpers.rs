//! Shared pure helpers for HTTP-based [`CiProvider`] implementations.
//!
//! This module must stay `reqwest`-free (see `lib.rs` trait-crate purity
//! note). All helpers operate on primitive values that the implementation
//! crates prepare from their HTTP responses — the shared surface is the
//! *shape of the response*, not the HTTP machinery.
//!
//! Typical caller pattern inside `gitlab-api` / `github-api`:
//!
//! ```ignore
//! if !resp.status().is_success() {
//!     let status = resp.status().as_u16();
//!     let message = resp.text().await.unwrap_or_default();
//!     return Err(http_helpers::api_error(status, message).into());
//! }
//! ```
//!
//! The helpers returned by this module are symmetric across both providers
//! and keep the error-mapping logic in one place.
//!
//! [`CiProvider`]: crate::CiProvider

use crate::ProviderError;

/// Build a [`ProviderError::Api`] from a status code + response body.
///
/// Sole purpose is to centralise the "non-2xx → Api variant" mapping so
/// each HTTP helper (GET, POST, PUT, DELETE) in every provider crate calls
/// the same constructor.
#[inline]
pub fn api_error(status: u16, message: impl Into<String>) -> ProviderError {
    ProviderError::Api {
        status,
        message: message.into(),
    }
}

/// Compute the seconds remaining until a rate-limit reset epoch.
///
/// GitHub returns `x-ratelimit-reset` as a unix epoch timestamp. When
/// `reset_epoch_secs <= now_epoch_secs` we return `0` — the quota has
/// already reset and a retry should succeed immediately.
///
/// # Example
///
/// ```
/// use provider::http_helpers::retry_after_secs;
/// assert_eq!(retry_after_secs(120, 100), 20);
/// assert_eq!(retry_after_secs(50, 100), 0); // already past
/// ```
#[inline]
pub fn retry_after_secs(reset_epoch_secs: u64, now_epoch_secs: u64) -> u64 {
    reset_epoch_secs.saturating_sub(now_epoch_secs)
}

/// Trim a trailing slash from a base URL.
///
/// Providers accept `"https://gitlab.com"` and `"https://gitlab.com/"`
/// interchangeably. Normalising at construction time keeps the rest of the
/// request-building code simpler.
#[inline]
pub fn trim_base_url(url: &str) -> &str {
    url.trim_end_matches('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_error_builds_api_variant() {
        let err = api_error(404, "not found");
        match err {
            ProviderError::Api { status, message } => {
                assert_eq!(status, 404);
                assert_eq!(message, "not found");
            }
            _ => panic!("expected Api variant"),
        }
    }

    #[test]
    fn retry_after_handles_past_reset() {
        assert_eq!(retry_after_secs(100, 200), 0);
    }

    #[test]
    fn retry_after_handles_future_reset() {
        assert_eq!(retry_after_secs(1000, 900), 100);
    }

    #[test]
    fn retry_after_handles_equal() {
        assert_eq!(retry_after_secs(500, 500), 0);
    }

    #[test]
    fn trim_base_url_strips_trailing_slash() {
        assert_eq!(trim_base_url("https://gitlab.com/"), "https://gitlab.com");
        assert_eq!(trim_base_url("https://gitlab.com"), "https://gitlab.com");
    }
}
