//! HTTP client and error types for the GitLab REST API v4.

use crate::types::Project;
use ::provider::http_helpers;
use serde::de::DeserializeOwned;

/// Async HTTP client authenticated against a GitLab instance via a Personal Access Token.
#[derive(Clone)]
pub struct GitLabClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

// Custom Debug that redacts the PRIVATE-TOKEN. Without this a `tracing::debug!(?client)`
// or accidental `dbg!` would leak the PAT into log files.
impl std::fmt::Debug for GitLabClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitLabClient")
            .field("base_url", &self.base_url)
            .field("token", &"[REDACTED]")
            .finish()
    }
}

/// Errors that can be returned by any [`GitLabClient`] request.
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    /// A low-level HTTP transport error from `reqwest`.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// The GitLab server returned a non-2xx status code.
    #[error("API error {status}: {message}")]
    Api {
        /// HTTP status code returned by the server.
        status: u16,
        /// Response body (usually a GitLab error message).
        message: String,
    },
    /// Failed to deserialize the JSON response body.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl GitLabClient {
    /// Create a new client targeting `base_url` authenticated with `token`.
    ///
    /// TLS validation is strict for the public cloud (`gitlab.com`) so a
    /// MITM on the same network cannot swap in a fake cert and capture the
    /// PAT. For self-hosted CE/EE instances with private/self-signed certs
    /// the client opts into accepting invalid certs.
    pub fn new(base_url: &str, token: &str) -> Self {
        let trimmed = http_helpers::trim_base_url(base_url);
        let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));
        if http_helpers::should_accept_invalid_certs(trimmed) {
            builder = builder.danger_accept_invalid_certs(true);
        }
        let http = builder.build().unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            base_url: trimmed.to_string(),
            token: token.to_string(),
        }
    }

    /// Returns the base URL this client is configured against.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = format!("{}/api/v4{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api { status, message });
        }

        Ok(resp.json().await?)
    }

    pub(crate) async fn get_text(&self, path: &str) -> Result<String, ApiError> {
        let url = format!("{}/api/v4{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api { status, message });
        }

        Ok(resp.text().await?)
    }

    /// Find a project by its path (e.g. "namespace/project-name").
    pub async fn get_project(&self, path: &str) -> Result<Project, ApiError> {
        let encoded = urlencoding::encode(path);
        self.get(&format!("/projects/{encoded}")).await
    }

    /// POST a JSON body and deserialize the JSON response.
    pub(crate) async fn post_json<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let url = format!("{}/api/v4{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api { status, message });
        }
        Ok(resp.json().await?)
    }

    /// POST a JSON body and discard the response body (for 201/204 endpoints).
    pub(crate) async fn post_no_body<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let url = format!("{}/api/v4{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api { status, message });
        }
        Ok(())
    }
}
