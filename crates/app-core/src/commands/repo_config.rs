//! Remote repository configuration via `gh` / `glab` CLIs.
//!
//! Exposes Tauri commands for reading and writing remote repo metadata
//! (description, homepage, topics, visibility, default branch, issues/wiki
//! toggles, archive state), labels, and GitHub branch protection. All
//! operations shell out through the injectable [`CommandRunner`] seam so
//! every path is unit-testable without network access.
//!
//! ## Provider detection
//!
//! [`detect_forge`] parses a repository's `origin` remote URL and returns
//! the matching [`ForgeKind`] (GitHub or GitLab). Non-forge remotes
//! (`bitbucket.org`, plain git servers, file URLs, …) return `None` — the
//! caller renders a graceful "not supported" state instead of erroring.
//!
//! The function is a thin wrapper around [`provider::parse_remote_url`]
//! that translates the internal [`provider::ProviderKind`] into the
//! forge-facing [`ForgeKind`]; the two enums exist for historical reasons
//! and should eventually converge, but that churn lives outside this
//! slice.

use forge_provider::ForgeKind;
use git_engine::Repository;
use serde::{Deserialize, Serialize};

use super::helpers::extract_origin_url;

// ───────────────────────────────────────────────────────────────────────────
// Data model
// ───────────────────────────────────────────────────────────────────────────

/// Visibility of a remote repository.
///
/// All three values are first-class on both GitHub and GitLab, although
/// only orgs / groups can set [`Visibility::Internal`]. The forge
/// enforces that restriction; callers do not need to pre-check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// World-readable.
    Public,
    /// Visible only to the owner / collaborators.
    Private,
    /// Visible to everyone in the org / group (forge-specific).
    Internal,
}

impl Visibility {
    /// CLI-flag string (identical on both `gh` and `glab`).
    pub fn as_cli_str(self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
            Visibility::Internal => "internal",
        }
    }

    /// Parse a lowercase visibility string as produced by the forge CLI.
    pub fn from_cli_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "public" => Some(Visibility::Public),
            "private" => Some(Visibility::Private),
            "internal" => Some(Visibility::Internal),
            _ => None,
        }
    }
}

/// Branch-protection rules (GitHub first-pass surface).
///
/// Mirrors the fields BeardGit actually surfaces in the UI; fields
/// such as `restrict_pushes`, `required_signatures`, or
/// `required_linear_history` are deliberately omitted from the
/// first slice — see the spec's "out of scope" list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchProtection {
    /// Require at least one approving review before merge.
    pub require_pull_request: bool,
    /// Minimum approving reviews required. `0` means "any".
    pub required_approvals: u32,
    /// Require status checks to pass before merging.
    pub require_status_checks: bool,
    /// Names of contexts (check runs) that must pass.
    pub status_check_contexts: Vec<String>,
    /// Require the branch to be up-to-date with base before merging.
    pub require_up_to_date: bool,
    /// Require all review conversations to be resolved.
    pub require_conversation_resolution: bool,
    /// Whether the rules apply to administrators too.
    pub enforce_admins: bool,
}

/// A repository label (issues / MR / PR categorisation).
///
/// Mirrors the [`forge_provider::Label`] shape so the types flow
/// through the IPC boundary without an extra translation step — we
/// keep it here as a separate `Serialize` struct to avoid pulling an
/// additional alias dependency into downstream consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// Label name (primary identifier).
    pub name: String,
    /// Hex color without the leading `#` (e.g. `"ff0000"`). `None` when
    /// the forge returns no color.
    pub color: Option<String>,
    /// Optional human-readable description.
    pub description: Option<String>,
}

/// The full set of remote repository settings BeardGit exposes.
///
/// Loaded by [`load_remote_repo_config`] (to be added in a later
/// phase) and diffed against the user's edited copy to produce a
/// [`RemoteRepoConfigPatch`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRepoConfig {
    /// Repository description.
    pub description: String,
    /// Homepage URL. `None` when unset on the forge.
    pub homepage: Option<String>,
    /// Repository topics / tags.
    pub topics: Vec<String>,
    /// Current visibility.
    pub visibility: Visibility,
    /// Name of the default branch.
    pub default_branch: String,
    /// Whether the issue tracker is enabled.
    pub issues_enabled: bool,
    /// Whether the wiki is enabled.
    pub wiki_enabled: bool,
    /// Whether the repo is archived.
    pub archived: bool,
    /// Branch-protection rules for `default_branch`, if any.
    ///
    /// GitLab leaves this `None` until dedicated CLI support lands
    /// (see spec "out of scope").
    pub branch_protection: Option<BranchProtection>,
    /// Repository labels.
    pub labels: Vec<Label>,
}

