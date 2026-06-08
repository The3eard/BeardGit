//! Stash push, pop, apply, drop, list, and show commands.

use mutation_events::MutationKind;
use tauri::{AppHandle, State};
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Push the current working-tree changes onto the stash stack.
///
/// # Parameters
/// - `message` – Optional stash description (equivalent to `git stash push -m <msg>`).
///
/// # Returns
/// The stdout of `git stash push` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::stash::push")]
pub async fn stash_push(
    message: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::Stash, || async move {
        tokio::task::spawn_blocking(move || {
            let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
            let result = repo
                .stash_push(message.as_deref())
                .map_err(|e| e.to_string())?;
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

/// Pop (apply and drop) a stash entry.
///
/// # Parameters
/// - `index` – Zero-based stash index to pop (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash pop` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::stash::pop")]
pub async fn stash_pop(
    index: Option<usize>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StashPop, || async move {
        tokio::task::spawn_blocking(move || {
            let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
            let result = repo.stash_pop(index).map_err(|e| e.to_string())?;
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

/// Return a list of stash entry descriptions (one per stash entry).
///
/// Each string corresponds to a line from `git stash list`.
#[tauri::command]
pub async fn stash_list(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.stash_list().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Apply a stash entry without removing it.
///
/// # Parameters
/// - `index` – Zero-based stash index to apply (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash apply` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::stash::apply")]
pub async fn stash_apply(
    index: Option<usize>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    // `apply` does NOT remove the stash entry — use the generic `Stash` kind,
    // not `StashPop` (which would mislabel the event as a removal).
    with_mutation_guard_async(&state, &app, MutationKind::Stash, || async move {
        tokio::task::spawn_blocking(move || {
            let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
            let result = repo.stash_apply(index).map_err(|e| e.to_string())?;
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

/// Restore a single file from a stash entry into the working directory.
///
/// # Parameters
/// - `index` – Zero-based stash index.
/// - `path` – Repo-relative file path to restore.
///
/// # Returns
/// The stdout of `git restore` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::stash::apply_file")]
pub async fn stash_apply_file(
    index: usize,
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    // `apply` does NOT remove the stash entry — use the generic `Stash` kind.
    with_mutation_guard_async(&state, &app, MutationKind::Stash, || async move {
        tokio::task::spawn_blocking(move || {
            let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
            let result = repo
                .stash_apply_file(index, &path)
                .map_err(|e| e.to_string())?;
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

/// Drop a stash entry without applying it.
///
/// # Parameters
/// - `index` – Zero-based stash index to drop (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash drop` on success, or stderr as an error.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::stash::drop")]
pub async fn stash_drop(
    index: Option<usize>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StashDrop, || async move {
        tokio::task::spawn_blocking(move || {
            let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
            let result = repo.stash_drop(index).map_err(|e| e.to_string())?;
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

/// Return structured stash entries with parsed metadata.
///
/// Each entry includes index, message, branch, timestamp, and commit OID.
#[tauri::command]
pub async fn stash_entries(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::StashEntry>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.stash_entries().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return the diff of a stash entry as structured `FileDiff` objects.
///
/// # Parameters
/// - `index` – Zero-based stash index (defaults to 0, i.e. the latest stash).
#[tauri::command]
pub async fn stash_show_parsed(
    index: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileDiff>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.stash_show_parsed(index).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use git_engine::Repository;
    use git_engine::test_support::{create_repo_with_n_commits, create_repo_with_stash};

    #[test]
    fn stash_push_creates_entry_with_message() {
        // Seed a repo with a tracked file we can modify and stash.
        let (_tmp, path) = create_repo_with_n_commits(1);
        std::fs::write(path.join("tracked.txt"), "v1\n").unwrap();
        let repo = Repository::open(&path).unwrap();
        repo.stage_files(&["tracked.txt".to_string()]).unwrap();
        repo.create_commit("add tracked").unwrap();
        std::fs::write(path.join("tracked.txt"), "v2\n").unwrap();

        let result = repo.stash_push(Some("work in progress")).unwrap();
        assert!(
            result.success,
            "stash push should succeed, stderr: {}",
            result.stderr
        );
        let entries = repo.stash_list().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0].contains("work in progress"),
            "stash list should include the message, got {entries:?}"
        );
    }

    #[test]
    fn stash_pop_restores_stashed_changes() {
        let (_tmp, path) = create_repo_with_stash(1);
        let repo = Repository::open(&path).unwrap();
        assert_eq!(repo.stash_list().unwrap().len(), 1);

        let result = repo.stash_pop(None).unwrap();
        assert!(
            result.success,
            "stash pop should succeed, stderr: {}",
            result.stderr
        );
        assert_eq!(repo.stash_list().unwrap().len(), 0);

        // The modified file should be back in the working tree.
        let content = std::fs::read_to_string(path.join("f0.txt")).unwrap();
        assert_eq!(content, "stash-0\n");
    }

    #[test]
    fn stash_drop_on_empty_stack_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let result = repo.stash_drop(None).unwrap();
        assert!(
            !result.success,
            "stash drop on empty stack should report failure"
        );
    }
}
