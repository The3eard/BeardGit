//! MR/PR listing, creation, editing, merging, and review commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// List merge requests / pull requests.
#[tauri::command]
pub async fn list_mr_prs(
    state_filter: Option<cli_provider::MrPrState>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<cli_provider::MrPr>, String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || {
        cli.list_mr_prs(state_filter, limit.unwrap_or(30))
            .map_err(|e| e.to_string())
    })
    .await
}

/// Get detailed info about a single MR/PR.
#[tauri::command]
pub async fn get_mr_pr_detail(
    number: u64,
    state: State<'_, AppState>,
) -> Result<cli_provider::MrPrDetail, String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || cli.get_mr_pr_detail(number).map_err(|e| e.to_string())).await
}

/// Get the changed files in a MR/PR.
#[tauri::command]
pub async fn get_mr_pr_diff(
    number: u64,
    state: State<'_, AppState>,
) -> Result<Vec<cli_provider::MrPrDiffFile>, String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || cli.get_mr_pr_diff(number).map_err(|e| e.to_string())).await
}

/// Create a new MR/PR.
///
/// Creates a merge request (GitLab) or pull request (GitHub) with the given
/// metadata. Returns the newly created MR/PR summary.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_mr_pr(
    source: String,
    target: String,
    title: String,
    body: String,
    draft: bool,
    labels: Vec<String>,
    reviewers: Vec<String>,
    state: State<'_, AppState>,
) -> Result<cli_provider::MrPr, String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || {
        cli.create_mr_pr(&source, &target, &title, &body, draft, &labels, &reviewers)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Edit a MR/PR's title and/or description.
#[tauri::command]
pub async fn edit_mr_pr(
    number: u64,
    title: Option<String>,
    body: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || {
        cli.edit_mr_pr(number, title.as_deref(), body.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

/// Merge a MR/PR with the given strategy.
#[tauri::command]
pub async fn merge_mr_pr(
    number: u64,
    strategy: cli_provider::MergeStrategy,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || cli.merge_mr_pr(number, strategy).map_err(|e| e.to_string())).await
}

/// Close a MR/PR without merging.
#[tauri::command]
pub async fn close_mr_pr(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || cli.close_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Approve a MR/PR.
#[tauri::command]
pub async fn approve_mr_pr(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || cli.approve_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Request changes on a MR/PR with a comment body.
///
/// On GitHub this submits a "request changes" review. On GitLab it posts
/// a comment (GitLab has no direct "request changes" concept).
#[tauri::command]
pub async fn request_changes_mr_pr(
    number: u64,
    body: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || {
        cli.request_changes(number, &body)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Add a general comment to a MR/PR.
#[tauri::command]
pub async fn add_mr_pr_comment(
    number: u64,
    body: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || cli.add_comment(number, &body).map_err(|e| e.to_string())).await
}

/// Add an inline comment on a specific file and line of a MR/PR diff.
#[tauri::command]
pub async fn add_mr_pr_inline_comment(
    number: u64,
    path: String,
    line: u64,
    body: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cli = build_cli_provider(&state)?;
    run_blocking(move || {
        cli.add_inline_comment(number, &path, line, &body)
            .map_err(|e| e.to_string())
    })
    .await
}
