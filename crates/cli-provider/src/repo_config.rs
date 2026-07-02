//! Forge-agnostic remote-repository configuration seam.
//!
//! Hosts the shared error/result types, forge detection from a remote URL,
//! the CLI availability probe, and the [`ForgeRepoConfig`] trait that
//! app-core dispatches through. The GitHub/GitLab specifics live in the
//! sibling [`crate::github::repo_config`] / [`crate::gitlab::repo_config`]
//! modules.

use std::path::Path;

use serde::{Deserialize, Serialize};

use forge_provider::{ForgeKind, Label, RemoteRepoConfig, RemoteRepoConfigPatch};

use crate::command_runner::{CliError as RunnerCliError, CommandRunner};
use crate::github::repo_config::{
    apply_github, create_label_github, delete_label_github, load_remote_repo_config_github,
    update_label_github,
};
use crate::gitlab::repo_config::{
    apply_gitlab, create_label_gitlab, delete_label_gitlab, load_remote_repo_config_gitlab,
    update_label_gitlab,
};

// Re-exported so app-core (and tests) can reach the GitHub-only branch
// protection helpers without depending on the `github` submodule path.
pub use crate::github::repo_config::{get_branch_protection_github, set_branch_protection_github};

/// Structured load error exposed to the Tauri boundary.
///
/// The variants are chosen so the frontend can distinguish
/// "CLI missing" / "not authenticated" / "command failed" without
/// having to parse a stringified error — Phase 7 will render
/// different empty states per variant.
#[derive(Debug, thiserror::Error)]
pub enum RepoConfigError {
    /// The `gh` / `glab` binary was not found on `PATH`.
    #[error("CLI binary not found: {0}")]
    CliNotFound(String),
    /// The CLI reported an authentication failure.
    ///
    /// Detected heuristically from stderr text (`gh` writes
    /// "not authenticated", `glab` writes "not logged in").
    #[error("CLI not authenticated: {0}")]
    NotAuthenticated(String),
    /// The CLI exited non-zero for any other reason.
    #[error("CLI command failed: {0}")]
    CommandFailed(String),
    /// The CLI output could not be parsed as expected JSON.
    #[error("JSON parse error: {0}")]
    JsonError(String),
    /// I/O error spawning or reading from the CLI.
    #[error("IO error: {0}")]
    Io(String),
    /// The origin remote did not map to a supported forge.
    #[error("unsupported forge for this repository")]
    UnsupportedForge,
}

impl From<RunnerCliError> for RepoConfigError {
    fn from(err: RunnerCliError) -> Self {
        match err {
            RunnerCliError::NotFound(p) => RepoConfigError::CliNotFound(p),
            RunnerCliError::NonZeroExit {
                stdout: _,
                stderr,
                exit_code,
            } => {
                let lower = stderr.to_ascii_lowercase();
                if lower.contains("not authenticated")
                    || lower.contains("not logged in")
                    || lower.contains("authentication required")
                    || lower.contains("auth token")
                    // `glab` reports auth failures as raw HTTP errors
                    // ("GET …/api/v4/projects/…: 401 {message: 401
                    // Unauthorized}") — e.g. an expired GITLAB_TOKEN env
                    // var shadowing a valid keyring login. Without this
                    // the user gets a cryptic "exit 1" instead of the
                    // authenticate CTA.
                    || lower.contains("401")
                    || lower.contains("unauthorized")
                {
                    RepoConfigError::NotAuthenticated(stderr)
                } else {
                    RepoConfigError::CommandFailed(format!("exit {exit_code}: {stderr}"))
                }
            }
            RunnerCliError::Io(m) => RepoConfigError::Io(m),
        }
    }
}

/// One field of the patch failed to apply.
///
/// Every invocation of `apply_*` collects failures rather than
/// short-circuiting so the UI can tell the user exactly which fields
/// went through and which didn't (`ApplyResult::failures`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
    /// Which patch field the failure relates to (e.g. `"description"`).
    pub field: String,
    /// Human-readable failure reason (usually CLI stderr).
    pub message: String,
}

/// Result of applying a [`RemoteRepoConfigPatch`] to a forge.
///
/// Callers combine `fields_updated` and `failures` to show a mixed
/// "some-succeeded, some-failed" toast. When `failures` is empty the
/// full patch made it through.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyResult {
    /// Names of patch fields that were successfully applied.
    pub fields_updated: Vec<String>,
    /// Per-field failures.
    pub failures: Vec<FieldError>,
}

impl ApplyResult {
    pub(crate) fn record_success(&mut self, field: &str) {
        self.fields_updated.push(field.to_string());
    }

    pub(crate) fn record_failure(&mut self, field: &str, err: impl std::fmt::Display) {
        self.failures.push(FieldError {
            field: field.to_string(),
            message: err.to_string(),
        });
    }
}

/// Helper: detect a forge from a raw remote URL string.
///
/// Exposed separately so unit tests can feed synthetic URLs without
/// needing a real `git2::Repository` on disk.
pub fn detect_forge_from_url(url: &str) -> Option<ForgeKind> {
    let (kind, _path) = provider::parse_remote_url(url, None, None)?;
    Some(match kind {
        provider::ProviderKind::GitHub => ForgeKind::GitHub,
        provider::ProviderKind::GitLab => ForgeKind::GitLab,
    })
}

/// Same as [`detect_forge_from_url`] but also accepts a connected
/// provider's base URL + kind so self-hosted GitHub Enterprise /
/// GitLab instances resolve correctly.
pub fn detect_forge_from_url_with_base(
    url: &str,
    base_url: Option<&str>,
    kind_hint: Option<ForgeKind>,
) -> Option<ForgeKind> {
    let provider_kind = kind_hint.map(|k| match k {
        ForgeKind::GitHub => provider::ProviderKind::GitHub,
        ForgeKind::GitLab => provider::ProviderKind::GitLab,
    });
    let (parsed, _) = provider::parse_remote_url(url, base_url, provider_kind)?;
    Some(match parsed {
        provider::ProviderKind::GitHub => ForgeKind::GitHub,
        provider::ProviderKind::GitLab => ForgeKind::GitLab,
    })
}

/// Pull the hostname out of a git remote URL.
///
/// Supports both SSH (`git@host:path.git`) and HTTPS
/// (`https://host/path[.git]`) forms. Returns `None` for shapes we don't
/// recognise (local paths, custom schemes).
pub fn extract_remote_host(url: &str) -> Option<String> {
    if let Some(after_at) = url.strip_prefix("git@")
        && let Some((host, _)) = after_at.split_once(':')
        && !host.is_empty()
    {
        return Some(host.to_string());
    }
    if url.starts_with("http") {
        let without_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))?;
        let host = without_scheme.split('/').next()?;
        if !host.is_empty() {
            return Some(host.to_string());
        }
    }
    None
}

/// Result of probing the forge CLI availability + auth state for a
/// repository.
///
/// The frontend uses this tagged enum to choose between three empty
/// states before rendering the repo-config dialog body:
///
///   - `Installed { authenticated: true, .. }` → render the dialog.
///   - `Installed { authenticated: false, .. }` → "sign in" state with
///     a deep-link to Settings → Integrations.
///   - `NotInstalled` → "install gh/glab" state.
///   - `UnsupportedForge` → neither GitHub nor GitLab, so we render
///     a graceful "not supported" card.
///
/// The serde representation is `tag = "kind"`, `rename_all =
/// "snake_case"` so the wire form is
/// `{ "kind": "installed", "authenticated": true, "account": "octocat" }`,
/// `{ "kind": "not_installed" }`, or
/// `{ "kind": "unsupported_forge" }` — which the TS mirror in
/// `src/lib/types/repoConfig.ts` matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ForgeCliStatus {
    /// The CLI binary was found on `PATH` and returned a version.
    Installed {
        /// `true` when `gh auth status` / `glab auth status` succeeded.
        authenticated: bool,
        /// Best-effort extracted account name ("octocat", etc.).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account: Option<String>,
    },
    /// The CLI binary is missing from `PATH`.
    NotInstalled,
    /// Repository's `origin` remote is neither GitHub nor GitLab.
    UnsupportedForge,
}

/// Pick the CLI binary name for a forge.
fn cli_binary(forge: ForgeKind) -> &'static str {
    match forge {
        ForgeKind::GitHub => "gh",
        ForgeKind::GitLab => "glab",
    }
}

