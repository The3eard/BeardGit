//! Tag listing, creation, deletion, and push commands.

use std::sync::Arc;

use task_runner::{TaskId, TaskManager};
use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Return all tags in the active repository, sorted newest-version-first.
#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> Result<Vec<git_engine::TagInfo>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.tags().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// List tags with pagination, sorted newest-version-first.
#[tauri::command]
pub async fn list_tags_paginated(
    per_page: u32,
    page: u32,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::TagInfo>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.tags_paginated(per_page, page)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Search all tags by name substring (case-insensitive).
#[tauri::command]
pub async fn search_tags(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::TagInfo>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.search_tags(&query).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Create a new tag in the active repository.
///
/// - If `message` is provided and non-empty, creates an annotated tag.
/// - Otherwise creates a lightweight tag.
/// - If `target` is empty, tags HEAD.
#[tauri::command]
pub async fn create_tag(
    name: String,
    target: String,
    message: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let msg = message.as_deref().filter(|m| !m.is_empty());
        let result = if target.is_empty() {
            repo.create_tag(&name, msg).map_err(|e| e.to_string())?
        } else {
            match msg {
                Some(m) => repo
                    .git_cmd(&["tag", "-a", &name, &target, "-m", m])
                    .map_err(|e| e.to_string())?,
                None => repo
                    .git_cmd(&["tag", &name, &target])
                    .map_err(|e| e.to_string())?,
            }
        };
        if result.success {
            Ok(())
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Delete a local tag by name.
#[tauri::command]
pub async fn delete_tag(name: String, state: State<'_, AppState>) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.delete_tag(&name).map_err(|e| e.to_string())?;
        if result.success {
            Ok(())
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Push a tag to a remote as a background task.
#[tauri::command]
pub async fn push_tag(
    tag_name: Option<String>,
    remote: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
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
