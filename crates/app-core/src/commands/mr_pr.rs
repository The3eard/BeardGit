//! MR/PR listing, creation, editing, merging, and review commands.
//!
//! Each command resolves the active forge provider via `build_forge_provider`
//! and invokes the corresponding trait method. Concrete implementations live
//! in `cli-provider` (`GitHubCli` / `GitLabCli`). Errors are stringified at
//! the IPC boundary per the Tauri convention.

use std::sync::Arc;

use forge_provider::{
    CheckoutResult, CreateMrPrInput, EditMrPrPatch, ForgeProvider, Label, MergeStrategy, MrPr,
    MrPrDetail, MrPrDiffFile, MrPrFilter, MrPrState,
};
use mutation_events::MutationKind;
use task_runner::{OutputLine, Stream as TaskStream, TaskId, TaskManager, TaskStatus};
use tauri::{AppHandle, Emitter, State};
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// List merge requests / pull requests.
#[tauri::command]
#[instrument(skip(state), name = "cmd::mr_pr::list")]
pub async fn list_mr_prs(
    state_filter: Option<MrPrState>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<MrPr>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let filter = MrPrFilter {
        state: state_filter,
        ..Default::default()
    };
    run_blocking(move || {
        provider
            .list_mr_prs(filter, limit.unwrap_or(30))
            .map_err(|e| e.to_string())
    })
    .await
}

/// Get detailed info about a single MR/PR.
#[tauri::command]
#[instrument(skip(state), name = "cmd::mr_pr::detail")]
pub async fn get_mr_pr_detail(
    number: u64,
    state: State<'_, AppState>,
) -> Result<MrPrDetail, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.get_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Get the changed files in a MR/PR.
#[tauri::command]
#[instrument(skip(state), name = "cmd::mr_pr::diff")]
pub async fn get_mr_pr_diff(
    number: u64,
    state: State<'_, AppState>,
) -> Result<Vec<MrPrDiffFile>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.get_mr_pr_diff(number).map_err(|e| e.to_string())).await
}

/// Create a new MR/PR.
///
/// Wraps the provider call inside a
/// [`MutationGuard`][mutation_events::MutationGuard] scope so that on success a
/// `project-mutated` event with [`MutationKind::Push`] is emitted — creating a
/// PR/MR implies the source branch has (or will have) been pushed to the
/// remote, so downstream consumers refresh as if a push happened.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
#[instrument(skip(state, body, app), name = "cmd::mr_pr::create")]
pub async fn create_mr_pr(
    source: String,
    target: String,
    title: String,
    body: String,
    draft: bool,
    labels: Vec<String>,
    reviewers: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<MrPr, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let input = CreateMrPrInput {
        source,
        target,
        title,
        body,
        draft,
        labels,
        reviewers,
    };
    with_mutation_guard_async(&state, &app, MutationKind::Push, || async move {
        run_blocking(move || provider.create_mr_pr(input).map_err(|e| e.to_string())).await
    })
    .await
}

