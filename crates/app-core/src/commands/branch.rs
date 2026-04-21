//! Branch creation, checkout, deletion, merge, and rebase commands.

use mutation_events::MutationKind;
use tauri::{AppHandle, State};
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Create a new local branch pointing at the current HEAD.
///
/// Wraps [`git_engine::Repository::create_branch`] inside a
/// [`MutationGuard`][mutation_events::MutationGuard] scope so that on success a
/// `project-mutated` event with [`MutationKind::BranchCreate`] is emitted.
///
/// # Parameters
/// - `name` – Name for the new branch.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::branch::create")]
pub fn create_branch(
    name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::BranchCreate, || {
        with_active_repo(&state, |repo| {
            repo.create_branch(&name).map_err(|e| e.to_string())
        })
    })
}

/// Create a new branch at a specific commit.
///
/// Wraps [`git_engine::Repository::create_branch_at`] inside a
/// [`MutationGuard`][mutation_events::MutationGuard] scope so that on success a
/// `project-mutated` event with [`MutationKind::BranchCreate`] is emitted.
///
/// # Parameters
/// - `name` – Name for the new branch.
/// - `oid` – Commit OID where the branch should point.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::branch::create_at")]
pub fn create_branch_at(
    name: String,
    oid: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::BranchCreate, || {
        with_active_repo(&state, |repo| {
            repo.create_branch_at(&name, &oid)
                .map_err(|e| e.to_string())
        })
    })
}

/// Checkout a specific commit (detached HEAD).
///
/// Wraps [`git_engine::Repository::checkout_detached`] inside a
/// [`MutationGuard`][mutation_events::MutationGuard] scope so that on success a
/// `project-mutated` event with [`MutationKind::Checkout`] is emitted.
///
/// # Parameters
/// - `oid` – Commit OID to checkout.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::branch::checkout_detached")]
pub fn checkout_detached(
    oid: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::Checkout, || {
        with_active_repo(&state, |repo| {
            repo.checkout_detached(&oid).map_err(|e| e.to_string())
        })
    })
}

/// Delete a local branch by name.
///
/// Wraps [`git_engine::Repository::delete_branch`] inside a
/// [`MutationGuard`][mutation_events::MutationGuard] scope so that on success a
/// `project-mutated` event with [`MutationKind::BranchDelete`] is emitted.
///
/// # Parameters
/// - `name` – Name of the branch to delete.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::branch::delete")]
pub fn delete_branch(
    name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::BranchDelete, || {
        with_active_repo(&state, |repo| {
            repo.delete_branch(&name).map_err(|e| e.to_string())
        })
    })
}

/// Switch the working tree to an existing local branch.
///
/// Wraps [`git_engine::Repository::checkout_branch`] inside a
/// [`MutationGuard`][mutation_events::MutationGuard] scope so that on success a
/// `project-mutated` event with [`MutationKind::Checkout`] is emitted.
///
/// # Parameters
/// - `name` – Name of the branch to check out.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::branch::checkout")]
pub fn checkout_branch(
    name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::Checkout, || {
        with_active_repo(&state, |repo| {
            repo.checkout_branch(&name).map_err(|e| e.to_string())
        })
    })
}

/// Merge a branch into the current branch via the git CLI.
///
/// Wraps [`git_engine::Repository::merge_branch`] inside a
/// [`MutationGuard`][mutation_events::MutationGuard] scope so that on success a
/// `project-mutated` event with [`MutationKind::Merge`] is emitted.
///
/// # Parameters
/// - `branch` – Name of the branch to merge into HEAD.
///
/// # Returns
/// The stdout of `git merge` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::branch::merge")]
pub async fn merge_branch(
    branch: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::Merge, || async move {
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
    })
    .await
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

#[cfg(test)]
mod tests {
    use git_engine::Repository;
    use git_engine::test_support::{create_repo_with_branches, create_repo_with_n_commits};

    #[test]
    fn create_branch_at_head_adds_new_branch() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        repo.create_branch("feature/foo").unwrap();

        assert!(
            repo.branches()
                .unwrap()
                .iter()
                .any(|b| b.name == "feature/foo"),
            "new branch should be listed"
        );
    }

    #[test]
    fn create_branch_with_existing_name_errors() {
        let (_tmp, path) = create_repo_with_branches(&["already"]);
        let repo = Repository::open(&path).unwrap();
        let err = repo.create_branch("already").err();
        assert!(
            err.is_some(),
            "creating a duplicate branch name should error"
        );
    }

    #[test]
    fn delete_branch_removes_local_ref() {
        // `delete_branch` in git-engine wraps `git2::Branch::delete`, which
        // ignores the merged/unmerged distinction — behaviour-equivalent to
        // `git branch -D` (force=true in the command plan).
        let (_tmp, path) = create_repo_with_branches(&["to-delete"]);
        let repo = Repository::open(&path).unwrap();

        repo.delete_branch("to-delete").unwrap();

        assert!(
            !repo
                .branches()
                .unwrap()
                .iter()
                .any(|b| b.name == "to-delete"),
            "deleted branch should be gone"
        );
    }

    #[test]
    fn checkout_branch_switches_head() {
        let (_tmp, path) = create_repo_with_branches(&["feat-a", "feat-b"]);
        let repo = Repository::open(&path).unwrap();

        repo.checkout_branch("feat-a").unwrap();
        assert_eq!(
            repo.get_current_branch().unwrap().as_deref(),
            Some("feat-a")
        );

        repo.checkout_branch("feat-b").unwrap();
        assert_eq!(
            repo.get_current_branch().unwrap().as_deref(),
            Some("feat-b")
        );
    }

    #[test]
    fn checkout_unknown_branch_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        assert!(repo.checkout_branch("does-not-exist").is_err());
    }

    #[test]
    fn create_branch_at_oid_points_to_that_commit() {
        let (_tmp, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();
        let commits = repo.walk_commits(0, 3).unwrap();
        let oldest = &commits.last().unwrap().oid;

        repo.create_branch_at("old-anchor", oldest).unwrap();
        let branch = repo
            .branches()
            .unwrap()
            .into_iter()
            .find(|b| b.name == "old-anchor")
            .expect("branch exists");
        assert_eq!(&branch.oid, oldest);
    }
}
