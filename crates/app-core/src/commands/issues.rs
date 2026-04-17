//! Tauri commands for Issues — list/detail/create/edit/close/reopen/comment/
//! labels/assignees/milestones. Wraps [`ForgeProvider`] trait methods.
//!
//! Each command resolves the active forge provider via [`build_forge_provider`]
//! and dispatches to the corresponding trait method on a `spawn_blocking`
//! thread. Errors are stringified at the IPC boundary per Tauri convention.

use std::sync::Arc;

use forge_provider::{
    CreateIssueInput, EditIssuePatch, ForgeProvider, Issue, IssueDetail, IssueFilter, IssueState,
    Milestone,
};
use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// List issues for the current repo with optional filters.
///
/// The `state_filter` arg accepts `"open"`, `"closed"`, or `None` (=all).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn list_issues(
    state_filter: Option<String>,
    author: Option<String>,
    assignee: Option<String>,
    label: Option<String>,
    milestone: Option<u64>,
    text: Option<String>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<Issue>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let filter = IssueFilter {
        state: match state_filter.as_deref() {
            Some("open") => Some(IssueState::Open),
            Some("closed") => Some(IssueState::Closed),
            _ => None,
        },
        author,
        assignee,
        label,
        milestone,
        text,
    };
    let limit = limit.unwrap_or(50);
    run_blocking(move || {
        provider
            .list_issues(filter, limit)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Fetch full detail (body + comments) for a single issue.
#[tauri::command]
pub async fn get_issue(number: u64, state: State<'_, AppState>) -> Result<IssueDetail, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.get_issue(number).map_err(|e| e.to_string())).await
}

/// Create a new issue.
#[tauri::command]
pub async fn create_issue(
    title: String,
    body: String,
    labels: Vec<String>,
    assignees: Vec<String>,
    milestone: Option<u64>,
    state: State<'_, AppState>,
) -> Result<Issue, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let input = CreateIssueInput {
        title,
        body,
        labels,
        assignees,
        milestone,
    };
    run_blocking(move || provider.create_issue(input).map_err(|e| e.to_string())).await
}

/// Edit an issue's title and/or body.
#[tauri::command]
pub async fn edit_issue(
    number: u64,
    title: Option<String>,
    body: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let patch = EditIssuePatch { title, body };
    run_blocking(move || {
        provider
            .edit_issue(number, patch)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Close an open issue.
#[tauri::command]
pub async fn close_issue(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.close_issue(number).map_err(|e| e.to_string())).await
}

/// Reopen a closed issue.
#[tauri::command]
pub async fn reopen_issue(number: u64, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.reopen_issue(number).map_err(|e| e.to_string())).await
}

/// Post a general comment on an issue.
#[tauri::command]
pub async fn add_issue_comment(
    number: u64,
    body: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .add_issue_comment(number, &body)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Add labels to an issue.
#[tauri::command]
pub async fn add_issue_labels(
    number: u64,
    labels: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .add_issue_labels(number, &labels)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Remove labels from an issue.
#[tauri::command]
pub async fn remove_issue_labels(
    number: u64,
    labels: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .remove_issue_labels(number, &labels)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Add assignees to an issue.
#[tauri::command]
pub async fn add_issue_assignees(
    number: u64,
    assignees: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .add_issue_assignees(number, &assignees)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Remove assignees from an issue.
#[tauri::command]
pub async fn remove_issue_assignees(
    number: u64,
    assignees: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .remove_issue_assignees(number, &assignees)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Set (or clear with `None`) the milestone on an issue.
#[tauri::command]
pub async fn set_issue_milestone(
    number: u64,
    milestone_id: Option<u64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .set_issue_milestone(number, milestone_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// List all milestones for the current repo (for picker UIs).
#[tauri::command]
pub async fn list_milestones(state: State<'_, AppState>) -> Result<Vec<Milestone>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.list_milestones().map_err(|e| e.to_string())).await
}
