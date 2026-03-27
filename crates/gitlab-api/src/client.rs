//! HTTP client and error types for the GitLab REST API v4.

use crate::types::Project;
use serde::de::DeserializeOwned;

/// Async HTTP client authenticated against a GitLab instance via a Personal Access Token.
#[derive(Debug, Clone)]
pub struct GitLabClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
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
    /// Invalid TLS certificates are accepted to support self-hosted instances with
    /// self-signed certificates; the PAT is relied upon for authentication.
    pub fn new(base_url: &str, token: &str) -> Self {
        // Accept invalid certs for self-hosted GitLab instances with
        // internal/self-signed certificates. The PAT validates the connection.
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
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
}