/// Detect which forge backend a repository talks to based on its
/// `origin` remote URL.
///
/// Returns `Some(ForgeKind::GitHub)` for `github.com` / GitHub Enterprise
/// URLs, `Some(ForgeKind::GitLab)` for `gitlab.com` / self-hosted GitLab,
/// and `None` for every other host (Bitbucket, Gitea, plain git servers,
/// or repos with no `origin` remote).
///
/// Both SSH (`git@host:path.git`) and HTTPS
/// (`https://host/path[.git]`) remote URL formats are recognised. The
/// trailing `.git` suffix is optional.
///
/// # Examples
///
/// ```ignore
/// use app_core::commands::detect_forge;
/// let kind = detect_forge(&repo); // Some(ForgeKind::GitHub) for github.com
/// ```
pub fn detect_forge(repo: &Repository) -> Option<ForgeKind> {
    let url = extract_origin_url(repo)?;
    detect_forge_from_url(&url)
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

/// Detect the forge kind plus the canonical hostname of a repository's
/// `origin` remote.
///
/// Returns `Some((kind, host))` for known forges where we can extract a
/// host from the URL, e.g. `("gitlab.com", GitLab)`,
/// `("gitlab.group.team.blue", GitLab)` (when matched against a connected
/// provider's base URL), or `("github.com", GitHub)`. Returns `None` when
/// the forge can't be identified.
///
/// The host is what callers pass to `gh auth status -h <host>` /
/// `glab auth status -h <host>` so multi-instance configs aren't poisoned
/// by an unrelated host's auth failure.
pub fn detect_forge_with_host(repo: &Repository) -> Option<(ForgeKind, String)> {
    let url = extract_origin_url(repo)?;
    let kind = detect_forge_from_url(&url)?;
    let host = extract_remote_host(&url)?;
    Some((kind, host))
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

// ───────────────────────────────────────────────────────────────────────────
// Patch + diff
// ───────────────────────────────────────────────────────────────────────────

/// Tri-state value for patch fields that distinguish "unchanged",
/// "cleared", and "set to new value".
///
/// We use a dedicated enum rather than `Option<Option<T>>` because
/// `serde_json` collapses `Some(None)` to plain `null`, which then
/// round-trips as `None` — losing the "user explicitly cleared this"
/// signal. The explicit variants survive the IPC boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum PatchValue<T> {
    /// Leave the field unchanged on the forge.
    #[default]
    Unchanged,
    /// Clear the field (explicit empty string on the CLI).
    Clear,
    /// Set the field to a new value.
    Set(T),
}

impl<T> PatchValue<T> {
    /// `true` when the patch would emit no CLI flag for this field.
    pub fn is_unchanged(&self) -> bool {
        matches!(self, PatchValue::Unchanged)
    }
}

/// Minimal patch describing the fields a user changed.
///
/// `None`-valued fields are left unchanged on the forge. `homepage`
/// uses a [`PatchValue`] tri-state so the UI can distinguish "leave
/// unchanged" from "clear" — `gh repo edit --homepage ""` clears the
/// field.
///
/// Topics are expressed as add/remove deltas rather than a full
/// replacement because both CLIs only support incremental edits
/// (`--add-topic` / `--remove-topic`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRepoConfigPatch {
    /// New description. `None` = unchanged.
    pub description: Option<String>,
    /// Homepage change: unchanged / clear / set.
    #[serde(default)]
    pub homepage: PatchValue<String>,
    /// Topics the user added.
    #[serde(default)]
    pub topics_added: Vec<String>,
    /// Topics the user removed.
    #[serde(default)]
    pub topics_removed: Vec<String>,
    /// New visibility. `None` = unchanged.
    pub visibility: Option<Visibility>,
    /// New default branch. `None` = unchanged.
    pub default_branch: Option<String>,
    /// Toggle the issue tracker. `None` = unchanged.
    pub issues_enabled: Option<bool>,
    /// Toggle the wiki. `None` = unchanged.
    pub wiki_enabled: Option<bool>,
    /// Toggle archive state. `None` = unchanged; `Some(true)` = archive;
    /// `Some(false)` = unarchive.
    pub archive: Option<bool>,
}

impl RemoteRepoConfigPatch {
    /// `true` when no fields would be sent to the CLI.
    pub fn is_empty(&self) -> bool {
        self.description.is_none()
            && self.homepage.is_unchanged()
            && self.topics_added.is_empty()
            && self.topics_removed.is_empty()
            && self.visibility.is_none()
            && self.default_branch.is_none()
            && self.issues_enabled.is_none()
            && self.wiki_enabled.is_none()
            && self.archive.is_none()
    }
}

/// Diff two [`RemoteRepoConfig`] snapshots and produce the minimal
/// patch that, when applied to `before`, yields `after`.
///
/// For topics, a stable set comparison is used — the returned
/// `topics_added` / `topics_removed` vectors are sorted
/// deterministically so that two diffs of the same inputs always
/// produce argv-identical CLI calls (important for the mock-based
/// test suite).
pub fn diff_config(before: &RemoteRepoConfig, after: &RemoteRepoConfig) -> RemoteRepoConfigPatch {
    let description = if before.description != after.description {
        Some(after.description.clone())
    } else {
        None
    };
    let homepage = if before.homepage == after.homepage {
        PatchValue::Unchanged
    } else {
        match &after.homepage {
            None => PatchValue::Clear,
            Some(v) => PatchValue::Set(v.clone()),
        }
    };
    let before_topics: std::collections::BTreeSet<_> = before.topics.iter().cloned().collect();
    let after_topics: std::collections::BTreeSet<_> = after.topics.iter().cloned().collect();
    let topics_added: Vec<String> = after_topics.difference(&before_topics).cloned().collect();
    let topics_removed: Vec<String> = before_topics.difference(&after_topics).cloned().collect();
    let visibility = if before.visibility != after.visibility {
        Some(after.visibility)
    } else {
        None
    };
    let default_branch = if before.default_branch != after.default_branch {
        Some(after.default_branch.clone())
    } else {
        None
    };
    let issues_enabled = if before.issues_enabled != after.issues_enabled {
        Some(after.issues_enabled)
    } else {
        None
    };
    let wiki_enabled = if before.wiki_enabled != after.wiki_enabled {
        Some(after.wiki_enabled)
    } else {
        None
    };
    let archive = if before.archived != after.archived {
        Some(after.archived)
    } else {
        None
    };

    RemoteRepoConfigPatch {
        description,
        homepage,
        topics_added,
        topics_removed,
        visibility,
        default_branch,
        issues_enabled,
        wiki_enabled,
        archive,
    }
}

// ───────────────────────────────────────────────────────────────────────────
// GitHub load
// ───────────────────────────────────────────────────────────────────────────

use std::path::Path;

use super::command_runner::{CliError as RunnerCliError, CommandRunner};

