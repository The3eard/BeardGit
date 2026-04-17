//! Provider kind enum and git remote URL parsing.
//!
//! Contains [`ProviderKind`] (GitLab vs GitHub) and [`parse_remote_url`] which
//! detects the provider from a git remote URL. Handles SSH + HTTPS formats,
//! well-known hosts (`github.com`, `gitlab.com`), and self-hosted instances
//! (matched against a connected provider's base URL).

use serde::{Deserialize, Serialize};

/// Identifies which git hosting provider is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    /// GitLab (cloud or self-hosted).
    GitLab,
    /// GitHub (cloud or GitHub Enterprise).
    GitHub,
}

impl ProviderKind {
    /// Parse a provider kind from a config string.
    ///
    /// Returns `None` for unrecognized strings.
    pub fn from_config_str(s: &str) -> Option<Self> {
        match s {
            "gitlab" => Some(Self::GitLab),
            "github" => Some(Self::GitHub),
            _ => None,
        }
    }

    /// Return the string representation used in config files.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GitLab => "gitlab",
            Self::GitHub => "github",
        }
    }
}

/// Parse a git remote URL to detect the provider and extract the project reference.
///
/// Returns `(ProviderKind, project_ref)` where `project_ref` is:
/// - GitLab: `"group/project"` (used URL-encoded in API calls)
/// - GitHub: `"owner/repo"`
///
/// For well-known hosts (`github.com`, `gitlab.com`), detection is automatic.
/// For self-hosted instances, pass the connected provider's base URL and kind
/// so the parser can match the domain.
///
/// # Examples
///
/// ```
/// use provider::{parse_remote_url, ProviderKind};
///
/// // GitHub SSH
/// let (kind, project) = parse_remote_url(
///     "git@github.com:owner/repo.git", None, None,
/// ).unwrap();
/// assert_eq!(kind, ProviderKind::GitHub);
/// assert_eq!(project, "owner/repo");
///
/// // GitLab HTTPS
/// let (kind, project) = parse_remote_url(
///     "https://gitlab.com/group/project.git", None, None,
/// ).unwrap();
/// assert_eq!(kind, ProviderKind::GitLab);
/// assert_eq!(project, "group/project");
/// ```
pub fn parse_remote_url(
    remote_url: &str,
    provider_base_url: Option<&str>,
    provider_kind: Option<ProviderKind>,
) -> Option<(ProviderKind, String)> {
    // 1. Try well-known hosts
    if let Some(result) = try_well_known_host(remote_url) {
        return Some(result);
    }

    // 2. Try matching against the connected provider's base URL
    if let (Some(base_url), Some(kind)) = (provider_base_url, provider_kind)
        && let Some(path) = try_match_base_url(remote_url, base_url)
    {
        return Some((kind, path));
    }

    None
}

/// Check if the remote URL points to a well-known host (github.com or gitlab.com).
fn try_well_known_host(remote_url: &str) -> Option<(ProviderKind, String)> {
    // SSH format: git@<host>:<path>.git
    if let Some(after_at) = remote_url.strip_prefix("git@")
        && let Some((host, path_with_git)) = after_at.split_once(':')
    {
        let path = path_with_git.trim_end_matches(".git");
        let kind = host_to_kind(host)?;
        return Some((kind, path.to_string()));
    }

    // HTTPS format: https://<host>/<path>.git
    if remote_url.starts_with("http") {
        let without_scheme = remote_url
            .strip_prefix("https://")
            .or_else(|| remote_url.strip_prefix("http://"))?;
        let (host, path_with_slash) = without_scheme.split_once('/')?;
        let kind = host_to_kind(host)?;
        let path = path_with_slash.trim_end_matches(".git");
        return Some((kind, path.to_string()));
    }

    None
}

/// Map a hostname to a well-known provider kind.
fn host_to_kind(host: &str) -> Option<ProviderKind> {
    if host == "github.com" {
        Some(ProviderKind::GitHub)
    } else if host == "gitlab.com" {
        Some(ProviderKind::GitLab)
    } else {
        None
    }
}

