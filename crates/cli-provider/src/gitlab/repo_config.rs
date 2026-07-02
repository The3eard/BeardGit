//! GitLab (`glab`) remote-repository configuration: load, apply, label CRUD.
//! Moved verbatim from app-core; exercised via
//! [`crate::command_runner::MockRunner`] in the crate-level `repo_config` tests.

use std::path::Path;

use serde::Deserialize;

use forge_provider::{Label, PatchValue, RemoteRepoConfig, RemoteRepoConfigPatch, Visibility};

use crate::command_runner::CommandRunner;
use crate::repo_config::{ApplyResult, RepoConfigError};

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

/// Delete a label on GitLab via `glab label delete <name>`.
pub fn delete_label_gitlab<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    name: &str,
) -> Result<(), RepoConfigError> {
    runner.run("glab", &["label", "delete", name], repo_path)?;
    Ok(())
}