/// JSON fields we request from `gh repo view` when loading config.
///
/// Exposed as a constant so the Tauri dispatcher and the tests agree
/// on exactly which fields are fetched (and in which order — `gh` is
/// tolerant, but keeping a canonical order helps snapshot-style tests).
const GH_REPO_VIEW_FIELDS: &str = "description,homepageUrl,repositoryTopics,visibility,defaultBranchRef,hasIssuesEnabled,hasWikiEnabled,isArchived";

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

// Raw JSON shapes emitted by `gh repo view --json ...`.

#[derive(Deserialize)]
struct GhDefaultBranchRef {
    name: String,
}

#[derive(Deserialize)]
struct GhRepositoryTopic {
    // `gh` wraps each topic in `{ "name": ..., "resourcePath": ... }`.
    name: String,
}

#[derive(Deserialize)]
struct GhRepoView {
    description: Option<String>,
    #[serde(rename = "homepageUrl")]
    homepage_url: Option<String>,
    // `gh` returns `"repositoryTopics": null` for repos with no topics
    // instead of omitting the field or emitting `[]`. `#[serde(default)]`
    // only handles the *missing* case, so we accept `Option<Vec<…>>`
    // here and collapse to `Vec::new()` below.
    #[serde(default, rename = "repositoryTopics")]
    repository_topics: Option<Vec<GhRepositoryTopic>>,
    visibility: String,
    #[serde(rename = "defaultBranchRef")]
    default_branch_ref: Option<GhDefaultBranchRef>,
    #[serde(default, rename = "hasIssuesEnabled")]
    has_issues_enabled: bool,
    #[serde(default, rename = "hasWikiEnabled")]
    has_wiki_enabled: bool,
    #[serde(default, rename = "isArchived")]
    is_archived: bool,
}

#[derive(Deserialize)]
struct GhLabel {
    name: String,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

/// Load repo config from GitHub via `gh repo view --json ...`.
///
/// Labels are fetched in a second call (`gh label list --json ...`)
/// because `gh repo view` does not include them and label pagination
/// would otherwise inflate the single-call response.
///
/// `branch_protection` is left as `None` here — branch protection is
/// loaded on demand by the Protection tab (see Phase 5) to avoid
/// paying the extra API call for repos the user never opens that tab
/// on.
pub fn load_remote_repo_config_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
) -> Result<RemoteRepoConfig, RepoConfigError> {
    let view_output = runner.run(
        "gh",
        &["repo", "view", "--json", GH_REPO_VIEW_FIELDS],
        repo_path,
    )?;
    let view: GhRepoView = serde_json::from_str(&view_output.stdout)
        .map_err(|e| RepoConfigError::JsonError(e.to_string()))?;

    let visibility = Visibility::from_cli_str(&view.visibility).ok_or_else(|| {
        RepoConfigError::JsonError(format!("unknown visibility: {}", view.visibility))
    })?;

    let labels = load_labels_github(runner, repo_path)?;

    let homepage = view
        .homepage_url
        .and_then(|v| if v.is_empty() { None } else { Some(v) });

    Ok(RemoteRepoConfig {
        description: view.description.unwrap_or_default(),
        homepage,
        topics: view
            .repository_topics
            .unwrap_or_default()
            .into_iter()
            .map(|t| t.name)
            .collect(),
        visibility,
        default_branch: view.default_branch_ref.map(|r| r.name).unwrap_or_default(),
        issues_enabled: view.has_issues_enabled,
        wiki_enabled: view.has_wiki_enabled,
        archived: view.is_archived,
        branch_protection: None,
        labels,
    })
}

/// Load the repository label list from GitHub via `gh label list`.
///
/// Extracted so the load path can be exercised without also running
/// `gh repo view`, and so Phase 4's label CRUD commands can share the
/// reader path.
pub fn load_labels_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
) -> Result<Vec<Label>, RepoConfigError> {
    let output = runner.run(
        "gh",
        &[
            "label",
            "list",
            "--json",
            "name,color,description",
            "--limit",
            "200",
        ],
        repo_path,
    )?;
    let labels: Vec<GhLabel> = serde_json::from_str(&output.stdout)
        .map_err(|e| RepoConfigError::JsonError(e.to_string()))?;
    Ok(labels
        .into_iter()
        .map(|l| Label {
            name: l.name,
            color: l.color,
            description: l.description,
        })
        .collect())
}

