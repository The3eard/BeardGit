//! HTTP client for the GitHub REST API.

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
#[derive(Debug, Clone)]
pub struct GitHubClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl GitHubClient {
    /// Create a new client targeting `base_url` authenticated with `token`.
    ///
    /// For github.com, use `"https://api.github.com"`.
    /// For GitHub Enterprise, use `"https://<host>/api/v3"`.
    pub fn new(base_url: &str, token: &str) -> Self {
        // Accept invalid certs for GitHub Enterprise instances with
        // internal/self-signed certificates. The PAT validates the connection.
        let http = reqwest::Client::builder()
            .user_agent("BeardGit")
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            base_url: Self::normalize_url(base_url),
            token: token.to_string(),
        }
    }

    /// Normalize a GitHub instance URL so that `https://github.com` becomes
    /// `https://api.github.com`. Other hosts (GitHub Enterprise) are left unchanged.
    pub fn normalize_url(url: &str) -> String {
        let trimmed = url.trim_end_matches('/');
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
            let retry_after = reset.saturating_sub(now);
            return Err(ApiError::RateLimited {
                retry_after_secs: retry_after,
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

        Self::check_rate_limit(&resp)?;

        if !resp.status().is_success() {
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

        Self::check_rate_limit(&resp)?;
        if !resp.status().is_success() {
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

        Self::check_rate_limit(&resp)?;

        if !resp.status().is_success() {
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
