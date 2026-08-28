//! Staging and unstaging commands (index manipulation).

use mutation_events::MutationKind;
use tauri::{AppHandle, State};
use tracing::instrument;

use super::helpers::*;
use crate::ipc_error::IpcError;
use crate::state::AppState;

/// Stage a specific list of files by path (equivalent to `git add <paths>`).
///
/// # Parameters
/// - `paths` – Workspace-relative paths to stage.
///
/// Runs on a blocking thread — index writes (and untracked-file reads) can be
/// sizeable and must not block the Tauri async runtime / freeze the UI.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::staging::stage_files")]
pub async fn stage_files(
    paths: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StagingChange, || async move {
        run_blocking(move || {
            let repo = git_engine::Repository::open(repo_path)?;
            repo.stage_files(&paths).map_err(IpcError::from)
        })
        .await
    })
    .await
}

/// Unstage a specific list of files (equivalent to `git restore --staged <paths>`).
///
/// # Parameters
/// - `paths` – Workspace-relative paths to unstage.
///
/// Runs on a blocking thread — index writes must not block the Tauri async
/// runtime / freeze the UI.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::staging::unstage_files")]
pub async fn unstage_files(
    paths: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StagingChange, || async move {
        run_blocking(move || {
            let repo = git_engine::Repository::open(repo_path)?;
            repo.unstage_files(&paths).map_err(IpcError::from)
        })
        .await
    })
    .await
}

/// Stage all modified and untracked files (equivalent to `git add -A`).
///
/// Runs on a blocking thread — `add_all` walks the whole working tree, which
/// must not block the Tauri async runtime / freeze the UI on a large repo.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::staging::stage_all")]
pub async fn stage_all(state: State<'_, AppState>, app: AppHandle) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StagingChange, || async move {
        run_blocking(move || {
            let repo = git_engine::Repository::open(repo_path)?;
            repo.stage_all().map_err(IpcError::from)
        })
        .await
    })
    .await
}

/// Unstage all staged changes (equivalent to `git restore --staged .`).
///
/// Runs on a blocking thread — a mixed reset rewrites the index and must not
/// block the Tauri async runtime / freeze the UI.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::staging::unstage_all")]
pub async fn unstage_all(state: State<'_, AppState>, app: AppHandle) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StagingChange, || async move {
        run_blocking(move || {
            let repo = git_engine::Repository::open(repo_path)?;
            repo.unstage_all().map_err(IpcError::from)
        })
        .await
    })
    .await
}

/// Stage selected hunks or individual lines from the working directory.
///
/// # Parameters
/// - `path` – Workspace-relative file path.
/// - `selections` – Which hunks/lines to stage.
/// - `context_lines` – The context the displayed diff was fetched with. A
///   selection is positional, so the backend has to cut the file into the
///   same hunks the user was looking at; `null` means libgit2's default of
///   3. See [`git_engine::Repository::stage_hunks`].
///
/// Runs on a blocking thread — building and applying the patch must not block
/// the Tauri async runtime / freeze the UI.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::staging::stage_hunks")]
pub async fn stage_hunks(
    path: String,
    selections: Vec<git_engine::HunkSelection>,
    context_lines: Option<u32>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StagingChange, || async move {
        run_blocking(move || {
            let repo = git_engine::Repository::open(repo_path)?;
            repo.stage_hunks(&path, &selections, context_lines)
                .map_err(IpcError::from)
        })
        .await
    })
    .await
}

/// Unstage selected hunks or individual lines from the index.
///
/// # Parameters
/// - `path` – Workspace-relative file path.
/// - `selections` – Which hunks/lines to unstage.
/// - `context_lines` – The context the displayed diff was fetched with. A
///   selection is positional, so the backend has to cut the file into the
///   same hunks the user was looking at; `null` means libgit2's default of
///   3. See [`git_engine::Repository::stage_hunks`].
///
/// Runs on a blocking thread — building and applying the patch must not block
/// the Tauri async runtime / freeze the UI.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::staging::unstage_hunks")]
pub async fn unstage_hunks(
    path: String,
    selections: Vec<git_engine::HunkSelection>,
    context_lines: Option<u32>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StagingChange, || async move {
        run_blocking(move || {
            let repo = git_engine::Repository::open(repo_path)?;
            repo.unstage_hunks(&path, &selections, context_lines)
                .map_err(IpcError::from)
        })
        .await
    })
    .await
}