// ───────────────────────────────────────────────────────────────────────────
// Apply patch — shared types
// ───────────────────────────────────────────────────────────────────────────

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
    fn record_success(&mut self, field: &str) {
        self.fields_updated.push(field.to_string());
    }

    fn record_failure(&mut self, field: &str, err: impl std::fmt::Display) {
        self.failures.push(FieldError {
            field: field.to_string(),
            message: err.to_string(),
        });
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Apply patch — GitHub
// ───────────────────────────────────────────────────────────────────────────

/// Apply a [`RemoteRepoConfigPatch`] to a GitHub repo via `gh repo edit`
/// / `gh repo archive` / `gh repo unarchive`.
///
/// Each sub-field is wrapped in its own CLI call so a failure in one
/// does not stop the others — the UI surfaces the mix via
/// [`ApplyResult`].
pub fn apply_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    patch: &RemoteRepoConfigPatch,
) -> ApplyResult {
    let mut result = ApplyResult::default();

    if let Some(desc) = patch.description.as_deref() {
        let args = ["repo", "edit", "--description", desc];
        match runner.run("gh", &args, repo_path) {
            Ok(_) => result.record_success("description"),
            Err(e) => result.record_failure("description", e),
        }
    }

    match &patch.homepage {
        PatchValue::Unchanged => {}
        PatchValue::Clear => {
            let args = ["repo", "edit", "--homepage", ""];
            match runner.run("gh", &args, repo_path) {
                Ok(_) => result.record_success("homepage"),
                Err(e) => result.record_failure("homepage", e),
            }
        }
        PatchValue::Set(url) => {
            let args = ["repo", "edit", "--homepage", url.as_str()];
            match runner.run("gh", &args, repo_path) {
                Ok(_) => result.record_success("homepage"),
                Err(e) => result.record_failure("homepage", e),
            }
        }
    }

    if !patch.topics_added.is_empty() {
        let mut args: Vec<&str> = vec!["repo", "edit"];
        for t in &patch.topics_added {
            args.push("--add-topic");
            args.push(t.as_str());
        }
        match runner.run("gh", &args, repo_path) {
            Ok(_) => result.record_success("topics_added"),
            Err(e) => result.record_failure("topics_added", e),
        }
    }

    if !patch.topics_removed.is_empty() {
        let mut args: Vec<&str> = vec!["repo", "edit"];
        for t in &patch.topics_removed {
            args.push("--remove-topic");
            args.push(t.as_str());
        }
        match runner.run("gh", &args, repo_path) {
            Ok(_) => result.record_success("topics_removed"),
            Err(e) => result.record_failure("topics_removed", e),
        }
    }

    if let Some(vis) = patch.visibility {
        let args = ["repo", "edit", "--visibility", vis.as_cli_str()];
        match runner.run("gh", &args, repo_path) {
            Ok(_) => result.record_success("visibility"),
            Err(e) => result.record_failure("visibility", e),
        }
    }

    if let Some(branch) = patch.default_branch.as_deref() {
        let args = ["repo", "edit", "--default-branch", branch];
        match runner.run("gh", &args, repo_path) {
            Ok(_) => result.record_success("default_branch"),
            Err(e) => result.record_failure("default_branch", e),
        }
    }

    if let Some(enabled) = patch.issues_enabled {
        let flag = if enabled { "true" } else { "false" };
        let args = ["repo", "edit", "--enable-issues", flag];
        match runner.run("gh", &args, repo_path) {
            Ok(_) => result.record_success("issues_enabled"),
            Err(e) => result.record_failure("issues_enabled", e),
        }
    }

    if let Some(enabled) = patch.wiki_enabled {
        let flag = if enabled { "true" } else { "false" };
        let args = ["repo", "edit", "--enable-wiki", flag];
        match runner.run("gh", &args, repo_path) {
            Ok(_) => result.record_success("wiki_enabled"),
            Err(e) => result.record_failure("wiki_enabled", e),
        }
    }

    if let Some(archive) = patch.archive {
        let sub = if archive { "archive" } else { "unarchive" };
        let args = ["repo", sub, "--yes"];
        match runner.run("gh", &args, repo_path) {
            Ok(_) => result.record_success("archive"),
            Err(e) => result.record_failure("archive", e),
        }
    }

    result
}

// ───────────────────────────────────────────────────────────────────────────
// GitLab load
// ───────────────────────────────────────────────────────────────────────────

/// Raw shape of `glab repo view -F json` output (subset we care about).
///
/// `glab` returns a GitLab project payload which uses snake_case keys
/// and encodes feature toggles as `{issues,wiki}_access_level` strings
/// (`"enabled"`, `"disabled"`, `"private"`). The loader maps that to
/// the simpler boolean used in [`RemoteRepoConfig`].
#[derive(Deserialize)]
struct GlabRepoView {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    web_url: Option<String>,
    /// `glab` sometimes surfaces a dedicated homepage field as
    /// `homepage`; otherwise the best we can do is `web_url`. We only
    /// treat `homepage` as the homepage — `web_url` stays in the
    /// sidebar instead.
    #[serde(default)]
    homepage: Option<String>,
    /// Canonical field on modern GitLab. The deprecated `tag_list`
    /// alias used to be accepted via `#[serde(alias = "tag_list")]`,
    /// but modern GitLab emits *both* fields in the same payload and
    /// serde rejects an aliased duplicate as `duplicate field "topics"`.
    /// Topic editing has been on `topics` since GitLab 14.0 — relying
    /// on the canonical name is safe.
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    visibility: String,
    #[serde(default)]
    default_branch: Option<String>,
    /// Access levels can be `"enabled" | "disabled" | "private"`.
    /// Anything other than `"disabled"` is treated as enabled.
    #[serde(default)]
    issues_access_level: Option<String>,
    #[serde(default)]
    wiki_access_level: Option<String>,
    #[serde(default)]
    archived: bool,
}

/// Raw shape of `glab label list --per-page 200 -F json` output.
#[derive(Deserialize)]
struct GlabLabel {
    name: String,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

fn glab_access_to_bool(value: Option<&str>) -> bool {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("disabled") => false,
        // `None` means the field was absent — default to enabled so the
        // UI doesn't wrongly show features as off on older `glab`
        // versions that omit the access levels.
        _ => true,
    }
}

/// Load repo config from GitLab via `glab repo view -F json`.
///
/// The `gitlab` CLI encodes its JSON output with snake_case keys and
/// uses `<feature>_access_level` strings for toggles; both are
/// adapted here so the returned [`RemoteRepoConfig`] looks identical
/// to the GitHub path.
pub fn load_remote_repo_config_gitlab<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
) -> Result<RemoteRepoConfig, RepoConfigError> {
    let view_output = runner.run("glab", &["repo", "view", "-F", "json"], repo_path)?;
    let view: GlabRepoView = serde_json::from_str(&view_output.stdout)
        .map_err(|e| RepoConfigError::JsonError(e.to_string()))?;

    let visibility = Visibility::from_cli_str(&view.visibility).ok_or_else(|| {
        RepoConfigError::JsonError(format!("unknown visibility: {}", view.visibility))
    })?;

    let homepage = view
        .homepage
        .or(view.web_url)
        .and_then(|v| if v.is_empty() { None } else { Some(v) });

    let labels = load_labels_gitlab(runner, repo_path)?;

    Ok(RemoteRepoConfig {
        description: view.description.unwrap_or_default(),
        homepage,
        topics: view.topics,
        visibility,
        default_branch: view.default_branch.unwrap_or_default(),
        issues_enabled: glab_access_to_bool(view.issues_access_level.as_deref()),
        wiki_enabled: glab_access_to_bool(view.wiki_access_level.as_deref()),
        archived: view.archived,
        branch_protection: None,
        labels,
    })
}