/// Try to extract an account name from `gh auth status` / `glab auth
/// status` stdout. Best-effort — returns `None` if the output does not
/// match the expected patterns, which is fine: the UI only uses the
/// account for a friendly "Signed in as …" hint.
fn extract_account_from_status(output: &str) -> Option<String> {
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        // `gh` prints: "  ✓ Logged in to github.com as octocat (…)"
        if let Some(idx) = lower.find(" as ") {
            let rest = &line[idx + 4..];
            let name: String = rest
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '(')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
        // `glab` prints: "   Logged in as octocat at gitlab.com"
        if let Some(stripped) = lower.strip_prefix("logged in as ") {
            let name: String = stripped
                .chars()
                .take_while(|c| !c.is_whitespace())
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Pure probe implementation exposed for unit tests.
///
/// Runs `<cli> --version` followed by `<cli> auth status` through the
/// injected [`CommandRunner`]. Any [`CliError::NotFound`] on the first
/// call collapses to `NotInstalled`; everything else is a successful
/// probe that may or may not be authenticated.
///
/// When `host` is `Some`, the auth check is scoped to that host via
/// `--hostname <host>`. This is critical for multi-instance configs:
/// `gh`/`glab auth status` without a host filter exits non-zero if *any*
/// configured instance is broken, even when the host the repo actually
/// uses is fully authenticated. Scoping ensures we only flag auth-required
/// when the host this repo points at is actually broken.
pub fn probe_forge_cli_status_with<R: CommandRunner + ?Sized>(
    runner: &R,
    forge: Option<ForgeKind>,
    host: Option<&str>,
    repo_path: &Path,
) -> ForgeCliStatus {
    let Some(forge) = forge else {
        return ForgeCliStatus::UnsupportedForge;
    };
    let bin = cli_binary(forge);
    // `--version` is the cheapest way to confirm the binary exists.
    match runner.run(bin, &["--version"], repo_path) {
        Ok(_) => {}
        Err(RunnerCliError::NotFound(_)) => return ForgeCliStatus::NotInstalled,
        // Any other failure of `--version` still means the binary
        // resolved — treat it as installed-but-broken and let the auth
        // probe decide the outcome.
        Err(_) => {}
    }

    let auth_args: Vec<&str> = match host {
        Some(h) => vec!["auth", "status", "--hostname", h],
        None => vec!["auth", "status"],
    };
    match runner.run(bin, &auth_args, repo_path) {
        Ok(out) => {
            let combined = format!("{}\n{}", out.stdout, out.stderr);
            ForgeCliStatus::Installed {
                authenticated: true,
                account: extract_account_from_status(&combined),
            }
        }
        Err(RunnerCliError::NonZeroExit { stdout, stderr, .. }) => {
            let combined = format!("{stdout}\n{stderr}");
            ForgeCliStatus::Installed {
                authenticated: false,
                account: extract_account_from_status(&combined),
            }
        }
        Err(RunnerCliError::NotFound(_)) => ForgeCliStatus::NotInstalled,
        Err(RunnerCliError::Io(_)) => ForgeCliStatus::Installed {
            authenticated: false,
            account: None,
        },
    }
}

/// Dispatch loader choice + invocation for any [`CommandRunner`].
///
/// Extracted as a plain function so unit tests can drive it with
/// [`MockRunner`] and a pre-detected [`ForgeKind`] without needing
/// the Tauri runtime.
pub fn load_remote_repo_config_with<R: CommandRunner + ?Sized>(
    runner: &R,
    forge: ForgeKind,
    repo_path: &Path,
) -> Result<RemoteRepoConfig, RepoConfigError> {
    match forge {
        ForgeKind::GitHub => load_remote_repo_config_github(runner, repo_path),
        ForgeKind::GitLab => load_remote_repo_config_gitlab(runner, repo_path),
    }
}

/// Dispatch apply choice + invocation for any [`CommandRunner`].
///
/// Picks [`apply_github`] or [`apply_gitlab`] based on the detected
/// [`ForgeKind`]. On GitLab, `current_topics` is required to compute
/// the full `--topics` replacement list; on GitHub it is ignored.
pub fn apply_remote_repo_config_with<R: CommandRunner + ?Sized>(
    runner: &R,
    forge: ForgeKind,
    repo_path: &Path,
    patch: &RemoteRepoConfigPatch,
    current_topics: &[String],
) -> ApplyResult {
    match forge {
        ForgeKind::GitHub => apply_github(runner, repo_path, patch),
        ForgeKind::GitLab => apply_gitlab(runner, repo_path, patch, current_topics),
    }
}

/// Remote-repository configuration operations for a single forge, over an
/// injected [`CommandRunner`]. Lets app-core dispatch load / apply / label
/// CRUD through one seam instead of matching on [`ForgeKind`] at each call.
pub trait ForgeRepoConfig {
    /// Load the current remote configuration.
    fn load(&self, repo_path: &Path) -> Result<RemoteRepoConfig, RepoConfigError>;
    /// Apply a patch, collecting per-field failures. `current_topics` is only
    /// consulted on GitLab (which replaces the whole topic list).
    fn apply(
        &self,
        repo_path: &Path,
        patch: &RemoteRepoConfigPatch,
        current_topics: &[String],
    ) -> ApplyResult;
    /// Create a repository label.
    fn create_label(&self, repo_path: &Path, label: &Label) -> Result<(), RepoConfigError>;
    /// Update a repository label (renaming when `old_name` differs).
    fn update_label(
        &self,
        repo_path: &Path,
        old_name: &str,
        label: &Label,
    ) -> Result<(), RepoConfigError>;
    /// Delete a repository label by name.
    fn delete_label(&self, repo_path: &Path, name: &str) -> Result<(), RepoConfigError>;
}

/// CLI-backed [`ForgeRepoConfig`] pairing a [`CommandRunner`] with the forge it
/// targets. The runner is `SystemRunner` in production and `MockRunner` in
/// tests.
pub struct CliForgeRepoConfig<R: CommandRunner> {
    runner: R,
    kind: ForgeKind,
}

impl<R: CommandRunner> CliForgeRepoConfig<R> {
    /// Pair a runner with the forge it should drive.
    pub fn new(runner: R, kind: ForgeKind) -> Self {
        Self { runner, kind }
    }
}

impl<R: CommandRunner> ForgeRepoConfig for CliForgeRepoConfig<R> {
    fn load(&self, repo_path: &Path) -> Result<RemoteRepoConfig, RepoConfigError> {
        load_remote_repo_config_with(&self.runner, self.kind, repo_path)
    }

    fn apply(
        &self,
        repo_path: &Path,
        patch: &RemoteRepoConfigPatch,
        current_topics: &[String],
    ) -> ApplyResult {
        apply_remote_repo_config_with(&self.runner, self.kind, repo_path, patch, current_topics)
    }

    fn create_label(&self, repo_path: &Path, label: &Label) -> Result<(), RepoConfigError> {
        match self.kind {
            ForgeKind::GitHub => create_label_github(&self.runner, repo_path, label),
            ForgeKind::GitLab => create_label_gitlab(&self.runner, repo_path, label),
        }
    }

    fn update_label(
        &self,
        repo_path: &Path,
        old_name: &str,
        label: &Label,
    ) -> Result<(), RepoConfigError> {
        match self.kind {
            ForgeKind::GitHub => update_label_github(&self.runner, repo_path, old_name, label),
            ForgeKind::GitLab => update_label_gitlab(&self.runner, repo_path, old_name, label),
        }
    }

    fn delete_label(&self, repo_path: &Path, name: &str) -> Result<(), RepoConfigError> {
        match self.kind {
            ForgeKind::GitHub => delete_label_github(&self.runner, repo_path, name),
            ForgeKind::GitLab => delete_label_gitlab(&self.runner, repo_path, name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_runner::{CliOutput, MockRunner};
    use crate::github::repo_config::*;
    use crate::gitlab::repo_config::*;
    use forge_provider::{
        BranchProtection, ForgeKind, Label, PatchValue, RemoteRepoConfig, RemoteRepoConfigPatch,
        Visibility, diff_config,
    };
    use std::path::Path;

    #[test]
    fn github_ssh_with_git_suffix() {
        let k = detect_forge_from_url("git@github.com:owner/repo.git");
        assert_eq!(k, Some(ForgeKind::GitHub));
    }

    #[test]
    fn github_ssh_without_git_suffix() {
        let k = detect_forge_from_url("git@github.com:owner/repo");
        assert_eq!(k, Some(ForgeKind::GitHub));
    }

    #[test]
    fn github_https_with_git_suffix() {
        let k = detect_forge_from_url("https://github.com/owner/repo.git");
        assert_eq!(k, Some(ForgeKind::GitHub));
    }

    #[test]
    fn github_https_without_git_suffix() {
        let k = detect_forge_from_url("https://github.com/owner/repo");
        assert_eq!(k, Some(ForgeKind::GitHub));
    }

    #[test]
    fn gitlab_ssh_with_git_suffix() {
        let k = detect_forge_from_url("git@gitlab.com:group/project.git");
        assert_eq!(k, Some(ForgeKind::GitLab));
    }

    #[test]
    fn gitlab_https_subgroups() {
        let k = detect_forge_from_url("https://gitlab.com/group/subgroup/project.git");
        assert_eq!(k, Some(ForgeKind::GitLab));
    }

    #[test]
    fn unknown_host_returns_none() {
        let k = detect_forge_from_url("git@bitbucket.org:team/repo.git");
        assert!(k.is_none());
    }

    #[test]
    fn unknown_host_self_hosted_without_hint_returns_none() {
        let k = detect_forge_from_url("https://git.internal.example/team/app.git");
        assert!(k.is_none());
    }

    #[test]
    fn self_hosted_gitlab_with_base_url_hint() {
        let k = detect_forge_from_url_with_base(
            "git@gitlab.internal.com:team/app.git",
            Some("https://gitlab.internal.com"),
            Some(ForgeKind::GitLab),
        );
        assert_eq!(k, Some(ForgeKind::GitLab));
    }

    #[test]
    fn github_enterprise_with_base_url_hint() {
        let k = detect_forge_from_url_with_base(
            "https://ghe.example.com/org/repo.git",
            Some("https://ghe.example.com"),
            Some(ForgeKind::GitHub),
        );
        assert_eq!(k, Some(ForgeKind::GitHub));
    }

    #[test]
    fn empty_url_returns_none() {
        assert!(detect_forge_from_url("").is_none());
    }

    // ─── Data-model tests ──────────────────────────────────────────────

    #[test]
    fn visibility_roundtrips_cli_string() {
        assert_eq!(Visibility::Public.as_cli_str(), "public");
        assert_eq!(Visibility::Private.as_cli_str(), "private");
        assert_eq!(Visibility::Internal.as_cli_str(), "internal");
        assert_eq!(Visibility::from_cli_str("public"), Some(Visibility::Public));
        assert_eq!(
            Visibility::from_cli_str("PRIVATE"),
            Some(Visibility::Private)
        );
        assert_eq!(
            Visibility::from_cli_str("internal"),
            Some(Visibility::Internal)
        );
        assert!(Visibility::from_cli_str("bogus").is_none());
    }

    #[test]
    fn visibility_serializes_lowercase_json() {
        assert_eq!(
            serde_json::to_string(&Visibility::Public).unwrap(),
            "\"public\""
        );
        let parsed: Visibility = serde_json::from_str("\"private\"").unwrap();
        assert_eq!(parsed, Visibility::Private);
    }

    #[test]
    fn label_roundtrips_json() {
        let l = Label {
            name: "bug".into(),
            color: Some("ff0000".into()),
            description: Some("Something broken".into()),
        };
        let json = serde_json::to_string(&l).unwrap();
        let back: Label = serde_json::from_str(&json).unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn remote_repo_config_roundtrips_snake_case_json() {
        let cfg = RemoteRepoConfig {
            description: "A fine project".into(),
            homepage: Some("https://example.com".into()),
            topics: vec!["rust".into(), "cli".into()],
            visibility: Visibility::Public,
            default_branch: "main".into(),
            issues_enabled: true,
            wiki_enabled: false,
            archived: false,
            branch_protection: None,
            labels: vec![Label {
                name: "bug".into(),
                color: Some("ff0000".into()),
                description: None,
            }],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"default_branch\":\"main\""));
        assert!(json.contains("\"issues_enabled\":true"));
        assert!(json.contains("\"wiki_enabled\":false"));
        assert!(json.contains("\"branch_protection\":null"));
        let back: RemoteRepoConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cfg);
    }

    // ─── GitHub load tests ─────────────────────────────────────────────

    fn gh_view_json_happy() -> &'static str {
        r#"{
            "description": "A neat little repo",
            "homepageUrl": "https://example.com",
            "repositoryTopics": [
                {"name": "rust"},
                {"name": "cli"}
            ],
            "visibility": "PUBLIC",
            "defaultBranchRef": {"name": "main"},
            "hasIssuesEnabled": true,
            "hasWikiEnabled": false,
            "isArchived": false
        }"#
    }

    fn gh_labels_json() -> &'static str {
        r#"[
            {"name": "bug", "color": "d73a4a", "description": "Something broken"},
            {"name": "enhancement", "color": "a2eeef", "description": null}
        ]"#
    }

    #[test]
    fn load_remote_repo_config_github_parses_happy_path() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: gh_view_json_happy().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &["label", "list"],
            Ok(CliOutput {
                stdout: gh_labels_json().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );

        let cfg = load_remote_repo_config_github(&runner, Path::new("/tmp/repo")).expect("load ok");

        assert_eq!(cfg.description, "A neat little repo");
        assert_eq!(cfg.homepage.as_deref(), Some("https://example.com"));
        assert_eq!(cfg.topics, vec!["rust".to_string(), "cli".to_string()]);
        assert_eq!(cfg.visibility, Visibility::Public);
        assert_eq!(cfg.default_branch, "main");
        assert!(cfg.issues_enabled);
        assert!(!cfg.wiki_enabled);
        assert!(!cfg.archived);
        assert!(cfg.branch_protection.is_none());
        assert_eq!(cfg.labels.len(), 2);
        assert_eq!(cfg.labels[0].name, "bug");
        assert_eq!(cfg.labels[0].color.as_deref(), Some("d73a4a"));
        assert_eq!(cfg.labels[1].name, "enhancement");
        assert!(cfg.labels[1].description.is_none());

        // Argv safety: exact flags must be passed per-argument.
        assert!(runner.was_called_with("gh", &["repo", "view", "--json", GH_REPO_VIEW_FIELDS,],));
        assert!(runner.was_called_with(
            "gh",
            &[
                "label",
                "list",
                "--json",
                "name,color,description",
                "--limit",
                "200",
            ],
        ));
    }

    #[test]
    fn load_github_handles_null_topics() {
        // Regression: `gh repo view --json repositoryTopics` returns
        // `"repositoryTopics": null` (not `[]`) for repos with no
        // topics. Serde's `#[serde(default)]` only covers the MISSING
        // case, so the field must deserialise into `Option<Vec<…>>`.
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: r#"{
                    "description": "x",
                    "homepageUrl": "",
                    "repositoryTopics": null,
                    "visibility": "PUBLIC",
                    "defaultBranchRef": {"name": "main"},
                    "hasIssuesEnabled": true,
                    "hasWikiEnabled": true,
                    "isArchived": false
                }"#
                .into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_github(&runner, Path::new(".")).unwrap();
        assert!(cfg.topics.is_empty());
        assert!(cfg.homepage.is_none());
        assert_eq!(cfg.visibility, Visibility::Public);
    }

    #[test]
    fn load_github_handles_empty_topics() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: r#"{
                    "description": "x",
                    "homepageUrl": null,
                    "repositoryTopics": [],
                    "visibility": "PRIVATE",
                    "defaultBranchRef": {"name": "main"},
                    "hasIssuesEnabled": false,
                    "hasWikiEnabled": false,
                    "isArchived": false
                }"#
                .into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_github(&runner, Path::new(".")).unwrap();
        assert!(cfg.topics.is_empty());
        assert!(cfg.labels.is_empty());
        assert!(cfg.homepage.is_none());
        assert_eq!(cfg.visibility, Visibility::Private);
    }

    #[test]
    fn load_github_missing_homepage_is_none() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: r#"{
                    "description": "",
                    "homepageUrl": "",
                    "repositoryTopics": [],
                    "visibility": "public",
                    "defaultBranchRef": {"name": "main"},
                    "hasIssuesEnabled": true,
                    "hasWikiEnabled": true,
                    "isArchived": false
                }"#
                .into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_github(&runner, Path::new(".")).unwrap();
        // Empty string homepage is surfaced as None.
        assert!(cfg.homepage.is_none());
    }

    #[test]
    fn load_github_no_default_branch_is_empty_string() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: r#"{
                    "description": "",
                    "homepageUrl": null,
                    "repositoryTopics": [],
                    "visibility": "public",
                    "defaultBranchRef": null,
                    "hasIssuesEnabled": true,
                    "hasWikiEnabled": true,
                    "isArchived": false
                }"#
                .into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_github(&runner, Path::new(".")).unwrap();
        assert_eq!(cfg.default_branch, "");
    }

    #[test]
    fn load_github_maps_auth_failure_to_structured_error() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Err(crate::command_runner::CliError::NonZeroExit {
                exit_code: 4,
                stdout: String::new(),
                stderr: "gh: not authenticated. Run gh auth login.".into(),
            }),
        );
        let err = load_remote_repo_config_github(&runner, Path::new(".")).unwrap_err();
        assert!(matches!(err, RepoConfigError::NotAuthenticated(_)));
    }

    #[test]
    fn glab_http_401_maps_to_not_authenticated() {
        // glab reports auth failures as raw HTTP errors (e.g. an expired
        // GITLAB_TOKEN env var shadowing a valid keyring login) — these
        // must surface the authenticate CTA, not a cryptic exit-1 error.
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["repo", "view", "-F", "json"],
            Err(crate::command_runner::CliError::NonZeroExit {
                exit_code: 1,
                stdout: String::new(),
                stderr: "ERROR Get https://gitlab.com/api/v4/projects/x%2Fy: \
                         401 {message: 401 Unauthorized}."
                    .into(),
            }),
        );
        let err = load_remote_repo_config_gitlab(&runner, Path::new(".")).unwrap_err();
        assert!(matches!(err, RepoConfigError::NotAuthenticated(_)));
    }

    #[test]
    fn load_github_maps_cli_missing_to_structured_error() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Err(crate::command_runner::CliError::NotFound("gh".into())),
        );
        let err = load_remote_repo_config_github(&runner, Path::new(".")).unwrap_err();
        assert!(matches!(err, RepoConfigError::CliNotFound(_)));
    }

    #[test]
    fn load_labels_github_parses_canned_output() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["label", "list"],
            Ok(CliOutput {
                stdout: gh_labels_json().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let labels = load_labels_github(&runner, Path::new(".")).unwrap();
        assert_eq!(labels.len(), 2);
    }

    // ─── GitLab load tests ─────────────────────────────────────────────

    fn glab_view_json_happy() -> &'static str {
        r#"{
            "description": "A GitLab repo",
            "homepage": "https://example.com",
            "web_url": "https://gitlab.com/group/project",
            "topics": ["rust", "cli"],
            "visibility": "public",
            "default_branch": "main",
            "issues_access_level": "enabled",
            "wiki_access_level": "enabled",
            "archived": false
        }"#
    }

    fn glab_labels_json() -> &'static str {
        r##"[
            {"name": "bug", "color": "#d73a4a", "description": "Something broken"},
            {"name": "enhancement", "color": "a2eeef", "description": null}
        ]"##
    }

    #[test]
    fn load_remote_repo_config_gitlab_parses_happy_path() {
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: glab_view_json_happy().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "glab",
            &["label", "list"],
            Ok(CliOutput {
                stdout: glab_labels_json().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );

        let cfg = load_remote_repo_config_gitlab(&runner, Path::new(".")).unwrap();
        assert_eq!(cfg.description, "A GitLab repo");
        assert_eq!(cfg.homepage.as_deref(), Some("https://example.com"));
        assert_eq!(cfg.topics, vec!["rust".to_string(), "cli".to_string()]);
        assert_eq!(cfg.visibility, Visibility::Public);
        assert_eq!(cfg.default_branch, "main");
        assert!(cfg.issues_enabled);
        assert!(cfg.wiki_enabled);
        assert!(!cfg.archived);
        assert!(cfg.branch_protection.is_none());
        assert_eq!(cfg.labels.len(), 2);
        // Leading '#' in the color is stripped so both forges agree.
        assert_eq!(cfg.labels[0].color.as_deref(), Some("d73a4a"));

        // Argv safety: exact flags are per-argument.
        assert!(runner.was_called_with("glab", &["repo", "view", "-F", "json"]));
        assert!(runner.was_called_with(
            "glab",
            &["label", "list", "--per-page", "200", "-F", "json"],
        ));
    }

    #[test]
    fn load_gitlab_empty_topics_and_no_homepage_falls_back_to_web_url() {
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: r#"{
                    "description": "x",
                    "web_url": "https://gitlab.com/group/project",
                    "topics": [],
                    "visibility": "private",
                    "default_branch": "main",
                    "issues_access_level": "disabled",
                    "wiki_access_level": "disabled",
                    "archived": false
                }"#
                .into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "glab",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_gitlab(&runner, Path::new(".")).unwrap();
        assert!(cfg.topics.is_empty());
        assert_eq!(
            cfg.homepage.as_deref(),
            Some("https://gitlab.com/group/project")
        );
        assert!(!cfg.issues_enabled);
        assert!(!cfg.wiki_enabled);
        assert_eq!(cfg.visibility, Visibility::Private);
    }

    #[test]
    fn load_gitlab_no_default_branch_is_empty_string() {
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: r#"{
                    "description": "",
                    "topics": [],
                    "visibility": "public",
                    "archived": false
                }"#
                .into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "glab",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_gitlab(&runner, Path::new(".")).unwrap();
        assert_eq!(cfg.default_branch, "");
        // Missing access-level fields default to enabled.
        assert!(cfg.issues_enabled);
        assert!(cfg.wiki_enabled);
    }

    #[test]
    fn load_gitlab_maps_not_logged_in_to_auth_error() {
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["repo", "view"],
            Err(crate::command_runner::CliError::NonZeroExit {
                exit_code: 1,
                stdout: String::new(),
                stderr: "Error: not logged in. run 'glab auth login'.".into(),
            }),
        );
        let err = load_remote_repo_config_gitlab(&runner, Path::new(".")).unwrap_err();
        assert!(matches!(err, RepoConfigError::NotAuthenticated(_)));
    }

    #[test]
    fn load_gitlab_accepts_payload_with_both_topics_and_tag_list() {
        // Modern GitLab emits BOTH `topics` (canonical) and `tag_list`
        // (deprecated alias) in the same payload. A previous
        // `#[serde(alias = "tag_list")]` on the Rust struct surfaced as
        // "duplicate field `topics`" because serde maps the alias to
        // the same struct field. We rely on `topics` only.
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: r#"{
                    "description": "dual-field repo",
                    "web_url": "https://gitlab.com/g/p",
                    "topics": ["rust", "cli"],
                    "tag_list": ["rust", "cli"],
                    "visibility": "public",
                    "default_branch": "main",
                    "issues_access_level": "enabled",
                    "wiki_access_level": "enabled",
                    "archived": false
                }"#
                .into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "glab",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_gitlab(&runner, Path::new(".")).unwrap();
        assert_eq!(cfg.topics, vec!["rust".to_string(), "cli".to_string()]);
    }

    #[test]
    fn load_labels_gitlab_strips_color_hash() {
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["label", "list"],
            Ok(CliOutput {
                stdout: glab_labels_json().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let labels = load_labels_gitlab(&runner, Path::new(".")).unwrap();
        assert_eq!(labels.len(), 2);
        // `#ff0000` → `ff0000`, bare hex is unchanged.
        assert_eq!(labels[0].color.as_deref(), Some("d73a4a"));
        assert_eq!(labels[1].color.as_deref(), Some("a2eeef"));
    }

    // ─── Diff tests ────────────────────────────────────────────────────

    fn sample_cfg() -> RemoteRepoConfig {
        RemoteRepoConfig {
            description: "initial".into(),
            homepage: Some("https://example.com".into()),
            topics: vec!["rust".into(), "cli".into()],
            visibility: Visibility::Public,
            default_branch: "main".into(),
            issues_enabled: true,
            wiki_enabled: true,
            archived: false,
            branch_protection: None,
            labels: vec![],
        }
    }

    #[test]
    fn diff_noop_returns_empty_patch() {
        let before = sample_cfg();
        let after = before.clone();
        let patch = diff_config(&before, &after);
        assert!(patch.is_empty());
    }

    #[test]
    fn diff_description_change_only() {
        let before = sample_cfg();
        let mut after = before.clone();
        after.description = "updated".into();
        let patch = diff_config(&before, &after);
        assert_eq!(patch.description.as_deref(), Some("updated"));
        assert!(patch.topics_added.is_empty());
        assert!(patch.topics_removed.is_empty());
        assert!(patch.homepage.is_unchanged());
    }

    #[test]
    fn diff_homepage_cleared_vs_unchanged() {
        let before = sample_cfg();
        // Clear: Some("…") → None.
        let mut after = before.clone();
        after.homepage = None;
        let patch = diff_config(&before, &after);
        assert_eq!(patch.homepage, PatchValue::Clear);

        // No change.
        let patch2 = diff_config(&before, &before);
        assert!(patch2.homepage.is_unchanged());

        // Set to a new value.
        let mut after3 = before.clone();
        after3.homepage = Some("https://new.example.com".into());
        let patch3 = diff_config(&before, &after3);
        assert_eq!(
            patch3.homepage,
            PatchValue::Set("https://new.example.com".into())
        );
    }

    #[test]
    fn diff_topic_add_and_remove_are_sorted_sets() {
        let before = sample_cfg(); // rust, cli
        let mut after = before.clone();
        after.topics = vec!["cli".into(), "tauri".into(), "svelte".into()];
        let patch = diff_config(&before, &after);
        assert_eq!(patch.topics_removed, vec!["rust".to_string()]);
        // BTreeSet ordering: svelte < tauri.
        assert_eq!(
            patch.topics_added,
            vec!["svelte".to_string(), "tauri".to_string()]
        );
    }

    #[test]
    fn diff_visibility_default_branch_and_toggles() {
        let before = sample_cfg();
        let mut after = before.clone();
        after.visibility = Visibility::Private;
        after.default_branch = "trunk".into();
        after.issues_enabled = false;
        after.wiki_enabled = false;
        after.archived = true;
        let patch = diff_config(&before, &after);
        assert_eq!(patch.visibility, Some(Visibility::Private));
        assert_eq!(patch.default_branch.as_deref(), Some("trunk"));
        assert_eq!(patch.issues_enabled, Some(false));
        assert_eq!(patch.wiki_enabled, Some(false));
        assert_eq!(patch.archive, Some(true));
    }

    #[test]
    fn patch_is_empty_predicate() {
        let p = RemoteRepoConfigPatch::default();
        assert!(p.is_empty());
        let p2 = RemoteRepoConfigPatch {
            description: Some("x".into()),
            ..Default::default()
        };
        assert!(!p2.is_empty());
    }

    #[test]
    fn patch_roundtrips_json_with_tristate_homepage() {
        let p = RemoteRepoConfigPatch {
            description: Some("x".into()),
            homepage: PatchValue::Clear,
            topics_added: vec!["a".into()],
            topics_removed: vec!["b".into()],
            visibility: Some(Visibility::Private),
            default_branch: Some("trunk".into()),
            issues_enabled: Some(false),
            wiki_enabled: Some(true),
            archive: Some(true),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: RemoteRepoConfigPatch = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);

        // Set variant round-trips too.
        let p2 = RemoteRepoConfigPatch {
            homepage: PatchValue::Set("https://ex".into()),
            ..Default::default()
        };
        let json2 = serde_json::to_string(&p2).unwrap();
        let back2: RemoteRepoConfigPatch = serde_json::from_str(&json2).unwrap();
        assert_eq!(back2, p2);
    }

    // ─── Apply-GitHub tests ────────────────────────────────────────────

    fn ok_output() -> CliOutput {
        CliOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }
    }

    #[test]
    fn apply_github_empty_patch_makes_no_calls() {
        let runner = MockRunner::new();
        let patch = RemoteRepoConfigPatch::default();
        let r = apply_github(&runner, Path::new("/tmp"), &patch);
        assert!(r.fields_updated.is_empty());
        assert!(r.failures.is_empty());
        assert!(runner.calls().is_empty());
    }

    #[test]
    fn apply_github_description_passes_exact_argv() {
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            description: Some("new description".into()),
            ..Default::default()
        };
        let r = apply_github(&runner, Path::new("."), &patch);
        assert_eq!(r.fields_updated, vec!["description".to_string()]);
        assert!(r.failures.is_empty());
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].args,
            vec!["repo", "edit", "--description", "new description"]
        );
    }

    #[test]
    fn apply_github_homepage_clear_uses_empty_string_argument() {
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            homepage: PatchValue::Clear,
            ..Default::default()
        };
        let _ = apply_github(&runner, Path::new("."), &patch);
        let calls = runner.calls();
        assert_eq!(calls[0].args, vec!["repo", "edit", "--homepage", ""]);
    }

    #[test]
    fn apply_github_homepage_set_passes_url() {
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            homepage: PatchValue::Set("https://example.com".into()),
            ..Default::default()
        };
        let _ = apply_github(&runner, Path::new("."), &patch);
        assert!(
            runner.was_called_with("gh", &["repo", "edit", "--homepage", "https://example.com"],)
        );
    }

    #[test]
    fn apply_github_topics_added_and_removed_emit_per_argument_flags() {
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            topics_added: vec!["rust".into(), "cli".into()],
            topics_removed: vec!["legacy".into()],
            ..Default::default()
        };
        let _ = apply_github(&runner, Path::new("."), &patch);
        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[0].args,
            vec!["repo", "edit", "--add-topic", "rust", "--add-topic", "cli",]
        );
        assert_eq!(
            calls[1].args,
            vec!["repo", "edit", "--remove-topic", "legacy"]
        );
    }

    #[test]
    fn apply_github_shell_metacharacters_pass_literally_as_one_argument() {
        // Regression: a malicious topic containing `; rm -rf /` must be
        // passed as a single argv entry, never interpreted by a shell.
        // The mock records argv verbatim — the fact that the dangerous
        // string survives as ONE argument (no splitting, no escaping)
        // is the shell-injection safety proof.
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            topics_added: vec!["x; rm -rf /".into()],
            ..Default::default()
        };
        let _ = apply_github(&runner, Path::new("."), &patch);
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].args,
            vec!["repo", "edit", "--add-topic", "x; rm -rf /"]
        );
    }

    #[test]
    fn apply_github_visibility_default_branch_and_toggles() {
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            visibility: Some(Visibility::Private),
            default_branch: Some("trunk".into()),
            issues_enabled: Some(false),
            wiki_enabled: Some(true),
            ..Default::default()
        };
        let r = apply_github(&runner, Path::new("."), &patch);
        assert!(r.failures.is_empty());
        assert_eq!(r.fields_updated.len(), 4);
        assert!(runner.was_called_with("gh", &["repo", "edit", "--visibility", "private"]));
        assert!(runner.was_called_with("gh", &["repo", "edit", "--default-branch", "trunk"]));
        assert!(runner.was_called_with("gh", &["repo", "edit", "--enable-issues", "false"]));
        assert!(runner.was_called_with("gh", &["repo", "edit", "--enable-wiki", "true"]));
    }

    #[test]
    fn apply_github_archive_true_calls_repo_archive() {
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "archive"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            archive: Some(true),
            ..Default::default()
        };
        let _ = apply_github(&runner, Path::new("."), &patch);
        assert!(runner.was_called_with("gh", &["repo", "archive", "--yes"]));
    }

    #[test]
    fn apply_github_archive_false_calls_repo_unarchive() {
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "unarchive"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            archive: Some(false),
            ..Default::default()
        };
        let _ = apply_github(&runner, Path::new("."), &patch);
        assert!(runner.was_called_with("gh", &["repo", "unarchive", "--yes"]));
    }

    #[test]
    fn apply_github_collects_partial_failures() {
        let runner = MockRunner::new();
        // description succeeds, visibility fails.
        runner.expect("gh", &["repo", "edit", "--description"], Ok(ok_output()));
        runner.expect(
            "gh",
            &["repo", "edit", "--visibility"],
            Err(crate::command_runner::CliError::NonZeroExit {
                exit_code: 1,
                stdout: String::new(),
                stderr: "insufficient permissions".into(),
            }),
        );
        let patch = RemoteRepoConfigPatch {
            description: Some("x".into()),
            visibility: Some(Visibility::Private),
            ..Default::default()
        };
        let r = apply_github(&runner, Path::new("."), &patch);
        assert_eq!(r.fields_updated, vec!["description".to_string()]);
        assert_eq!(r.failures.len(), 1);
        assert_eq!(r.failures[0].field, "visibility");
        assert!(r.failures[0].message.contains("insufficient permissions"));
    }

    // ─── Apply-GitLab tests ────────────────────────────────────────────

    #[test]
    fn apply_gitlab_empty_patch_makes_no_calls() {
        let runner = MockRunner::new();
        let patch = RemoteRepoConfigPatch::default();
        let r = apply_gitlab(&runner, Path::new("."), &patch, &[]);
        assert!(r.fields_updated.is_empty());
        assert!(r.failures.is_empty());
        assert!(runner.calls().is_empty());
    }

    #[test]
    fn apply_gitlab_description_passes_exact_argv() {
        let runner = MockRunner::new();
        runner.expect("glab", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            description: Some("hi".into()),
            ..Default::default()
        };
        let _ = apply_gitlab(&runner, Path::new("."), &patch, &[]);
        assert!(runner.was_called_with("glab", &["repo", "edit", "--description", "hi"]));
    }

    #[test]
    fn apply_gitlab_topics_merge_emits_single_comma_joined_flag() {
        let runner = MockRunner::new();
        runner.expect("glab", &["repo", "edit"], Ok(ok_output()));
        let current = vec!["rust".to_string(), "legacy".to_string()];
        let patch = RemoteRepoConfigPatch {
            topics_added: vec!["tauri".into(), "cli".into()],
            topics_removed: vec!["legacy".into()],
            ..Default::default()
        };
        let _ = apply_gitlab(&runner, Path::new("."), &patch, &current);
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        // BTreeSet order: cli, rust, tauri (legacy removed).
        assert_eq!(
            calls[0].args,
            vec!["repo", "edit", "--topics", "cli,rust,tauri"]
        );
    }

    #[test]
    fn apply_gitlab_shell_metacharacters_in_topic_pass_literally() {
        let runner = MockRunner::new();
        runner.expect("glab", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            topics_added: vec!["x; rm -rf /".into()],
            ..Default::default()
        };
        let _ = apply_gitlab(&runner, Path::new("."), &patch, &[]);
        let calls = runner.calls();
        // The whole dangerous string becomes one argv entry — no
        // splitting, no escaping.
        assert_eq!(
            calls[0].args,
            vec!["repo", "edit", "--topics", "x; rm -rf /"]
        );
    }

    #[test]
    fn apply_gitlab_visibility_and_branches_and_toggles() {
        let runner = MockRunner::new();
        runner.expect("glab", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            visibility: Some(Visibility::Private),
            default_branch: Some("trunk".into()),
            issues_enabled: Some(false),
            wiki_enabled: Some(true),
            ..Default::default()
        };
        let _ = apply_gitlab(&runner, Path::new("."), &patch, &[]);
        assert!(runner.was_called_with("glab", &["repo", "edit", "--visibility", "private"]));
        assert!(runner.was_called_with("glab", &["repo", "edit", "--default-branch", "trunk"]));
        assert!(runner.was_called_with(
            "glab",
            &["repo", "edit", "--issues-access-level", "disabled"],
        ));
        assert!(
            runner.was_called_with("glab", &["repo", "edit", "--wiki-access-level", "enabled"],)
        );
    }

    #[test]
    fn apply_gitlab_ignores_archive_field() {
        let runner = MockRunner::new();
        runner.expect("glab", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            archive: Some(true),
            ..Default::default()
        };
        let r = apply_gitlab(&runner, Path::new("."), &patch, &[]);
        assert!(r.failures.is_empty());
        assert!(r.fields_updated.is_empty());
        assert!(runner.calls().is_empty());
    }

    #[test]
    fn apply_gitlab_homepage_clear_sends_empty_string() {
        let runner = MockRunner::new();
        runner.expect("glab", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            homepage: PatchValue::Clear,
            ..Default::default()
        };
        let _ = apply_gitlab(&runner, Path::new("."), &patch, &[]);
        assert!(runner.was_called_with("glab", &["repo", "edit", "--homepage", ""]));
    }

    // ─── Label CRUD tests ──────────────────────────────────────────────

    #[test]
    fn create_label_github_passes_name_color_description_as_flags() {
        let runner = MockRunner::new();
        runner.expect("gh", &["label", "create"], Ok(ok_output()));
        let label = Label {
            name: "bug".into(),
            color: Some("ff0000".into()),
            description: Some("Broken".into()),
        };
        create_label_github(&runner, Path::new("."), &label).unwrap();
        let calls = runner.calls();
        assert_eq!(
            calls[0].args,
            vec![
                "label",
                "create",
                "bug",
                "--color",
                "ff0000",
                "--description",
                "Broken",
            ]
        );
    }

    #[test]
    fn create_label_gitlab_uses_name_flag() {
        let runner = MockRunner::new();
        runner.expect("glab", &["label", "create"], Ok(ok_output()));
        let label = Label {
            name: "bug".into(),
            color: Some("ff0000".into()),
            description: None,
        };
        create_label_gitlab(&runner, Path::new("."), &label).unwrap();
        assert!(runner.was_called_with(
            "glab",
            &["label", "create", "--name", "bug", "--color", "ff0000"],
        ));
    }

    #[test]
    fn update_label_github_renames_when_name_changed() {
        let runner = MockRunner::new();
        runner.expect("gh", &["label", "edit"], Ok(ok_output()));
        let new = Label {
            name: "defect".into(),
            color: Some("aa0000".into()),
            description: None,
        };
        update_label_github(&runner, Path::new("."), "bug", &new).unwrap();
        let calls = runner.calls();
        assert_eq!(
            calls[0].args,
            vec![
                "label", "edit", "bug", "--name", "defect", "--color", "aa0000"
            ]
        );
    }

    #[test]
    fn update_label_github_skips_name_flag_when_unchanged() {
        let runner = MockRunner::new();
        runner.expect("gh", &["label", "edit"], Ok(ok_output()));
        let new = Label {
            name: "bug".into(),
            color: Some("aa0000".into()),
            description: None,
        };
        update_label_github(&runner, Path::new("."), "bug", &new).unwrap();
        let calls = runner.calls();
        assert_eq!(
            calls[0].args,
            vec!["label", "edit", "bug", "--color", "aa0000"]
        );
    }

    #[test]
    fn update_label_gitlab_uses_label_update_subcommand() {
        let runner = MockRunner::new();
        runner.expect("glab", &["label", "update"], Ok(ok_output()));
        let new = Label {
            name: "bug".into(),
            color: Some("aa0000".into()),
            description: Some("x".into()),
        };
        update_label_gitlab(&runner, Path::new("."), "bug", &new).unwrap();
        let calls = runner.calls();
        assert_eq!(
            calls[0].args,
            vec![
                "label",
                "update",
                "bug",
                "--color",
                "aa0000",
                "--description",
                "x",
            ]
        );
    }

    #[test]
    fn delete_label_github_passes_yes_confirmation() {
        let runner = MockRunner::new();
        runner.expect("gh", &["label", "delete"], Ok(ok_output()));
        delete_label_github(&runner, Path::new("."), "bug").unwrap();
        assert!(runner.was_called_with("gh", &["label", "delete", "bug", "--yes"]));
    }

    #[test]
    fn delete_label_gitlab_passes_name_verbatim() {
        let runner = MockRunner::new();
        runner.expect("glab", &["label", "delete"], Ok(ok_output()));
        delete_label_gitlab(&runner, Path::new("."), "bug").unwrap();
        assert!(runner.was_called_with("glab", &["label", "delete", "bug"]));
    }

    #[test]
    fn label_name_with_shell_metacharacters_passes_literally() {
        // Same shell-injection regression as topics — a malicious
        // label name must survive as exactly one argv entry.
        let runner = MockRunner::new();
        runner.expect("gh", &["label", "create"], Ok(ok_output()));
        let label = Label {
            name: "bug; rm -rf /".into(),
            color: Some("ff0000".into()),
            description: None,
        };
        create_label_github(&runner, Path::new("."), &label).unwrap();
        let calls = runner.calls();
        assert_eq!(
            calls[0].args,
            vec!["label", "create", "bug; rm -rf /", "--color", "ff0000"]
        );
    }

    // ─── Branch-protection read tests ──────────────────────────────────

    fn gh_protection_full() -> &'static str {
        r#"{
            "required_pull_request_reviews": {
                "required_approving_review_count": 2,
                "require_code_owner_reviews": true,
                "dismiss_stale_reviews": false,
                "required_review_thread_resolution": true
            },
            "required_status_checks": {
                "strict": true,
                "contexts": ["ci/lint", "ci/test"]
            },
            "required_conversation_resolution": {"enabled": true},
            "enforce_admins": {"enabled": false}
        }"#
    }

    #[test]
    fn get_branch_protection_parses_full_payload() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["api"],
            Ok(CliOutput {
                stdout: gh_protection_full().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let p = get_branch_protection_github(&runner, Path::new("."), "main")
            .expect("ok")
            .expect("some");
        assert!(p.require_pull_request);
        assert_eq!(p.required_approvals, 2);
        assert!(p.require_status_checks);
        assert_eq!(p.status_check_contexts, vec!["ci/lint", "ci/test"]);
        assert!(p.require_up_to_date);
        assert!(p.require_conversation_resolution);
        assert!(!p.enforce_admins);

        // Exact argv — `repos/:owner/:repo/branches/main/protection`.
        assert!(runner.was_called_with(
            "gh",
            &["api", "repos/:owner/:repo/branches/main/protection"],
        ));
    }

    #[test]
    fn get_branch_protection_returns_none_on_404() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["api"],
            Err(crate::command_runner::CliError::NonZeroExit {
                exit_code: 1,
                stdout: String::new(),
                stderr: "gh: HTTP 404: Branch not protected".into(),
            }),
        );
        let p = get_branch_protection_github(&runner, Path::new("."), "main").unwrap();
        assert!(p.is_none());
    }

    #[test]
    fn get_branch_protection_handles_no_pr_reviews_block() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["api"],
            Ok(CliOutput {
                stdout: r#"{
                    "required_status_checks": null,
                    "enforce_admins": {"enabled": true}
                }"#
                .into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let p = get_branch_protection_github(&runner, Path::new("."), "main")
            .unwrap()
            .unwrap();
        assert!(!p.require_pull_request);
        assert_eq!(p.required_approvals, 0);
        assert!(!p.require_status_checks);
        assert!(p.status_check_contexts.is_empty());
        assert!(p.enforce_admins);
    }

    #[test]
    fn get_branch_protection_surfaces_auth_errors() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["api"],
            Err(crate::command_runner::CliError::NonZeroExit {
                exit_code: 4,
                stdout: String::new(),
                stderr: "gh: not authenticated".into(),
            }),
        );
        let e = get_branch_protection_github(&runner, Path::new("."), "main").unwrap_err();
        assert!(matches!(e, RepoConfigError::NotAuthenticated(_)));
    }

    // ─── Branch-protection write tests ────────────────────────────────

    #[test]
    fn build_payload_all_rules_enabled() {
        let rules = BranchProtection {
            require_pull_request: true,
            required_approvals: 2,
            require_status_checks: true,
            status_check_contexts: vec!["ci/lint".into(), "ci/test".into()],
            require_up_to_date: true,
            require_conversation_resolution: true,
            enforce_admins: true,
        };
        let payload = build_set_branch_protection_payload(&rules);
        let pr = &payload["required_pull_request_reviews"];
        assert_eq!(pr["required_approving_review_count"], 2);
        assert_eq!(pr["required_review_thread_resolution"], true);
        let checks = &payload["required_status_checks"];
        assert_eq!(checks["strict"], true);
        assert_eq!(
            checks["contexts"],
            serde_json::json!(["ci/lint", "ci/test"])
        );
        assert_eq!(payload["enforce_admins"], true);
        assert_eq!(payload["required_conversation_resolution"], true);
        assert_eq!(payload["restrictions"], serde_json::Value::Null);
    }

    #[test]
    fn build_payload_pr_disabled_uses_null_not_empty_object() {
        let rules = BranchProtection {
            require_pull_request: false,
            required_approvals: 0,
            require_status_checks: false,
            status_check_contexts: vec![],
            require_up_to_date: false,
            require_conversation_resolution: false,
            enforce_admins: false,
        };
        let payload = build_set_branch_protection_payload(&rules);
        assert_eq!(
            payload["required_pull_request_reviews"],
            serde_json::Value::Null
        );
        assert_eq!(payload["required_status_checks"], serde_json::Value::Null);
        assert_eq!(payload["enforce_admins"], false);
    }

    #[test]
    fn set_branch_protection_calls_gh_api_put_with_input_flag() {
        let runner = MockRunner::new();
        runner.expect("gh", &["api", "-X", "PUT"], Ok(ok_output()));
        let tmp = tempfile::tempdir().unwrap();
        let rules = BranchProtection {
            require_pull_request: true,
            required_approvals: 1,
            ..Default::default()
        };
        set_branch_protection_github(&runner, tmp.path(), "main", &rules).unwrap();
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0], "api");
        assert_eq!(calls[0].args[1], "-X");
        assert_eq!(calls[0].args[2], "PUT");
        assert_eq!(calls[0].args[3], "--input");
        assert!(calls[0].args[4].ends_with(".json"));
        assert_eq!(
            calls[0].args[5],
            "repos/:owner/:repo/branches/main/protection"
        );
    }

    #[test]
    fn set_branch_protection_cleans_up_temp_file_on_success() {
        let runner = MockRunner::new();
        runner.expect("gh", &["api"], Ok(ok_output()));
        let tmp = tempfile::tempdir().unwrap();
        let rules = BranchProtection::default();
        set_branch_protection_github(&runner, tmp.path(), "main", &rules).unwrap();
        // No stray temp file must remain.
        let entries: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().flatten().collect();
        assert!(
            entries.is_empty(),
            "expected empty tempdir, got: {:?}",
            entries.iter().map(|e| e.path()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn set_branch_protection_cleans_up_temp_file_on_failure() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["api"],
            Err(crate::command_runner::CliError::NonZeroExit {
                exit_code: 1,
                stdout: String::new(),
                stderr: "boom".into(),
            }),
        );
        let tmp = tempfile::tempdir().unwrap();
        let rules = BranchProtection::default();
        let err = set_branch_protection_github(&runner, tmp.path(), "main", &rules).unwrap_err();
        assert!(matches!(err, RepoConfigError::CommandFailed(_)));
        let entries: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().flatten().collect();
        assert!(entries.is_empty(), "tempfile should be cleaned on failure");
    }

    // ─── Dispatcher tests ──────────────────────────────────────────────

    #[test]
    fn dispatcher_load_routes_github_to_gh() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: gh_view_json_happy().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_with(&runner, ForgeKind::GitHub, Path::new("."))
            .expect("github load");
        assert_eq!(cfg.description, "A neat little repo");
        assert!(runner.was_called_with("gh", &["repo", "view", "--json", GH_REPO_VIEW_FIELDS]));
    }

    #[test]
    fn dispatcher_apply_routes_github_to_gh() {
        let runner = MockRunner::new();
        runner.expect("gh", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            description: Some("x".into()),
            ..Default::default()
        };
        let r =
            apply_remote_repo_config_with(&runner, ForgeKind::GitHub, Path::new("."), &patch, &[]);
        assert_eq!(r.fields_updated, vec!["description".to_string()]);
        assert!(runner.was_called_with("gh", &["repo", "edit", "--description", "x"]));
    }

    #[test]
    fn dispatcher_apply_routes_gitlab_to_glab_with_current_topics() {
        let runner = MockRunner::new();
        runner.expect("glab", &["repo", "edit"], Ok(ok_output()));
        let patch = RemoteRepoConfigPatch {
            topics_added: vec!["new".into()],
            ..Default::default()
        };
        let _ = apply_remote_repo_config_with(
            &runner,
            ForgeKind::GitLab,
            Path::new("."),
            &patch,
            &["old".to_string()],
        );
        assert!(runner.was_called_with("glab", &["repo", "edit", "--topics", "new,old"]));
    }

    #[test]
    fn dispatcher_load_routes_gitlab_to_glab() {
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: glab_view_json_happy().into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "glab",
            &["label", "list"],
            Ok(CliOutput {
                stdout: "[]".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let cfg = load_remote_repo_config_with(&runner, ForgeKind::GitLab, Path::new("."))
            .expect("gitlab load");
        assert_eq!(cfg.description, "A GitLab repo");
        assert!(runner.was_called_with("glab", &["repo", "view", "-F", "json"]));
    }

    #[test]
    fn branch_protection_defaults_to_permissive() {
        let p = BranchProtection::default();
        assert!(!p.require_pull_request);
        assert_eq!(p.required_approvals, 0);
        assert!(!p.require_status_checks);
        assert!(p.status_check_contexts.is_empty());
        assert!(!p.require_up_to_date);
        assert!(!p.require_conversation_resolution);
        assert!(!p.enforce_admins);
    }

    // ──────────────────────────────────────────────────────────────
    // Phase 7 — forge CLI status probe
    // ──────────────────────────────────────────────────────────────

    use crate::command_runner::CliError;

    #[test]
    fn probe_returns_unsupported_forge_when_forge_is_none() {
        let runner = MockRunner::new();
        let status = probe_forge_cli_status_with(&runner, None, None, Path::new("."));
        assert_eq!(status, ForgeCliStatus::UnsupportedForge);
        // Should not even try to run the CLI in this case.
        assert!(runner.calls().is_empty());
    }

    #[test]
    fn probe_returns_not_installed_when_binary_missing() {
        let runner = MockRunner::new();
        runner.expect("gh", &["--version"], Err(CliError::NotFound("gh".into())));
        let status =
            probe_forge_cli_status_with(&runner, Some(ForgeKind::GitHub), None, Path::new("."));
        assert_eq!(status, ForgeCliStatus::NotInstalled);
    }

    #[test]
    fn probe_returns_installed_authenticated_when_auth_succeeds() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["--version"],
            Ok(CliOutput {
                stdout: "gh version 2.42.0".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &["auth", "status"],
            Ok(CliOutput {
                stdout: String::new(),
                stderr: "Logged in to github.com as octocat (keyring)".into(),
                exit_code: 0,
            }),
        );
        let status =
            probe_forge_cli_status_with(&runner, Some(ForgeKind::GitHub), None, Path::new("."));
        match status {
            ForgeCliStatus::Installed {
                authenticated,
                account,
            } => {
                assert!(authenticated);
                assert_eq!(account.as_deref(), Some("octocat"));
            }
            other => panic!("expected Installed, got {other:?}"),
        }
    }

    #[test]
    fn probe_returns_installed_unauthenticated_when_auth_fails() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["--version"],
            Ok(CliOutput {
                stdout: "gh version 2.42.0".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &["auth", "status"],
            Err(CliError::NonZeroExit {
                exit_code: 1,
                stdout: String::new(),
                stderr: "You are not logged in.".into(),
            }),
        );
        let status =
            probe_forge_cli_status_with(&runner, Some(ForgeKind::GitHub), None, Path::new("."));
        match status {
            ForgeCliStatus::Installed { authenticated, .. } => {
                assert!(!authenticated);
            }
            other => panic!("expected Installed, got {other:?}"),
        }
    }

    #[test]
    fn probe_uses_glab_binary_for_gitlab_forges() {
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["--version"],
            Ok(CliOutput {
                stdout: "glab 1.0".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "glab",
            &["auth", "status"],
            Ok(CliOutput {
                stdout: "Logged in as devuser at gitlab.com".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let status =
            probe_forge_cli_status_with(&runner, Some(ForgeKind::GitLab), None, Path::new("."));
        match status {
            ForgeCliStatus::Installed {
                authenticated,
                account,
            } => {
                assert!(authenticated);
                assert_eq!(account.as_deref(), Some("devuser"));
            }
            other => panic!("expected Installed, got {other:?}"),
        }
        let calls = runner.calls();
        assert!(calls.iter().all(|c| c.cmd == "glab"));
    }

    #[test]
    fn probe_passes_hostname_flag_when_host_is_known() {
        // Given a known repo host, the auth probe must scope to that host
        // so multi-instance configs (e.g. gitlab.com + self-hosted) don't
        // poison each other.
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["--version"],
            Ok(CliOutput {
                stdout: "glab 1.92.1".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "glab",
            &["auth", "status", "--hostname", "gitlab.com"],
            Ok(CliOutput {
                stdout: "Logged in as devuser at gitlab.com".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let status = probe_forge_cli_status_with(
            &runner,
            Some(ForgeKind::GitLab),
            Some("gitlab.com"),
            Path::new("."),
        );
        assert!(matches!(
            status,
            ForgeCliStatus::Installed {
                authenticated: true,
                ..
            }
        ));
        let calls = runner.calls();
        let auth_call = calls
            .iter()
            .find(|c| c.cmd == "glab" && c.args.first().map(|s| s.as_str()) == Some("auth"))
            .expect("auth status call recorded");
        assert_eq!(
            auth_call.args,
            vec!["auth", "status", "--hostname", "gitlab.com"],
        );
    }

    #[test]
    fn probe_succeeds_for_authenticated_host_even_when_other_host_fails() {
        // Reproduces the multi-instance bug: bare `glab auth status` exits
        // non-zero if any configured host is unreachable. Scoped to a
        // single host, the probe must still succeed for the host we care
        // about.
        let runner = MockRunner::new();
        runner.expect(
            "glab",
            &["--version"],
            Ok(CliOutput {
                stdout: "glab 1.92.1".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        // Bare `auth status` (no hostname) returns the multi-host failure.
        runner.expect(
            "glab",
            &["auth", "status"],
            Err(CliError::NonZeroExit {
                exit_code: 1,
                stdout: String::new(),
                stderr: "could not authenticate to one or more of the configured GitLab instances"
                    .into(),
            }),
        );
        // Scoped to gitlab.com it's fine.
        runner.expect(
            "glab",
            &["auth", "status", "--hostname", "gitlab.com"],
            Ok(CliOutput {
                stdout: "Logged in as devuser at gitlab.com".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let status = probe_forge_cli_status_with(
            &runner,
            Some(ForgeKind::GitLab),
            Some("gitlab.com"),
            Path::new("."),
        );
        assert!(matches!(
            status,
            ForgeCliStatus::Installed {
                authenticated: true,
                ..
            }
        ));
    }

    #[test]
    fn extract_remote_host_handles_ssh_and_https() {
        assert_eq!(
            extract_remote_host("git@gitlab.com:group/project.git").as_deref(),
            Some("gitlab.com"),
        );
        assert_eq!(
            extract_remote_host("https://gitlab.group.team.blue/team/app.git").as_deref(),
            Some("gitlab.group.team.blue"),
        );
        assert_eq!(
            extract_remote_host("git@github.enterprise.example:org/repo").as_deref(),
            Some("github.enterprise.example"),
        );
        assert_eq!(extract_remote_host("/local/path/repo").as_deref(), None);
        assert_eq!(extract_remote_host("git@:no-host").as_deref(), None);
    }

    // ──────────────────────────────────────────────────────────────
    // Shell-injection safety — per-argument argv checks
    //
    // Each of these tests feeds a hostile payload (containing `; echo
    // INJECTED &&`) into the patch / label surfaces and asserts that
    // the literal payload survives into the recorded argv unmodified.
    // Because `CommandRunner` never passes the args through a shell
    // and because every `arg()` call forwards a single `&str`, the
    // payload cannot execute — but the assertions encode that
    // invariant explicitly so a regression (e.g. switching to a
    // shell string) trips the test suite.
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn apply_topics_preserves_literal_semicolons() {
        let runner = MockRunner::new();
        runner.set_default_ok("");
        let patch = RemoteRepoConfigPatch {
            topics_added: vec!["safe".into(), "x; echo INJECTED".into()],
            ..Default::default()
        };
        let _ = apply_github(&runner, Path::new("."), &patch);
        let calls = runner.calls();
        let add_topic_call = calls
            .iter()
            .find(|c| c.cmd == "gh" && c.args.contains(&"--add-topic".to_string()))
            .expect("expected an --add-topic invocation");
        // The hostile topic must be passed as a single literal argv
        // entry — no splitting on `;`, no escaping.
        assert!(
            add_topic_call
                .args
                .contains(&"x; echo INJECTED".to_string()),
            "topics_added argv missing literal semicolon: {:?}",
            add_topic_call.args
        );
    }

    #[test]
    fn apply_description_preserves_shell_metacharacters() {
        let runner = MockRunner::new();
        runner.set_default_ok("");
        let patch = RemoteRepoConfigPatch {
            description: Some("hello && rm -rf / $HOME".into()),
            ..Default::default()
        };
        let _ = apply_github(&runner, Path::new("."), &patch);
        let calls = runner.calls();
        assert!(
            calls
                .iter()
                .any(|c| c.cmd == "gh" && c.args.iter().any(|a| a == "hello && rm -rf / $HOME"))
        );
    }

    #[test]
    fn create_label_preserves_backticks_in_name() {
        let runner = MockRunner::new();
        runner.set_default_ok("");
        let label = Label {
            name: "`reboot`".into(),
            color: Some("ff0000".into()),
            description: Some("malicious `name`".into()),
        };
        let _ = create_label_github(&runner, Path::new("."), &label);
        let calls = runner.calls();
        // The name arg should appear literally (no stripping of backticks).
        assert!(
            calls
                .iter()
                .any(|c| c.args.contains(&"`reboot`".to_string()))
        );
    }
}
