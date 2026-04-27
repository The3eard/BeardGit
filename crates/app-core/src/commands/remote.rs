//! Remote fetch, pull, push, rename, and remove commands.

use std::sync::Arc;

use mutation_events::MutationKind;
use task_runner::{SpawnOptions, TaskId, TaskKind, TaskManager};
use tauri::{AppHandle, State};
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Fetch all updates from a remote as a background task.
///
/// Spawns `git fetch <remote>` via the task manager and returns immediately
/// with the task ID. Progress streams to the task popover and the unified
/// tasks drawer (via the `task://update` bridge).
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::remote::fetch")]
pub async fn fetch_remote(
    remote: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Fetch {}", remote);
    let id = task_manager
        .spawn_with_options(SpawnOptions {
            label,
            command: "git",
            args: &["fetch", &remote],
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::GitFetch,
            stdin: None,
        })
        .await;

    Ok(id)
}

/// Pull a branch from a remote (merge strategy) as a background task.
///
/// Spawns `git pull <remote> <branch>` via the task manager.
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::remote::pull")]
pub async fn pull_remote(
    remote: String,
    branch: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Pull {}/{}", remote, branch);
    let id = task_manager
        .spawn_with_options(SpawnOptions {
            label,
            command: "git",
            args: &["pull", &remote, &branch],
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::GitPull,
            stdin: None,
        })
        .await;

    Ok(id)
}

/// Push a branch to a remote as a background task.
///
/// Spawns `git push -u [--force-with-lease] <remote> <branch>` via
/// the task manager. `-u` is always passed so the first push
/// establishes the upstream; subsequent pushes are no-ops on that
/// flag. `--force-with-lease` is added only when `force` is true.
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::remote::push")]
pub async fn push_remote(
    remote: String,
    branch: String,
    force: bool,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let mut args: Vec<&str> = vec!["push", "-u"];
    if force {
        args.push("--force-with-lease");
    }
    args.push(&remote);
    args.push(&branch);

    let label = if force {
        format!("Push (force) {}/{}", remote, branch)
    } else {
        format!("Push {}/{}", remote, branch)
    };
    let id = task_manager
        .spawn_with_options(SpawnOptions {
            label,
            command: "git",
            args: &args,
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::GitPush,
            stdin: None,
        })
        .await;

    Ok(id)
}

/// Renames a remote in the active repository.
///
/// Equivalent to `git remote rename <old_name> <new_name>`. Returns an error
/// if `old_name` does not exist or `new_name` is already taken.
#[tauri::command]
#[instrument(skip(state), name = "cmd::remote::rename")]
pub async fn rename_remote(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.rename_remote(&old_name, &new_name)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Removes a remote from the active repository.
///
/// Equivalent to `git remote remove <name>`. Returns an error if the remote
/// does not exist.
///
/// Wraps the work inside a [`MutationGuard`][mutation_events::MutationGuard]
/// scope so that on success a `project-mutated` event with
/// [`MutationKind::RemoteRemove`] is emitted.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::remote::remove")]
pub async fn remove_remote(
    name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::RemoteRemove, || async move {
        tokio::task::spawn_blocking(move || {
            let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
            repo.remove_remote(&name).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    })
    .await
}

/// Ensures a commit SHA is present in the local object database.
///
/// Used by the PR diff view to materialize fork-head commits before the
/// frontend calls `get_file_at_commit`. Behaviour:
/// 1. `git cat-file -e <sha>` — if success, returns immediately.
/// 2. Otherwise `git fetch <remote_url || "origin"> <sha>` via
///    [`TaskManager`], streamed to the tasks drawer.
/// 3. Re-checks presence; errors if still missing.
///
/// Returns `Ok(())` on presence (no task spawned) or after the fetch
/// completes; returns `Err` with a human-readable message otherwise.
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::remote::ensure_commit_local")]
pub async fn ensure_commit_local(
    sha: String,
    remote_url: Option<String>,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<(), String> {
    let cwd = get_active_project_path(&state)?;
    if commit_exists_locally(&cwd, &sha) {
        return Ok(());
    }

    let remote = remote_url.unwrap_or_else(|| "origin".to_string());
    let short = &sha[..sha.len().min(8)];
    let label = format!("Fetch {short}");
    let id = task_manager
        .spawn_with_options(SpawnOptions {
            label,
            command: "git",
            args: &["fetch", &remote, &sha],
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::GitFetch,
            stdin: None,
        })
        .await;

    // Wait for the task to reach a terminal state before re-checking.
    task_manager
        .wait_for_terminal(id)
        .await
        .map_err(|e| e.to_string())?;

    if !commit_exists_locally(&cwd, &sha) {
        return Err(format!(
            "commit {} not found after fetching from {remote}",
            &sha[..sha.len().min(8)]
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn remotes_list_is_empty_on_fresh_repo() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let names: Vec<String> = repo
            .inner()
            .remotes()
            .unwrap()
            .iter()
            .flatten()
            .map(String::from)
            .collect();
        assert!(names.is_empty());
    }

    #[test]
    fn remotes_list_reflects_git_remote_add() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let git_repo = git2::Repository::open(&path).unwrap();
        git_repo
            .remote("origin", "https://example.invalid/x/y.git")
            .unwrap();
        drop(git_repo);

        let repo = Repository::open(&path).unwrap();
        let origin = repo.inner().find_remote("origin").unwrap();
        assert_eq!(origin.name(), Some("origin"));
        assert_eq!(origin.url(), Some("https://example.invalid/x/y.git"));
    }

    #[test]
    fn rename_remote_on_nonexistent_remote_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = repo.rename_remote("nope", "whatever").err();
        assert!(
            err.is_some(),
            "renaming a non-existent remote must surface an error"
        );
    }

    #[test]
    fn remove_remote_on_nonexistent_remote_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = repo.remove_remote("nope").err();
        assert!(err.is_some());
    }

    #[test]
    fn ensure_commit_local_early_returns_when_commit_exists() {
        use crate::commands::helpers::commit_exists_locally;
        use git_engine::test_support::create_repo_with_n_commits;
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = git2::Repository::open(&path).unwrap();
        let head = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string();
        drop(repo);
        let present = commit_exists_locally(&path, &head);
        assert!(present, "freshly committed SHA must be cat-file-e present");
    }

    #[test]
    fn ensure_commit_local_reports_missing_for_random_sha() {
        use crate::commands::helpers::commit_exists_locally;
        use git_engine::test_support::create_repo_with_n_commits;
        let (_tmp, path) = create_repo_with_n_commits(1);
        let present = commit_exists_locally(&path, "deadbeefdeadbeefdeadbeefdeadbeef00000000");
        assert!(!present);
    }

    #[test]
    fn rename_remote_renames_existing_entry() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let git_repo = git2::Repository::open(&path).unwrap();
        git_repo
            .remote("origin", "https://example.invalid/x/y.git")
            .unwrap();
        drop(git_repo);

        let repo = Repository::open(&path).unwrap();
        repo.rename_remote("origin", "canonical").unwrap();

        let names: Vec<String> = repo
            .inner()
            .remotes()
            .unwrap()
            .iter()
            .flatten()
            .map(String::from)
            .collect();
        assert!(names.contains(&"canonical".to_string()));
        assert!(!names.contains(&"origin".to_string()));
    }
}