/// Load labels from GitLab via `glab label list --per-page 200 -F json`.
pub fn load_labels_gitlab<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
) -> Result<Vec<Label>, RepoConfigError> {
    let output = runner.run(
        "glab",
        &["label", "list", "--per-page", "200", "-F", "json"],
        repo_path,
    )?;
    let labels: Vec<GlabLabel> = serde_json::from_str(&output.stdout)
        .map_err(|e| RepoConfigError::JsonError(e.to_string()))?;
    Ok(labels
        .into_iter()
        .map(|l| Label {
            name: l.name,
            color: l.color.map(|c| c.trim_start_matches('#').to_string()),
            description: l.description,
        })
        .collect())
}

// ───────────────────────────────────────────────────────────────────────────
// Apply patch — GitLab
// ───────────────────────────────────────────────────────────────────────────

/// Apply a patch to a GitLab repo via `glab repo edit`.
///
/// `glab repo edit` accepts `--topics` as a comma-separated full
/// replacement list, not incremental add/remove — so this helper
/// merges `current_topics` ∪ `patch.topics_added` \\ `patch.topics_removed`
/// and emits a single `--topics a,b,c` flag when either add or
/// remove is non-empty.
///
/// `glab` has no dedicated archive/unarchive subcommand today, so
/// the archive field is silently ignored on GitLab — the UI is
/// responsible for hiding the toggle on this provider (see spec
/// "out of scope").
pub fn apply_gitlab<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    patch: &RemoteRepoConfigPatch,
    current_topics: &[String],
) -> ApplyResult {
    let mut result = ApplyResult::default();

    if let Some(desc) = patch.description.as_deref() {
        let args = ["repo", "edit", "--description", desc];
        match runner.run("glab", &args, repo_path) {
            Ok(_) => result.record_success("description"),
            Err(e) => result.record_failure("description", e),
        }
    }

    if !matches!(patch.homepage, PatchValue::Unchanged) {
        let value = match &patch.homepage {
            PatchValue::Set(v) => v.as_str(),
            PatchValue::Clear => "",
            PatchValue::Unchanged => unreachable!(),
        };
        let args = ["repo", "edit", "--homepage", value];
        match runner.run("glab", &args, repo_path) {
            Ok(_) => result.record_success("homepage"),
            Err(e) => result.record_failure("homepage", e),
        }
    }

    if !patch.topics_added.is_empty() || !patch.topics_removed.is_empty() {
        // Merge current ∪ added, minus removed. BTreeSet keeps the
        // argv deterministic for mock-based tests.
        let removed: std::collections::BTreeSet<&str> =
            patch.topics_removed.iter().map(|s| s.as_str()).collect();
        let mut merged: std::collections::BTreeSet<&str> =
            current_topics.iter().map(|s| s.as_str()).collect();
        for a in &patch.topics_added {
            merged.insert(a.as_str());
        }
        let merged_vec: Vec<&str> = merged
            .iter()
            .copied()
            .filter(|t| !removed.contains(t))
            .collect();
        let joined = merged_vec.join(",");
        let args = ["repo", "edit", "--topics", joined.as_str()];
        match runner.run("glab", &args, repo_path) {
            Ok(_) => result.record_success("topics"),
            Err(e) => result.record_failure("topics", e),
        }
    }

    if let Some(vis) = patch.visibility {
        let args = ["repo", "edit", "--visibility", vis.as_cli_str()];
        match runner.run("glab", &args, repo_path) {
            Ok(_) => result.record_success("visibility"),
            Err(e) => result.record_failure("visibility", e),
        }
    }

    if let Some(branch) = patch.default_branch.as_deref() {
        let args = ["repo", "edit", "--default-branch", branch];
        match runner.run("glab", &args, repo_path) {
            Ok(_) => result.record_success("default_branch"),
            Err(e) => result.record_failure("default_branch", e),
        }
    }

    if let Some(enabled) = patch.issues_enabled {
        let flag = if enabled { "enabled" } else { "disabled" };
        let args = ["repo", "edit", "--issues-access-level", flag];
        match runner.run("glab", &args, repo_path) {
            Ok(_) => result.record_success("issues_enabled"),
            Err(e) => result.record_failure("issues_enabled", e),
        }
    }

    if let Some(enabled) = patch.wiki_enabled {
        let flag = if enabled { "enabled" } else { "disabled" };
        let args = ["repo", "edit", "--wiki-access-level", flag];
        match runner.run("glab", &args, repo_path) {
            Ok(_) => result.record_success("wiki_enabled"),
            Err(e) => result.record_failure("wiki_enabled", e),
        }
    }

    // archive is intentionally skipped on GitLab — see doc comment.
    let _ = patch.archive;

    result
}

// ───────────────────────────────────────────────────────────────────────────
// Labels CRUD
// ───────────────────────────────────────────────────────────────────────────

/// Create a label on GitHub via `gh label create`.
pub fn create_label_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    label: &Label,
) -> Result<(), RepoConfigError> {
    let mut args: Vec<&str> = vec!["label", "create", label.name.as_str()];
    if let Some(c) = label.color.as_deref() {
        args.push("--color");
        args.push(c);
    }
    if let Some(d) = label.description.as_deref() {
        args.push("--description");
        args.push(d);
    }
    runner.run("gh", &args, repo_path)?;
    Ok(())
}

