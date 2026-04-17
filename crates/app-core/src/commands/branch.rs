//! Branch creation, checkout, deletion, merge, and rebase commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Create a new local branch pointing at the current HEAD.
///
/// # Parameters
/// - `name` – Name for the new branch.
#[tauri::command]
#[instrument(skip(state), name = "cmd::branch::create")]
pub fn create_branch(name: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.create_branch(&name).map_err(|e| e.to_string())
    })
}

/// Create a new branch at a specific commit.
///
/// # Parameters
/// - `name` – Name for the new branch.
/// - `oid` – Commit OID where the branch should point.
#[tauri::command]
#[instrument(skip(state), name = "cmd::branch::create_at")]
pub fn create_branch_at(
    name: String,
    oid: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.create_branch_at(&name, &oid)
            .map_err(|e| e.to_string())
    })
}

/// Checkout a specific commit (detached HEAD).
///
/// # Parameters
/// - `oid` – Commit OID to checkout.
#[tauri::command]
#[instrument(skip(state), name = "cmd::branch::checkout_detached")]
pub fn checkout_detached(oid: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.checkout_detached(&oid).map_err(|e| e.to_string())
    })
}

/// Delete a local branch by name.
///
/// # Parameters
/// - `name` – Name of the branch to delete.
#[tauri::command]
#[instrument(skip(state), name = "cmd::branch::delete")]
pub fn delete_branch(name: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.delete_branch(&name).map_err(|e| e.to_string())
    })
}

/// Switch the working tree to an existing local branch.
///
/// # Parameters
/// - `name` – Name of the branch to check out.
#[tauri::command]
#[instrument(skip(state), name = "cmd::branch::checkout")]
pub fn checkout_branch(name: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.checkout_branch(&name).map_err(|e| e.to_string())
    })
}

/// Merge a branch into the current branch via the git CLI.
///
/// # Parameters
/// - `branch` – Name of the branch to merge into HEAD.
///
/// # Returns
/// The stdout of `git merge` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state), name = "cmd::branch::merge")]
pub async fn merge_branch(branch: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.merge_branch(&branch).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Rebase the current branch onto another branch or commit via the git CLI.
///
/// # Parameters
/// - `onto` – Branch name or commit SHA to rebase onto.
///
/// # Returns
/// The stdout of `git rebase` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state), name = "cmd::branch::rebase")]
pub async fn rebase_branch(onto: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.rebase_branch(&onto).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