/// Edit a MR/PR's title and/or description.
#[tauri::command]
#[instrument(skip(state, body), name = "cmd::mr_pr::edit")]
pub async fn edit_mr_pr(
    number: u64,
    title: Option<String>,
    body: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let patch = EditMrPrPatch {
        title,
        body,
        draft: None,
    };
    run_blocking(move || {
        provider
            .edit_mr_pr(number, patch)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Merge a MR/PR with the given strategy.
#[tauri::command]
#[instrument(skip(state), name = "cmd::mr_pr::merge")]
pub async fn merge_mr_pr(
    number: u64,
    strategy: MergeStrategy,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .merge_mr_pr(number, strategy)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Close a MR/PR without merging.
#[tauri::command]
#[instrument(skip(state), name = "cmd::mr_pr::close")]
pub async fn close_mr_pr(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.close_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Approve a MR/PR.
#[tauri::command]
#[instrument(skip(state), name = "cmd::mr_pr::approve")]
pub async fn approve_mr_pr(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.approve_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Request changes on a MR/PR with a comment body.
#[tauri::command]
#[instrument(skip(state, body), name = "cmd::mr_pr::request_changes")]
pub async fn request_changes_mr_pr(
    number: u64,
    body: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .request_changes(number, &body)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Add a general comment to a MR/PR.
#[tauri::command]
#[instrument(skip(state, body), name = "cmd::mr_pr::comment")]
pub async fn add_mr_pr_comment(
    number: u64,
    body: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .add_mr_pr_comment(number, &body)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Add an inline comment on a specific file and line of a MR/PR diff.
#[tauri::command]
#[instrument(skip(state, body), name = "cmd::mr_pr::inline_comment")]
pub async fn add_mr_pr_inline_comment(
    number: u64,
    path: String,
    line: u64,
    body: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .add_mr_pr_inline_comment(number, &path, line, &body)
            .map_err(|e| e.to_string())
    })
    .await
}

// ─── Phase 8.2: MR/PR enhancements ─────────────────────────────────────

/// Add labels to an existing MR/PR.
#[tauri::command]
pub async fn add_mr_pr_labels(
    number: u64,
    labels: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .add_mr_pr_labels(number, &labels)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Remove labels from an existing MR/PR.
#[tauri::command]
pub async fn remove_mr_pr_labels(
    number: u64,
    labels: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .remove_mr_pr_labels(number, &labels)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Add reviewers to an existing MR/PR.
#[tauri::command]
pub async fn add_mr_pr_reviewers(
    number: u64,
    reviewers: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .add_mr_pr_reviewers(number, &reviewers)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Remove reviewers from an existing MR/PR.
#[tauri::command]
pub async fn remove_mr_pr_reviewers(
    number: u64,
    reviewers: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .remove_mr_pr_reviewers(number, &reviewers)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Mark a draft MR/PR as ready for review.
#[tauri::command]
pub async fn mark_mr_pr_ready(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.mark_mr_pr_ready(number).map_err(|e| e.to_string())).await
}

/// Convert a ready MR/PR back to draft.
#[tauri::command]
pub async fn mark_mr_pr_draft(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.mark_mr_pr_draft(number).map_err(|e| e.to_string())).await
}

/// Reopen a previously closed MR/PR.
#[tauri::command]
pub async fn reopen_mr_pr(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.reopen_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Mark a GitLab discussion thread as resolved. GitHub returns `NotSupported`.
#[tauri::command]
pub async fn resolve_discussion(
    number: u64,
    discussion_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .resolve_discussion(number, &discussion_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Mark a GitLab discussion thread as unresolved. GitHub returns `NotSupported`.
#[tauri::command]
pub async fn unresolve_discussion(
    number: u64,
    discussion_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .unresolve_discussion(number, &discussion_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// List all repository labels for populating the label picker UI.
#[tauri::command]
pub async fn list_labels(state: State<'_, AppState>) -> Result<Vec<Label>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.list_labels().map_err(|e| e.to_string())).await
}

/// Check out a MR/PR branch locally as a background task.
///
/// Spawns `gh pr checkout N` or `glab mr checkout N` via the task runner so
/// stdout/stderr stream to the task popover. On success, emits
/// `mr-pr-checked-out` with the parsed [`CheckoutResult`] payload and
/// `repo-changed` so the graph refreshes.
#[tauri::command]
pub async fn checkout_mr_pr_locally(
    number: u64,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
    app_handle: AppHandle,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    // Determine which CLI to invoke based on the active provider kind.
    let (binary, subcmd): (std::path::PathBuf, &'static str) = {
        let kind = {
            let providers = state.providers.lock().map_err(|e| e.to_string())?;
            let active = state
                .active_provider_index
                .lock()
                .map_err(|e| e.to_string())?;
            let idx = active.ok_or_else(|| "No active provider".to_string())?;
            let conn = providers
                .get(idx)
                .ok_or_else(|| "Active provider index out of bounds".to_string())?;
            conn.kind
        };
        let bin = resolve_cli_binary(&state, kind)?;
        let subcmd = match kind {
            provider::ProviderKind::GitHub => "pr",
            provider::ProviderKind::GitLab => "mr",
        };
        (bin, subcmd)
    };

    let num_str = number.to_string();
    let args: Vec<&str> = vec![subcmd, "checkout", &num_str];

    let label = format!("Checkout MR/PR #{number}");
    let binary_str = binary.to_string_lossy().to_string();

    let id = task_manager
        .spawn(label, &binary_str, &args, &cwd, true)
        .await;

    // Spawn a listener that parses task output on completion and emits events.
    let tm: Arc<TaskManager> = Arc::clone(&task_manager);
    let handle = app_handle.clone();
    tokio::spawn(async move {
        if let Ok(TaskStatus::Completed) = tm.wait_for_terminal(id).await
            && let Some(output) = tm.get_output(id).await
        {
            let stdout = stdout_from_output(&output);
            let result = parse_checkout_output(&stdout);
            let _ = handle.emit("mr-pr-checked-out", &result);
            let _ = handle.emit("repo-changed", ());
        }
    });

    Ok(id)
}

/// Join stdout-only lines from a task's captured output into a single string.
fn stdout_from_output(output: &[OutputLine]) -> String {
    output
        .iter()
        .filter(|l| matches!(l.stream, TaskStream::Stdout))
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Heuristic parser for `gh pr checkout` / `glab mr checkout` output.
///
/// Handles git's "Switched to ..." lines and both `gh`'s quoted
/// "Added remote 'name'" / `glab`'s unquoted "Adding remote name" forms.
pub(super) fn parse_checkout_output(stdout: &str) -> CheckoutResult {
    let mut branch_name = String::new();
    let mut remote_added: Option<String> = None;
    for line in stdout.lines() {
        let after_new = line.strip_prefix("Switched to a new branch '");
        let after_existing = line.strip_prefix("Switched to branch '");
        let after_glab = line.strip_prefix("Checking out branch '");
        if let Some(rest) = after_new.or(after_existing).or(after_glab)
            && let Some(end) = rest.find('\'')
        {
            branch_name = rest[..end].to_string();
        }
        if let Some(rest) = line.strip_prefix("Added remote '") {
            if let Some(end) = rest.find('\'') {
                remote_added = Some(rest[..end].to_string());
            }
        } else if let Some(rest) = line.strip_prefix("Adding remote ") {
            remote_added = Some(rest.trim().to_string());
        }
    }
    CheckoutResult {
        branch_name,
        is_fork: remote_added.is_some(),
        remote_added,
    }
}

#[cfg(test)]
mod checkout_output_tests {
    use super::parse_checkout_output;

    #[test]
    fn parses_gh_simple_output() {
        let out = "From github.com:foo/bar\n   abc..def  pull/42/head -> origin/pull/42/head\nSwitched to a new branch 'feature-x'\n";
        let r = parse_checkout_output(out);
        assert_eq!(r.branch_name, "feature-x");
        assert!(!r.is_fork);
        assert_eq!(r.remote_added, None);
    }

    #[test]
    fn parses_gh_fork_output() {
        let out = "Added remote 'contributor'\nFrom github.com:contributor/bar\nSwitched to a new branch 'contributor-feature'\n";
        let r = parse_checkout_output(out);
        assert_eq!(r.branch_name, "contributor-feature");
        assert!(r.is_fork);
        assert_eq!(r.remote_added.as_deref(), Some("contributor"));
    }

    #[test]
    fn parses_glab_simple_output() {
        let out = "Checking out branch 'feature-y' from merge request !7\n";
        let r = parse_checkout_output(out);
        assert_eq!(r.branch_name, "feature-y");
        assert!(!r.is_fork);
    }

    #[test]
    fn parses_glab_fork_output() {
        let out =
            "Adding remote fork-user\nChecking out branch 'fork-feature' from merge request !7\n";
        let r = parse_checkout_output(out);
        assert_eq!(r.branch_name, "fork-feature");
        assert!(r.is_fork);
        assert_eq!(r.remote_added.as_deref(), Some("fork-user"));
    }
}
