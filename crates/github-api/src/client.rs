//! HTTP client for the GitHub REST API.

use ::provider::http_helpers;
use serde::de::DeserializeOwned;

/// Errors returned by [`GitHubClient`] requests.
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    /// A low-level HTTP transport error from `reqwest`.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// The GitHub server returned a non-2xx status code.
    #[error("API error {status}: {message}")]
    Api {
        /// HTTP status code returned by the server.
        status: u16,
        /// Response body (usually a GitHub error message).
        message: String,
    },
    /// Failed to deserialize the JSON response body.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Rate limit has been exhausted.
    #[error("Rate limited — retry after {retry_after_secs}s")]
    RateLimited {
        /// Seconds until the rate limit resets.
        retry_after_secs: u64,
    },
}

/// Async HTTP client for the GitHub REST API.
///
/// Authenticates via `Authorization: Bearer <pat>` and sets the required
/// `Accept` and `X-GitHub-Api-Version` headers on every request.
/// Checks `x-ratelimit-remaining` on each response and returns
/// [`ApiError::RateLimited`] when the quota is exhausted.
#[derive(Clone)]
pub struct GitHubClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

// Custom Debug that redacts the bearer token. Without this a `tracing::debug!(?client)`
// or accidental `dbg!` would leak the PAT into log files.
impl std::fmt::Debug for GitHubClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubClient")
            .field("base_url", &self.base_url)
            .field("token", &"[REDACTED]")
            .finish()
    }
}

impl GitHubClient {
    /// Create a new client targeting `base_url` authenticated with `token`.
    ///
    /// For github.com, use `"https://api.github.com"`.
    /// For GitHub Enterprise, use `"https://<host>/api/v3"`.
    ///
    /// TLS validation is strict for the public cloud (`api.github.com` /
    /// `github.com`) so a MITM cannot swap in a fake cert and capture the
    /// bearer token. For Enterprise instances with private/self-signed
    /// certs the client opts into accepting invalid certs, mirroring how
    /// the user's `gh` CLI typically already trusts those internal CAs.
    pub fn new(base_url: &str, token: &str) -> Self {
        let normalized = Self::normalize_url(base_url);
        let mut builder = reqwest::Client::builder()
            .user_agent("BeardGit")
            .timeout(std::time::Duration::from_secs(30));
        if http_helpers::should_accept_invalid_certs(&normalized) {
            builder = builder.danger_accept_invalid_certs(true);
        }
        // The fallback must still carry a timeout — a default `Client::new()`
        // has none and could hang indefinitely on a stalled connection.
        let http = builder.build().unwrap_or_else(|_| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        });

        Self {
            http,
            base_url: normalized,
            token: token.to_string(),
        }
    }

    /// Normalize a GitHub instance URL so that `https://github.com` becomes
    /// `https://api.github.com`. Other hosts (GitHub Enterprise) are left unchanged.
    pub fn normalize_url(url: &str) -> String {
        let trimmed = http_helpers::trim_base_url(url);
        let lower = trimmed.to_lowercase();
        if lower == "https://github.com" || lower == "http://github.com" {
            "https://api.github.com".to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Returns the base URL this client is configured against.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Check response headers for GitHub rate limit exhaustion.
    ///
    /// Returns `Err(ApiError::RateLimited)` if the remaining quota is zero.
    /// Uses [`http_helpers::retry_after_secs`] for the pure reset-epoch
    /// arithmetic so the logic is unit-testable without fabricating a
    /// full HTTP response.
    fn check_rate_limit(resp: &reqwest::Response) -> Result<(), ApiError> {
        if let Some(remaining) = resp.headers().get("x-ratelimit-remaining")
            && remaining.to_str().unwrap_or("1") == "0"
        {
            let reset = resp
                .headers()
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return Err(ApiError::RateLimited {
                retry_after_secs: http_helpers::retry_after_secs(reset, now),
            });
        }
        Ok(())
    }

    /// Perform a GET request, deserializing the JSON response.
    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?;

        if !resp.status().is_success() {
            // Only treat a quota-exhausted response as RateLimited on an error
            // status — a 2xx that happens to consume the last quota unit
            // (x-ratelimit-remaining: 0) is a successful fetch and must return
            // its data rather than being discarded as a failure.
            Self::check_rate_limit(&resp)?;
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api { status, message });
        }

        Ok(resp.json().await?)
    }

    /// POST a JSON body for endpoints that return 202/204/No Content.
    pub(crate) async fn post_no_body<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            // See `get`: only a non-2xx response is treated as rate-limited.
            Self::check_rate_limit(&resp)?;
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api { status, message });
        }
        Ok(())
    }

    /// Perform a GET request that returns plain text (follows redirects).
    pub(crate) async fn get_text(&self, path: &str) -> Result<String, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?;

        if !resp.status().is_success() {
            // Only treat a quota-exhausted response as RateLimited on an error
            // status — a 2xx that happens to consume the last quota unit
            // (x-ratelimit-remaining: 0) is a successful fetch and must return
            // its data rather than being discarded as a failure.
            Self::check_rate_limit(&resp)?;
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api { status, message });
        }

        Ok(resp.text().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_rewrites_github_com() {
        assert_eq!(
            GitHubClient::normalize_url("https://github.com"),
            "https://api.github.com"
        );
        assert_eq!(
            GitHubClient::normalize_url("https://github.com/"),
            "https://api.github.com"
        );
        assert_eq!(
            GitHubClient::normalize_url("https://GitHub.com"),
            "https://api.github.com"
        );
    }

    #[test]
    fn normalize_url_preserves_api_url() {
        assert_eq!(
            GitHubClient::normalize_url("https://api.github.com"),
            "https://api.github.com"
        );
    }

    #[test]
    fn normalize_url_preserves_enterprise() {
        assert_eq!(
            GitHubClient::normalize_url("https://github.example.com/api/v3"),
            "https://github.example.com/api/v3"
        );
    }
}
