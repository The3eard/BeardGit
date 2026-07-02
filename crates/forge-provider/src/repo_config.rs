//! Remote repository configuration types shared across forge backends.
//!
//! Pure data model for the repo-settings feature (visibility, branch
//! protection, the full config snapshot, the edit patch + diff). The forge
//! CLI implementations that populate and apply these types live in
//! `cli-provider`; this crate stays dependency-light per the trait-purity
//! guard.

use serde::{Deserialize, Serialize};

use crate::Label;

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
