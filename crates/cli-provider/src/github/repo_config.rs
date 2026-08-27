//! GitHub (`gh`) remote-repository configuration: load, apply, label CRUD,
//! and branch protection. Moved verbatim from app-core; exercised via
//! [`crate::command_runner::MockRunner`] in the crate-level `repo_config` tests.

use std::path::Path;

use serde::Deserialize;

use forge_provider::{
    BranchProtection, Label, PatchValue, RemoteRepoConfig, RemoteRepoConfigPatch, Visibility,
};

use crate::command_runner::CommandRunner;
use crate::repo_config::{ApplyResult, RepoConfigError};

/// JSON fields we request from `gh repo view` when loading config.
///
/// Exposed as a constant so the Tauri dispatcher and the tests agree
/// on exactly which fields are fetched (and in which order — `gh` is
/// tolerant, but keeping a canonical order helps snapshot-style tests).
pub(crate) const GH_REPO_VIEW_FIELDS: &str = "description,homepageUrl,repositoryTopics,visibility,defaultBranchRef,hasIssuesEnabled,hasWikiEnabled,isArchived";

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

    let homepage = view.homepage_url.filter(|v| !v.is_empty());

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

/// Delete a label on GitHub via `gh label delete <name> --yes`.
pub fn delete_label_github<R: CommandRunner + ?Sized>(
    runner: &R,
    repo_path: &Path,
    name: &str,
) -> Result<(), RepoConfigError> {
    runner.run("gh", &["label", "delete", name, "--yes"], repo_path)?;
    Ok(())
}

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
        Err(crate::command_runner::CliError::NonZeroExit { stderr, .. })
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