/// Discard unstaged changes for whole files.
///
/// For tracked files with working-tree modifications the working copy is
/// reset to match the index (`git checkout -- <path>` semantics — staged
/// content is preserved). For untracked files the file is deleted from disk.
///
/// # Parameters
/// - `paths` – Workspace-relative paths to discard.
///
/// Runs on a blocking thread — resetting the working tree / deleting files is
/// real disk I/O and must not block the Tauri async runtime / freeze the UI.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::staging::discard_files")]
pub async fn discard_files(
    paths: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StagingChange, || async move {
        run_blocking(move || {
            let repo = git_engine::Repository::open(repo_path)?;
            repo.discard_files(&paths).map_err(IpcError::from)
        })
        .await
    })
    .await
}

/// Discard selected hunks or individual lines from the working directory.
///
/// # Parameters
/// - `path` – Workspace-relative file path.
/// - `selections` – Which hunks/lines to discard.
/// - `context_lines` – The context the displayed diff was fetched with. A
///   selection is positional, so the backend has to cut the file into the
///   same hunks the user was looking at; `null` means libgit2's default of
///   3. See [`git_engine::Repository::stage_hunks`].
///
/// Runs on a blocking thread — building and applying the patch to the working
/// tree must not block the Tauri async runtime / freeze the UI.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::staging::discard_hunks")]
pub async fn discard_hunks(
    path: String,
    selections: Vec<git_engine::HunkSelection>,
    context_lines: Option<u32>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repo_path = get_active_project_path(&state)?;
    with_mutation_guard_async(&state, &app, MutationKind::StagingChange, || async move {
        run_blocking(move || {
            let repo = git_engine::Repository::open(repo_path)?;
            repo.discard_hunks(&path, &selections, context_lines)
                .map_err(IpcError::from)
        })
        .await
    })
    .await
}

#[cfg(test)]
mod tests {
    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn stage_files_adds_new_file_to_index() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        std::fs::write(path.join("new.txt"), "hello\n").unwrap();
        let repo = Repository::open(&path).unwrap();

        repo.stage_files(&["new.txt".to_string()]).unwrap();

        let statuses = repo.file_statuses().unwrap();
        let entry = statuses
            .iter()
            .find(|s| s.path == "new.txt")
            .expect("new.txt should appear in status");
        assert!(entry.is_staged, "file should be staged");
    }

    #[test]
    fn unstage_files_removes_staged_file_from_index() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        std::fs::write(path.join("new.txt"), "hello\n").unwrap();
        let repo = Repository::open(&path).unwrap();
        repo.stage_files(&["new.txt".to_string()]).unwrap();

        repo.unstage_files(&["new.txt".to_string()]).unwrap();

        let statuses = repo.file_statuses().unwrap();
        let entry = statuses
            .iter()
            .find(|s| s.path == "new.txt")
            .expect("new.txt should still show as untracked");
        assert!(!entry.is_staged, "file should no longer be staged");
    }

    #[test]
    fn unstage_files_on_headless_repo_errors() {
        // `unstage_files` needs HEAD to reset the index entry to. A brand-new
        // repo with no commits has no HEAD, so the call fails.
        let dir = tempfile::TempDir::new().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        // seed identity so unstage gets past signature lookup
        let mut config = git_repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "t@t.io").unwrap();
        drop(config);
        drop(git_repo);

        let repo = Repository::open(dir.path()).unwrap();
        let err = repo.unstage_files(&["anything.txt".to_string()]).err();
        assert!(err.is_some(), "unstage on HEAD-less repo should error");
    }

    #[test]
    fn stage_all_stages_new_and_modified_files() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        // Create a fresh untracked file and ensure `stage_all` picks it up.
        std::fs::write(path.join("fresh.txt"), "x\n").unwrap();
        let repo = Repository::open(&path).unwrap();
        repo.stage_all().unwrap();

        let statuses = repo.file_statuses().unwrap();
        assert!(
            statuses
                .iter()
                .any(|s| s.path == "fresh.txt" && s.is_staged),
            "expected fresh.txt to be staged, got: {statuses:?}"
        );
    }

    #[test]
    fn stage_hunks_with_no_matching_file_errors() {
        // When the file isn't in the working-tree diff (because the file
        // doesn't exist on disk), `stage_hunks` surfaces a "No diff found"
        // error. This stands in for the "partial-hunk staging" error path —
        // we don't exercise the happy hunk-staging path here because it
        // requires assembling a valid patch, which is covered by
        // git-engine's own tests.
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = repo.stage_hunks("missing.txt", &[], None).err();
        assert!(err.is_some(), "stage_hunks on a missing file should error");
    }
}
