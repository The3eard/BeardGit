//! Diff and file content commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// List files changed by a commit, including their change status and patch.
///
/// # Parameters
/// - `oid` – Full or abbreviated commit SHA.
#[tauri::command]
pub fn get_commit_files(
    oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitFileChange>, String> {
    with_active_repo(&state, |repo| {
        repo.commit_files(&oid).map_err(|e| e.to_string())
    })
}

/// Return files changed between two arbitrary commits.
///
/// # Parameters
/// - `from_oid` – SHA of the base commit.
/// - `to_oid` – SHA of the target commit.
#[tauri::command]
pub fn get_diff_between_commits(
    from_oid: String,
    to_oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitFileChange>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_commits(&from_oid, &to_oid)
            .map_err(|e| e.to_string())
    })
}

/// Return the full diff (hunks + lines) for a single file in a commit.
#[tauri::command]
#[instrument(skip(state), name = "cmd::diff::commit_file_diff")]
pub async fn get_commit_file_diff(
    oid: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileDiff>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.commit_file_diff(&oid, &path)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Returns raw file content at a specific commit.
///
/// # Parameters
/// - `oid` – Full or abbreviated commit SHA.
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw UTF-8 file content (binary blobs are lossy-decoded), or an error string
/// if the OID or path is invalid.
#[tauri::command]
pub fn get_file_at_commit(
    oid: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_at_commit(&oid, &path)
            .map_err(|e| e.to_string())
    })
}

/// Returns raw file content from the working directory.
///
/// # Parameters
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw file content, or an IO error string if the file does not exist.
#[tauri::command]
pub fn get_file_workdir(path: String, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_workdir(&path).map_err(|e| e.to_string())
    })
}

/// Returns raw file content from the index (staged version).
///
/// # Parameters
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw staged file content, or an error string if the file is not staged.
#[tauri::command]
pub fn get_file_index(path: String, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_index(&path).map_err(|e| e.to_string())
    })
}

/// Return the unstaged diff between the working tree and the index.
///
/// Equivalent to `git diff` (without `--cached`).
#[tauri::command]
pub fn get_diff_workdir(state: State<'_, AppState>) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_workdir().map_err(|e| e.to_string())
    })
}

/// Return the staged diff between the index and HEAD.
///
/// Equivalent to `git diff --cached`.
#[tauri::command]
pub fn get_diff_index(state: State<'_, AppState>) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| repo.diff_index().map_err(|e| e.to_string()))
}

#[cfg(test)]
mod tests {
    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;

    /// Build a repo with one commit, then modify a tracked file so
    /// `diff_workdir` has something to report. Returns the repo and the path
    /// of the modified file.
    fn repo_with_workdir_change() -> (tempfile::TempDir, std::path::PathBuf) {
        let (tmp, path) = create_repo_with_n_commits(1);
        // Commit a file we can modify.
        std::fs::write(path.join("tracked.txt"), "v1\n").unwrap();
        let repo = Repository::open(&path).unwrap();
        repo.stage_files(&["tracked.txt".to_string()]).unwrap();
        repo.create_commit("add tracked").unwrap();
        // Now introduce a workdir change.
        std::fs::write(path.join("tracked.txt"), "v2\n").unwrap();
        (tmp, path)
    }

    #[test]
    fn diff_workdir_returns_hunks_for_changed_file() {
        let (_tmp, path) = repo_with_workdir_change();
        let repo = Repository::open(&path).unwrap();
        let diffs = repo.diff_workdir().unwrap();
        let tracked = diffs
            .iter()
            .find(|d| d.path == "tracked.txt")
            .expect("expected a diff entry for tracked.txt");
        assert!(
            !tracked.hunks.is_empty(),
            "diff should contain at least one hunk"
        );
    }

    #[test]
    fn get_file_at_commit_on_missing_path_errors() {
        let (_tmp, path) = repo_with_workdir_change();
        let repo = Repository::open(&path).unwrap();
        let head_oid = repo.inner().head().unwrap().target().unwrap().to_string();
        let err = repo.get_file_at_commit(&head_oid, "not-a-file.txt").err();
        assert!(
            err.is_some(),
            "reading a path that isn't in the commit should error"
        );
    }

    #[test]
    fn diff_commits_between_two_commits_returns_combined_changes() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        // Commit A: add a.txt.
        std::fs::write(path.join("a.txt"), "alpha\n").unwrap();
        repo.stage_files(&["a.txt".to_string()]).unwrap();
        let a = repo.create_commit("add a").unwrap();

        // Commit B: add b.txt on top of A.
        std::fs::write(path.join("b.txt"), "bravo\n").unwrap();
        repo.stage_files(&["b.txt".to_string()]).unwrap();
        let b = repo.create_commit("add b").unwrap();

        let changes = repo.diff_commits(&a, &b).unwrap();
        let paths: Vec<_> = changes.iter().map(|c| c.path.clone()).collect();
        assert!(
            paths.iter().any(|p| p == "b.txt"),
            "diff A..B should include b.txt, got {paths:?}"
        );
    }

    #[test]
    fn get_file_workdir_on_missing_file_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = repo.get_file_workdir("does-not-exist.txt").err();
        assert!(err.is_some(), "reading a missing workdir file should error");
    }
}