/// Try to match a remote URL against a provider's base URL.
///
/// Extracts the project path from both SSH and HTTPS URLs by matching
/// the domain from the base URL.
fn try_match_base_url(remote_url: &str, base_url: &str) -> Option<String> {
    let base_domain = extract_domain(base_url)?;

    // SSH format: git@<domain>:<path>.git
    if let Some(after_at) = remote_url.strip_prefix("git@")
        && let Some((host, path_with_git)) = after_at.split_once(':')
        && host == base_domain
    {
        let path = path_with_git.trim_end_matches(".git");
        return Some(path.to_string());
    }

    // HTTPS format: https://<domain>/<path>.git
    if remote_url.starts_with("http") {
        let base_trimmed = base_url.trim_end_matches('/');
        if let Some(path) = remote_url.strip_prefix(base_trimmed) {
            let path = path.trim_start_matches('/').trim_end_matches(".git");
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
    }

    None
}

/// Extract the domain from a URL (e.g. `"https://gitlab.example.com"` → `"gitlab.example.com"`).
fn extract_domain(url: &str) -> Option<&str> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let domain = without_scheme.split('/').next()?;
    Some(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_kind_from_str() {
        assert_eq!(
            ProviderKind::from_config_str("gitlab"),
            Some(ProviderKind::GitLab)
        );
        assert_eq!(
            ProviderKind::from_config_str("github"),
            Some(ProviderKind::GitHub)
        );
        assert_eq!(ProviderKind::from_config_str("bitbucket"), None);
    }

    #[test]
    fn test_provider_kind_serialization() {
        let json = serde_json::to_string(&ProviderKind::GitHub).unwrap();
        assert_eq!(json, "\"github\"");

        let kind: ProviderKind = serde_json::from_str("\"gitlab\"").unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
    }

    #[test]
    fn test_parse_github_ssh() {
        let (kind, project) =
            parse_remote_url("git@github.com:owner/repo.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitHub);
        assert_eq!(project, "owner/repo");
    }

    #[test]
    fn test_parse_github_https() {
        let (kind, project) =
            parse_remote_url("https://github.com/owner/repo.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitHub);
        assert_eq!(project, "owner/repo");
    }

    #[test]
    fn test_parse_gitlab_ssh() {
        let (kind, project) =
            parse_remote_url("git@gitlab.com:group/project.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "group/project");
    }

    #[test]
    fn test_parse_gitlab_https() {
        let (kind, project) =
            parse_remote_url("https://gitlab.com/group/project.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "group/project");
    }

    #[test]
    fn test_parse_gitlab_https_no_git_suffix() {
        let (kind, project) =
            parse_remote_url("https://gitlab.com/group/project", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "group/project");
    }

    #[test]
    fn test_parse_self_hosted_gitlab_ssh() {
        let result = parse_remote_url(
            "git@gitlab.internal.com:team/app.git",
            Some("https://gitlab.internal.com"),
            Some(ProviderKind::GitLab),
        );
        let (kind, project) = result.unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "team/app");
    }

    #[test]
    fn test_parse_self_hosted_gitlab_https() {
        let result = parse_remote_url(
            "https://gitlab.internal.com/team/app.git",
            Some("https://gitlab.internal.com"),
            Some(ProviderKind::GitLab),
        );
        let (kind, project) = result.unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "team/app");
    }

    #[test]
    fn test_parse_github_enterprise_ssh() {
        let result = parse_remote_url(
            "git@github.enterprise.com:org/repo.git",
            Some("https://github.enterprise.com"),
            Some(ProviderKind::GitHub),
        );
        let (kind, project) = result.unwrap();
        assert_eq!(kind, ProviderKind::GitHub);
        assert_eq!(project, "org/repo");
    }

    #[test]
    fn test_parse_unknown_host_no_base_url() {
        let result = parse_remote_url("git@unknown.com:org/repo.git", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_subgroup_gitlab() {
        let (kind, project) =
            parse_remote_url("git@gitlab.com:group/subgroup/project.git", None, None).unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
        assert_eq!(project, "group/subgroup/project");
    }
}