/// Create a label on GitLab via `glab label create`.
pub fn create_label_gitlab<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    label: &Label,
) -> Result<(), RepoConfigError> {
    let mut args: Vec<&str> = vec!["label", "create", "--name", label.name.as_str()];
    if let Some(c) = label.color.as_deref() {
        args.push("--color");
        args.push(c);
    }
    if let Some(d) = label.description.as_deref() {
        args.push("--description");
        args.push(d);
    }
    runner.run("glab", &args, repo_path)?;
    Ok(())
}

/// Edit a label on GitHub via `gh label edit`.
///
/// `old_name` is the label's current name; when the user renames the
/// label it is passed to `--name` so `gh` can rename in place.
pub fn update_label_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    old_name: &str,
    label: &Label,
) -> Result<(), RepoConfigError> {
    let mut args: Vec<&str> = vec!["label", "edit", old_name];
    if old_name != label.name {
        args.push("--name");
        args.push(label.name.as_str());
    }
    if let Some(c) = label.color.as_deref() {
        args.push("--color");
        args.push(c);
    }
    if let Some(d) = label.description.as_deref() {
        args.push("--description");
        args.push(d);
    }
    runner.run("gh", &args, repo_path)?;
    Ok(())
}

/// Edit a label on GitLab via `glab label update`.
pub fn update_label_gitlab<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    old_name: &str,
    label: &Label,
) -> Result<(), RepoConfigError> {
    let mut args: Vec<&str> = vec!["label", "update", old_name];
    if old_name != label.name {
        args.push("--name");
        args.push(label.name.as_str());
    }
    if let Some(c) = label.color.as_deref() {
        args.push("--color");
        args.push(c);
    }
    if let Some(d) = label.description.as_deref() {
        args.push("--description");
        args.push(d);
    }
    runner.run("glab", &args, repo_path)?;
    Ok(())
}

/// Delete a label on GitHub via `gh label delete <name> --yes`.
pub fn delete_label_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    name: &str,
) -> Result<(), RepoConfigError> {
    runner.run("gh", &["label", "delete", name, "--yes"], repo_path)?;
    Ok(())
}

/// Delete a label on GitLab via `glab label delete <name>`.
pub fn delete_label_gitlab<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    name: &str,
) -> Result<(), RepoConfigError> {
    runner.run("glab", &["label", "delete", name], repo_path)?;
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// Branch protection (GitHub)
// ───────────────────────────────────────────────────────────────────────────

/// Raw shape of `gh api repos/:owner/:repo/branches/:branch/protection`.
///
/// GitHub wraps each section in an `enabled` / `value` pair; the
/// loader flattens these into the simpler [`BranchProtection`]
/// struct.
#[derive(Deserialize)]
struct GhProtectionEnabled {
    #[serde(default)]
    enabled: bool,
}

#[derive(Deserialize)]
struct GhRequiredPrReviews {
    #[serde(default)]
    required_approving_review_count: u32,
    #[serde(default)]
    required_review_thread_resolution: bool,
}

#[derive(Deserialize)]
struct GhRequiredStatusChecks {
    #[serde(default)]
    strict: bool,
    #[serde(default)]
    contexts: Vec<String>,
}

#[derive(Deserialize)]
struct GhProtection {
    #[serde(default)]
    required_pull_request_reviews: Option<GhRequiredPrReviews>,
    #[serde(default)]
    required_status_checks: Option<GhRequiredStatusChecks>,
    #[serde(default)]
    required_conversation_resolution: Option<GhProtectionEnabled>,
    #[serde(default)]
    enforce_admins: Option<GhProtectionEnabled>,
}

fn is_404_stderr(stderr: &str) -> bool {
    let l = stderr.to_ascii_lowercase();
    l.contains("http 404") || l.contains("not found") || l.contains("branch not protected")
}

/// Load the branch-protection rules for `branch` on a GitHub repo.
///
/// Returns `Ok(None)` when the branch exists but has no protection
/// rule — GitHub returns HTTP 404 in that case, which we treat as
/// "no rule" rather than an error. Every other CLI failure (auth,
/// network, unknown status) surfaces as `Err`.
pub fn get_branch_protection_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    branch: &str,
) -> Result<Option<BranchProtection>, RepoConfigError> {
    let endpoint = format!("repos/:owner/:repo/branches/{branch}/protection");
    let args = ["api", endpoint.as_str()];
    let output = match runner.run("gh", &args, repo_path) {
        Ok(o) => o,
        Err(super::command_runner::CliError::NonZeroExit { stderr, .. })
            if is_404_stderr(&stderr) =>
        {
            return Ok(None);
        }
        Err(e) => return Err(e.into()),
    };

    let raw: GhProtection = serde_json::from_str(&output.stdout)
        .map_err(|e| RepoConfigError::JsonError(e.to_string()))?;

    let (require_pull_request, required_approvals) = match &raw.required_pull_request_reviews {
        Some(r) => (true, r.required_approving_review_count),
        None => (false, 0),
    };
    let (require_status_checks, require_up_to_date, status_check_contexts) =
        match &raw.required_status_checks {
            Some(s) => (true, s.strict, s.contexts.clone()),
            None => (false, false, Vec::new()),
        };
    let require_conversation_resolution = raw
        .required_conversation_resolution
        .as_ref()
        .map(|v| v.enabled)
        .unwrap_or(false)
        || raw
            .required_pull_request_reviews
            .as_ref()
            .map(|r| r.required_review_thread_resolution)
            .unwrap_or(false);
    let enforce_admins = raw
        .enforce_admins
        .as_ref()
        .map(|v| v.enabled)
        .unwrap_or(false);

    Ok(Some(BranchProtection {
        require_pull_request,
        required_approvals,
        require_status_checks,
        status_check_contexts,
        require_up_to_date,
        require_conversation_resolution,
        enforce_admins,
    }))
}

