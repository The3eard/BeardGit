//! Personal Access Token (PAT) validation for git hosting providers.
//!
//! Each validation function calls the provider's "get current user" API endpoint
//! to confirm the token is valid, then normalizes the response into a
//! [`provider::ProviderUser`].

use provider::ProviderUser;

use crate::error::AuthError;

// NOTE: These response types intentionally duplicate the types in gitlab-api and
// github-api crates. The auth crate performs lightweight PAT validation independently
// of the full provider clients, so it has its own deserialization types to avoid
// creating a dependency from auth → gitlab-api/github-api.

/// GitHub user profile as returned by `GET /user`.
///
/// Internal deserialization type — converted to [`ProviderUser`] before returning.
#[derive(serde::Deserialize)]
struct GitHubUserResponse {
    id: u64,
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
    html_url: String,
}

/// GitLab user profile as returned by `GET /api/v4/user`.
///
/// Internal deserialization type — converted to [`ProviderUser`] before returning.
#[derive(serde::Deserialize)]
struct GitLabUserResponse {
    id: u64,
    username: String,
    name: String,
    email: String,
    avatar_url: Option<String>,
    web_url: String,
}

/// Validate a GitLab PAT by calling `GET /api/v4/user` on the given instance.
///
/// # Arguments
/// - `instance_url` — Base URL of the GitLab instance (e.g. `"https://gitlab.com"`).
/// - `token` — Personal Access Token with at least `read_api` scope.
///
/// # Returns
/// The authenticated user's profile as a [`ProviderUser`].
pub async fn validate_gitlab_pat(
    instance_url: &str,
    token: &str,
) -> Result<ProviderUser, AuthError> {
    let url = format!("{}/api/v4/user", instance_url.trim_end_matches('/'));
    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));
    if provider::http_helpers::should_accept_invalid_certs(instance_url) {
        builder = builder.danger_accept_invalid_certs(true);
    }
    let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());

    let response = client
        .get(&url)
        .header("PRIVATE-TOKEN", token)
        .send()
        .await
        .map_err(|e| AuthError::Http(e.to_string()))?;

    if !response.status().is_success() {
        return Err(AuthError::InvalidToken(format!(
            "GitLab returned status {}",
            response.status()
        )));
    }

    let user: GitLabUserResponse = response
        .json()
        .await
        .map_err(|e| AuthError::Http(e.to_string()))?;

    Ok(ProviderUser {
        id: user.id,
        username: user.username,
        display_name: user.name,
        email: Some(user.email),
        avatar_url: user.avatar_url,
        profile_url: user.web_url,
    })
}

/// Validate a GitHub PAT by calling `GET /user` on the GitHub API.
///
/// # Arguments
/// - `instance_url` — Base URL of the GitHub API (e.g. `"https://api.github.com"`).
/// - `token` — Personal Access Token (classic or fine-grained).
///
/// # Returns
/// The authenticated user's profile as a [`ProviderUser`].
/// Note: `email` will be `None` if the user has a private email on GitHub.
pub async fn validate_github_pat(
    instance_url: &str,
    token: &str,
) -> Result<ProviderUser, AuthError> {
    let base = github_api::GitHubClient::normalize_url(instance_url);
    let url = format!("{}/user", base);
    let mut builder = reqwest::Client::builder()
        .user_agent("BeardGit")
        .timeout(std::time::Duration::from_secs(30));
    if provider::http_helpers::should_accept_invalid_certs(&base) {
        builder = builder.danger_accept_invalid_certs(true);
    }
    let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| AuthError::Http(e.to_string()))?;

    if !response.status().is_success() {
        return Err(AuthError::InvalidToken(format!(
            "GitHub returned status {}",
            response.status()
        )));
    }

    let user: GitHubUserResponse = response
        .json()
        .await
        .map_err(|e| AuthError::Http(e.to_string()))?;

    Ok(ProviderUser {
        id: user.id,
        username: user.login,
        display_name: user.name.unwrap_or_default(),
        email: user.email,
        avatar_url: user.avatar_url,
        profile_url: user.html_url,
    })
}
