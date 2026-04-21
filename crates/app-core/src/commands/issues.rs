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

/// Parse the `state_filter` string from the IPC layer into an optional
/// [`IssueState`].
///
/// Extracted so it can be exercised in unit tests without a live
/// `State<AppState>` — the IPC command wrapper delegates here.
///
/// Returns `None` for unknown strings (and for `None`), matching the
/// "show everything" semantics the frontend relies on.
pub(crate) fn parse_issue_state_filter(raw: Option<&str>) -> Option<IssueState> {
    match raw {
        Some("open") => Some(IssueState::Open),
        Some("closed") => Some(IssueState::Closed),
        _ => None,
    }
}

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
        state: parse_issue_state_filter(state_filter.as_deref()),
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

#[cfg(test)]
mod tests {
    //! Tests the pure dispatch helper (`parse_issue_state_filter`) and
    //! exercises the issues trait methods through `MockProvider` — a
    //! `ForgeProvider` with default no-op / NotSupported impls. This
    //! confirms the command layer's calling convention (Arc<dyn> +
    //! trait method) compiles and dispatches correctly.

    use super::parse_issue_state_filter;
    use forge_provider::mock::MockProvider;
    use forge_provider::{
        CreateIssueInput, EditIssuePatch, ForgeError, ForgeKind, ForgeProvider, IssueFilter,
        IssueState,
    };

    #[test]
    fn parse_issue_state_filter_maps_known_strings() {
        assert_eq!(
            parse_issue_state_filter(Some("open")),
            Some(IssueState::Open)
        );
        assert_eq!(
            parse_issue_state_filter(Some("closed")),
            Some(IssueState::Closed)
        );
    }

    #[test]
    fn parse_issue_state_filter_unknown_or_none_returns_none() {
        assert_eq!(parse_issue_state_filter(None), None);
        // Typos / unknown strings fall through to "no filter", not an error.
        assert_eq!(parse_issue_state_filter(Some("merged")), None);
        assert_eq!(parse_issue_state_filter(Some("")), None);
    }

    #[test]
    fn mock_provider_list_issues_returns_not_supported() {
        let provider = MockProvider::new(ForgeKind::GitHub);
        let filter = IssueFilter {
            state: Some(IssueState::Open),
            author: None,
            assignee: None,
            label: None,
            milestone: None,
            text: None,
        };
        assert!(matches!(
            provider.list_issues(filter, 50),
            Err(ForgeError::NotSupported)
        ));
    }

    #[test]
    fn mock_provider_create_issue_returns_not_supported() {
        let provider = MockProvider::new(ForgeKind::GitLab);
        let input = CreateIssueInput {
            title: "t".into(),
            body: "b".into(),
            labels: vec![],
            assignees: vec![],
            milestone: None,
        };
        assert!(matches!(
            provider.create_issue(input),
            Err(ForgeError::NotSupported)
        ));
    }

    #[test]
    fn mock_provider_edit_issue_returns_not_supported() {
        let provider = MockProvider::new(ForgeKind::GitHub);
        let patch = EditIssuePatch {
            title: Some("new".into()),
            body: None,
        };
        assert!(matches!(
            provider.edit_issue(1, patch),
            Err(ForgeError::NotSupported)
        ));
    }
}