/// Build the JSON payload `gh api -X PUT` expects for branch
/// protection.
///
/// Extracted for testability — assembling the payload correctly
/// (unset sections sent as `null`, not `{}`) is the part that can
/// drift against the GitHub API.
pub fn build_set_branch_protection_payload(rules: &BranchProtection) -> serde_json::Value {
    use serde_json::json;

    let required_pull_request_reviews = if rules.require_pull_request {
        json!({
            "required_approving_review_count": rules.required_approvals,
            "dismiss_stale_reviews": false,
            "require_code_owner_reviews": false,
            "required_review_thread_resolution": rules.require_conversation_resolution,
        })
    } else {
        serde_json::Value::Null
    };
    let required_status_checks = if rules.require_status_checks {
        json!({
            "strict": rules.require_up_to_date,
            "contexts": rules.status_check_contexts,
        })
    } else {
        serde_json::Value::Null
    };

    json!({
        "required_status_checks": required_status_checks,
        "enforce_admins": rules.enforce_admins,
        "required_pull_request_reviews": required_pull_request_reviews,
        "restrictions": serde_json::Value::Null,
        "required_conversation_resolution": rules.require_conversation_resolution,
    })
}

/// Write branch-protection rules via `gh api -X PUT …`.
///
/// The payload is built from [`build_set_branch_protection_payload`]
/// and serialised to a single `--input -` style stdin would be ideal,
/// but `gh api -X PUT -f key=value` takes simple flat fields — for a
/// nested payload we pass `--input -` with stdin. The mocked runner
/// does not support stdin in this first slice, so instead we use
/// `gh api -X PUT --input <path>` with a tempfile written by the
/// caller — for now we simply serialise the JSON into a single
/// `--raw-field` arg which `gh` accepts as `field=@-`. To keep the
/// test surface simple and still prove per-argument safety, we pass
/// the JSON body with the `--input -` flag wiring deferred; today
/// we use `--input` + a scratch file in the repo path.
///
/// Implementation detail: the payload is piped through `--input -`
/// via stdin is not yet supported by [`CommandRunner`], so we write
/// the JSON to a hidden tempfile under the repo and pass
/// `--input <path>`. The file is removed after the CLI returns.
pub fn set_branch_protection_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    branch: &str,
    rules: &BranchProtection,
) -> Result<(), RepoConfigError> {
    let payload = build_set_branch_protection_payload(rules);
    let body =
        serde_json::to_string(&payload).map_err(|e| RepoConfigError::JsonError(e.to_string()))?;

    // Write to a temp file inside the repo path so Windows file
    // locking doesn't bite us (cross-device moves are avoided by
    // staying on the same volume).
    let tmp_name = format!(".beardgit-branch-protection-{branch}.json");
    let sanitized_name = tmp_name.replace('/', "_");
    let tmp_path = repo_path.join(&sanitized_name);
    std::fs::write(&tmp_path, body).map_err(|e| RepoConfigError::Io(e.to_string()))?;

    let endpoint = format!("repos/:owner/:repo/branches/{branch}/protection");
    let tmp_str = tmp_path.to_string_lossy().into_owned();
    let args = [
        "api",
        "-X",
        "PUT",
        "--input",
        tmp_str.as_str(),
        endpoint.as_str(),
    ];
    let run_result = runner.run("gh", &args, repo_path);
    let _ = std::fs::remove_file(&tmp_path);
    run_result?;
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// Tauri dispatcher (load)
// ───────────────────────────────────────────────────────────────────────────

use super::command_runner::SystemRunner;
use crate::state::AppState;
use tauri::State;
use tracing::instrument;

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

/// Tauri command: load the remote repo configuration for a given
/// repository path.
///
/// The frontend knows which repo is active and passes its path in
/// explicitly — the command deliberately does not read the active
/// index because the settings dialog may be opened from a sidebar
/// context menu for a repo that is not the active one.
///
/// Detects the forge from the repo's `origin` remote and calls the
/// matching loader. Returns a human-readable error string when the
/// remote is neither GitHub nor GitLab so the frontend can render a
/// "not supported" state instead of popping an error toast.
///
/// Uses the real [`SystemRunner`] — tests exercise
/// [`load_remote_repo_config_with`] directly with a mock.
#[tauri::command]
#[instrument(skip(_state), name = "cmd::repo_config::load")]
pub async fn load_remote_repo_config(
    repo_path: String,
    _state: State<'_, AppState>,
) -> Result<RemoteRepoConfig, String> {
    let path = std::path::PathBuf::from(&repo_path);

    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let runner = SystemRunner::new();
        load_remote_repo_config_with(&runner, forge, &path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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

/// Tauri command: apply a `RemoteRepoConfigPatch` to the remote repo
/// at `repo_path`.
///
/// On GitLab, loads the current config first so the helper can compute
/// the full `--topics` replacement list; on GitHub that extra CLI call
/// is skipped. Partial failures are returned inside
/// [`ApplyResult::failures`] so the UI can render a mixed-state toast
/// without aborting the whole save.
#[tauri::command]
#[instrument(skip(_state, patch), name = "cmd::repo_config::apply")]
pub async fn apply_remote_repo_config(
    repo_path: String,
    patch: RemoteRepoConfigPatch,
    _state: State<'_, AppState>,
) -> Result<ApplyResult, String> {
    let path = std::path::PathBuf::from(&repo_path);

    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let runner = SystemRunner::new();

        let current_topics = if matches!(forge, ForgeKind::GitLab) {
            load_remote_repo_config_with(&runner, forge, &path)
                .map(|c| c.topics)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(apply_remote_repo_config_with(
            &runner,
            forge,
            &path,
            &patch,
            &current_topics,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Tauri command: create a new label on the remote repo.
///
/// Dispatches to `create_label_github` / `create_label_gitlab` based
/// on the forge detected from the repository's `origin` remote.
#[tauri::command]
#[instrument(skip(_state, label), name = "cmd::repo_config::create_label")]
pub async fn create_label(
    repo_path: String,
    label: Label,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let runner = SystemRunner::new();
        let result = match forge {
            ForgeKind::GitHub => create_label_github(&runner, &path, &label),
            ForgeKind::GitLab => create_label_gitlab(&runner, &path, &label),
        };
        result.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Tauri command: update an existing label on the remote repo.
#[tauri::command]
#[instrument(
    skip(_state, label),
    name = "cmd::repo_config::update_label",
    fields(old_name = %old_name)
)]
pub async fn update_label(
    repo_path: String,
    old_name: String,
    label: Label,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let runner = SystemRunner::new();
        let result = match forge {
            ForgeKind::GitHub => update_label_github(&runner, &path, &old_name, &label),
            ForgeKind::GitLab => update_label_gitlab(&runner, &path, &old_name, &label),
        };
        result.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Tauri command: delete a label by name.
#[tauri::command]
#[instrument(skip(_state), name = "cmd::repo_config::delete_label", fields(name = %name))]
pub async fn delete_label(
    repo_path: String,
    name: String,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let runner = SystemRunner::new();
        let result = match forge {
            ForgeKind::GitHub => delete_label_github(&runner, &path, &name),
            ForgeKind::GitLab => delete_label_gitlab(&runner, &path, &name),
        };
        result.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Tauri command: load GitHub branch-protection rules for a branch.
///
/// Returns `Ok(None)` when the branch is not protected. GitLab is
/// not supported in the first slice; calling this command on a
/// GitLab repo returns an error string the frontend turns into a
/// "not supported on this provider" empty state.
#[tauri::command]
#[instrument(
    skip(_state),
    name = "cmd::repo_config::get_branch_protection",
    fields(branch = %branch)
)]
pub async fn get_branch_protection(
    repo_path: String,
    branch: String,
    _state: State<'_, AppState>,
) -> Result<Option<BranchProtection>, String> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        match forge {
            ForgeKind::GitHub => {
                let runner = SystemRunner::new();
                get_branch_protection_github(&runner, &path, &branch).map_err(|e| e.to_string())
            }
            ForgeKind::GitLab => {
                Err("Branch protection is not supported on GitLab yet".to_string())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Tauri command: write GitHub branch-protection rules for a branch.
#[tauri::command]
#[instrument(
    skip(_state, rules),
    name = "cmd::repo_config::set_branch_protection",
    fields(branch = %branch)
)]
pub async fn set_branch_protection(
    repo_path: String,
    branch: String,
    rules: BranchProtection,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        match forge {
            ForgeKind::GitHub => {
                let runner = SystemRunner::new();
                set_branch_protection_github(&runner, &path, &branch, &rules)
                    .map_err(|e| e.to_string())
            }
            ForgeKind::GitLab => {
                Err("Branch protection is not supported on GitLab yet".to_string())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

// ───────────────────────────────────────────────────────────────────────────
// CLI status probe (Phase 7)
// ───────────────────────────────────────────────────────────────────────────

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

/// Tauri command: probe the forge CLI availability + auth state for
/// the repo at `repo_path`.
///
/// Used by the frontend before rendering the repo-config dialog body
/// so we can pick between "install gh/glab", "authenticate first", or
/// the real UI. Never returns a hard error — every failure mode maps
/// to a structured [`ForgeCliStatus`] variant the frontend renders as
/// an empty state.
#[tauri::command]
#[instrument(skip(_state), name = "cmd::repo_config::probe_cli")]
pub async fn probe_forge_cli_status(
    repo_path: String,
    _state: State<'_, AppState>,
) -> Result<ForgeCliStatus, String> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let detected = detect_forge_with_host(&repo);
        let (forge, host) = match &detected {
            Some((k, h)) => (Some(*k), Some(h.as_str())),
            None => (None, None),
        };
        let runner = SystemRunner::new();
        Ok(probe_forge_cli_status_with(&runner, forge, host, &path))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn detect_forge_from_repository_with_github_origin() {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        git_repo
            .remote("origin", "https://github.com/test/repo.git")
            .unwrap();
        drop(git_repo);
        let repo = Repository::open(dir.path()).unwrap();
        assert_eq!(detect_forge(&repo), Some(ForgeKind::GitHub));
    }

    #[test]
    fn detect_forge_from_repository_with_gitlab_origin() {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        git_repo
            .remote("origin", "git@gitlab.com:team/app.git")
            .unwrap();
        drop(git_repo);
        let repo = Repository::open(dir.path()).unwrap();
        assert_eq!(detect_forge(&repo), Some(ForgeKind::GitLab));
    }

    #[test]
    fn detect_forge_from_repository_without_origin_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        assert!(detect_forge(&repo).is_none());
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

    use super::super::command_runner::{CliOutput, MockRunner};

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
            Err(super::super::command_runner::CliError::NonZeroExit {
                exit_code: 4,
                stdout: String::new(),
                stderr: "gh: not authenticated. Run gh auth login.".into(),
            }),
        );
        let err = load_remote_repo_config_github(&runner, Path::new(".")).unwrap_err();
        assert!(matches!(err, RepoConfigError::NotAuthenticated(_)));
    }

    #[test]
    fn load_github_maps_cli_missing_to_structured_error() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Err(super::super::command_runner::CliError::NotFound(
                "gh".into(),
            )),
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
            Err(super::super::command_runner::CliError::NonZeroExit {
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
            Err(super::super::command_runner::CliError::NonZeroExit {
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
            Err(super::super::command_runner::CliError::NonZeroExit {
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
            Err(super::super::command_runner::CliError::NonZeroExit {
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
            Err(super::super::command_runner::CliError::NonZeroExit {
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

    use super::super::command_runner::CliError;

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
