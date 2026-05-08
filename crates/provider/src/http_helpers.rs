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

/// Return `true` if a `reqwest::Client` targeting `url` may safely accept
/// invalid TLS certificates.
///
/// Self-hosted GitHub Enterprise and GitLab CE/EE instances are commonly
/// served with private/self-signed certificates and rely on the PAT for
/// authentication. The public clouds (`api.github.com`, `github.com`,
/// `gitlab.com`) **must** validate certs strictly — otherwise a MITM on the
/// same network can capture the bearer token and decrypt every subsequent
/// API request.
///
/// Returns `true` (skip cert verification) **only** when the user has set
/// `BEARDGIT_INSECURE_TLS=1` *and* the host is not one of the public
/// clouds. Otherwise always returns `false` so TLS verification stays on.
///
/// The previous behaviour silently disabled cert checks for any host that
/// wasn't a public-cloud forge — a MITM with a self-signed cert on a
/// corporate LAN could harvest PATs from every BeardGit instance pointed
/// at `gitlab.company.com` without warning. Switching to opt-in matches
/// the precedent set by `gh` (`GH_INSECURE`) and makes the trust
/// decision visible in the user's environment instead of implicit in
/// the binary.
pub fn should_accept_invalid_certs(url: &str) -> bool {
    if is_public_forge_host(url) {
        return false;
    }
    matches!(
        std::env::var("BEARDGIT_INSECURE_TLS").as_deref(),
        Ok("1" | "true" | "yes")
    )
}

/// Hostname check for the public clouds. Case-insensitive; tolerates an
/// optional port and any path / query segment.
fn is_public_forge_host(url: &str) -> bool {
    const PUBLIC_HOSTS: &[&str] = &["api.github.com", "github.com", "gitlab.com"];

    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    PUBLIC_HOSTS.iter().any(|h| host == *h)
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

    #[test]
    fn public_clouds_require_strict_tls() {
        for url in [
            "https://api.github.com",
            "https://api.github.com/",
            "https://api.github.com/user",
            "https://github.com",
            "https://github.com/api/v3",
            "https://gitlab.com",
            "https://gitlab.com/api/v4/projects",
            "https://API.GITHUB.COM/user",
        ] {
            assert!(
                !should_accept_invalid_certs(url),
                "expected strict TLS for {url}"
            );
        }
    }

    #[test]
    fn tls_default_strict_and_opt_in_only_for_self_hosted() {
        // Single combined test because all three behaviours read the same
        // process-global env var; running them in parallel would race.
        // SAFETY: we hold the lock for the whole test and restore the
        // variable on exit, so concurrent module tests stay deterministic.
        let _g = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("BEARDGIT_INSECURE_TLS");
        }

        // 1. Self-hosted defaults to strict TLS.
        for url in [
            "https://github.example.com/api/v3",
            "https://gitlab.example.com/api/v4",
            "https://my-internal-gitlab/",
            "https://gitlab.com.attacker.example",
            "https://192.168.1.10/",
        ] {
            assert!(
                !should_accept_invalid_certs(url),
                "expected strict TLS by default for {url}"
            );
        }

        // 2. Opt-in flips self-hosted to lenient.
        unsafe {
            std::env::set_var("BEARDGIT_INSECURE_TLS", "1");
        }
        assert!(
            should_accept_invalid_certs("https://my-internal-gitlab/"),
            "BEARDGIT_INSECURE_TLS=1 should permit self-hosted laxity"
        );

        // 3. Public clouds stay strict even with the opt-in.
        assert!(
            !should_accept_invalid_certs("https://api.github.com/user"),
            "public clouds must stay strict regardless of env"
        );

        unsafe {
            std::env::remove_var("BEARDGIT_INSECURE_TLS");
        }
    }

    /// Lock to serialise tests that mutate `BEARDGIT_INSECURE_TLS`.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
