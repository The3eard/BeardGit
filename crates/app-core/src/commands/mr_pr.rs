//! MR/PR listing, creation, editing, merging, and review commands.
//!
//! Each command resolves the active forge provider via `build_forge_provider`
//! and invokes the corresponding trait method. Concrete implementations live
//! in `cli-provider` (`GitHubCli` / `GitLabCli`). Errors are stringified at
//! the IPC boundary per the Tauri convention.

use std::sync::Arc;

use forge_provider::{
    CreateMrPrInput, EditMrPrPatch, ForgeProvider, MergeStrategy, MrPr, MrPrDetail, MrPrDiffFile,
    MrPrFilter, MrPrState,
};
use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// List merge requests / pull requests.
#[tauri::command]
pub async fn list_mr_prs(
    state_filter: Option<MrPrState>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<MrPr>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let filter = MrPrFilter {
        state: state_filter,
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
pub async fn get_mr_pr_detail(
    number: u64,
    state: State<'_, AppState>,
) -> Result<MrPrDetail, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.get_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Get the changed files in a MR/PR.
#[tauri::command]
pub async fn get_mr_pr_diff(
    number: u64,
    state: State<'_, AppState>,
) -> Result<Vec<MrPrDiffFile>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.get_mr_pr_diff(number).map_err(|e| e.to_string())).await
}

/// Create a new MR/PR.
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
    run_blocking(move || provider.create_mr_pr(input).map_err(|e| e.to_string())).await
}

/// Edit a MR/PR's title and/or description.
#[tauri::command]
pub async fn edit_mr_pr(
    number: u64,
    title: Option<String>,
    body: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let patch = EditMrPrPatch { title, body };
    run_blocking(move || {
        provider
            .edit_mr_pr(number, patch)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Merge a MR/PR with the given strategy.
#[tauri::command]
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
pub async fn close_mr_pr(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.close_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Approve a MR/PR.
#[tauri::command]
pub async fn approve_mr_pr(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.approve_mr_pr(number).map_err(|e| e.to_string())).await
}

/// Request changes on a MR/PR with a comment body.
#[tauri::command]
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
