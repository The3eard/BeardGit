//! Tag listing, creation, deletion, and push commands.

use std::sync::Arc;

use mutation_events::MutationKind;
use task_runner::{TaskId, TaskManager};
use tauri::{AppHandle, State};
use tracing::instrument;

use super::helpers::*;
use crate::ipc_error::IpcError;
use crate::state::AppState;

/// Return all tags in the active repository, sorted newest-version-first.
#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> Result<Vec<git_engine::TagInfo>, IpcError> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.tags().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// List tags with pagination, sorted newest-version-first.
#[tauri::command]
pub async fn list_tags_paginated(
    per_page: u32,
    page: u32,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::TagInfo>, IpcError> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.tags_paginated(per_page, page)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Search all tags by name substring (case-insensitive).
#[tauri::command]
pub async fn search_tags(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::TagInfo>, IpcError> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.search_tags(&query).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Create a new tag in the active repository.
///
/// - If `message` is provided and non-empty, creates an annotated tag.
/// - Otherwise creates a lightweight tag.
/// - If `target` is empty, tags HEAD.
///
/// Wraps the work inside a [`MutationGuard`][mutation_events::MutationGuard]
/// scope so that on success a `project-mutated` event with
/// [`MutationKind::TagCreate`] is emitted.
#[tauri::command]
// `skip_all` keeps the annotation `message` out; name and target are
// refs, which are safe and are the useful part.
#[instrument(skip_all, fields(tag = %name, target = %target), name = "cmd::tag::create")]
pub async fn create_tag(
    name: String,
    target: String,
    message: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state).map_err(|e| IpcError::new("internal", e))?;
    with_mutation_guard_async(&state, &app, MutationKind::TagCreate, || async move {
        tokio::task::spawn_blocking(move || {
            let repo = git_engine::Repository::open(repo_path).map_err(IpcError::from)?;
            let msg = message.as_deref().filter(|m| !m.is_empty());
            let result = if target.is_empty() {
                repo.create_tag(&name, msg).map_err(IpcError::from)?
            } else {
                match msg {
                    Some(m) => repo
                        .git_cmd(&["tag", "-a", &name, &target, "-m", m])
                        .map_err(IpcError::from)?,
                    None => repo
                        .git_cmd(&["tag", &name, &target])
                        .map_err(IpcError::from)?,
                }
            };
            if result.success {
                Ok(())
            } else {
                Err(IpcError::new("cli_error", result.stderr))
            }
        })
        .await
        .map_err(|e| IpcError::new("internal", e.to_string()))?
    })
    .await
    .map_err(IpcError::from)
}

/// Delete a local tag by name.
///
/// Wraps the work inside a [`MutationGuard`][mutation_events::MutationGuard]
/// scope so that on success a `project-mutated` event with
/// [`MutationKind::TagDelete`] is emitted.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::tag::delete")]
pub async fn delete_tag(
    name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state).map_err(|e| IpcError::new("internal", e))?;
    with_mutation_guard_async(&state, &app, MutationKind::TagDelete, || async move {
        tokio::task::spawn_blocking(move || {
            let repo = git_engine::Repository::open(repo_path).map_err(IpcError::from)?;
            let result = repo.delete_tag(&name).map_err(IpcError::from)?;
            if result.success {
                Ok(())
            } else {
                Err(IpcError::new("cli_error", result.stderr))
            }
        })
        .await
        .map_err(|e| IpcError::new("internal", e.to_string()))?
    })
    .await
    .map_err(IpcError::from)
}

/// Push a tag to a remote as a background task.
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::tag::push")]
pub async fn push_tag(
    tag_name: Option<String>,
    remote: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, IpcError> {
    let cwd = get_active_project_path(&state)?;
    let remote = if remote.is_empty() {
        "origin".to_string()
    } else {
        remote
    };
    match tag_name {
        Some(name) => {
            let label = format!("Push tag {}", name);
            let tag_ref = format!("refs/tags/{}", name);
            let id = task_manager
                .spawn(label, "git", &["push", &remote, &tag_ref], &cwd, true)
                .await;
            Ok(id)
        }
        None => {
            let label = "Push all tags".to_string();
            let id = task_manager
                .spawn(label, "git", &["push", &remote, "--tags"], &cwd, true)
                .await;
            Ok(id)
        }
    }
}

#[cfg(test)]
mod tests {
    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn create_lightweight_tag_at_head() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        let result = repo.create_tag("v0.1.0", None).unwrap();
        assert!(
            result.success,
            "lightweight tag should succeed, stderr: {}",
            result.stderr
        );

        let tags = repo.tags().unwrap();
        let v010 = tags
            .iter()
            .find(|t| t.name == "v0.1.0")
            .expect("tag exists");
        assert!(!v010.annotated, "expected lightweight tag");
    }

    #[test]
    fn create_annotated_tag_has_message() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        let result = repo.create_tag("v0.2.0", Some("release cut")).unwrap();
        assert!(result.success);

        let tags = repo.tags().unwrap();
        let v020 = tags
            .iter()
            .find(|t| t.name == "v0.2.0")
            .expect("tag exists");
        assert!(v020.annotated, "expected annotated tag");
        assert!(
            v020.message.contains("release cut"),
            "annotated tag message missing, got {:?}",
            v020.message
        );
    }

    #[test]
    fn delete_nonexistent_tag_reports_failure() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let result = repo.delete_tag("does-not-exist").unwrap();
        assert!(
            !result.success,
            "delete of missing tag should be reported as failure"
        );
    }
}
