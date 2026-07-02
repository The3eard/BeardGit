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
#[instrument(skip(state), name = "cmd::diff::commit_files")]
pub async fn get_commit_files(
    oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitFileChange>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.commit_files(&oid).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return files changed between two arbitrary commits.
///
/// # Parameters
/// - `from_oid` – SHA of the base commit.
/// - `to_oid` – SHA of the target commit.
#[tauri::command]
#[instrument(skip(state), name = "cmd::diff::diff_between_commits")]
pub async fn get_diff_between_commits(
    from_oid: String,
    to_oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitFileChange>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.diff_commits(&from_oid, &to_oid)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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

/// Return the structured diff for **every** file in a commit in a single
/// libgit2 walk. Used by detail panes to avoid the N-subprocess fan-out of
/// `get_commit_file_diff` per file.
#[tauri::command]
#[instrument(skip(state), name = "cmd::diff::commit_full_diff")]
pub async fn get_commit_full_diff(
    oid: String,
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, git_engine::FileDiff>, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.commit_full_diff(&oid).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Structured result for [`get_file_at_commit`].
///
/// Uses an internally-tagged enum so the IPC payload serialises as
/// `{ "kind": "text", "data": "..." }`, `{ "kind": "binary" }`, or
/// `{ "kind": "too_large", "size": 12345678 }`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FileAtCommitResult {
    /// Text content, UTF-8 lossy decoded.
    Text {
        /// The file contents.
        data: String,
    },
    /// Blob contained a NUL byte in its first 8 KB.
    Binary,
    /// Blob exceeded `MAX_FILE_AT_COMMIT_BYTES` and was not loaded.
    /// The frontend should render a "file too large to diff"
    /// placeholder rather than try to fetch it.
    TooLarge {
        /// Byte size of the blob.
        size: usize,
    },
}

/// Returns raw file content at a commit, or the tagged sentinel
/// `{"kind": "binary"}` if the blob is binary.
///
/// # Parameters
/// - `oid` – Full or abbreviated commit SHA.
/// - `path` – Repo-relative file path.
///
/// # Returns
/// `{ "kind": "text", "data": "..." }` on success, `{ "kind": "binary" }` for
/// binary blobs, or an `Err` string for missing OID / path.
#[tauri::command]
pub fn get_file_at_commit(
    oid: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<FileAtCommitResult, String> {
    with_active_repo(&state, |repo| match repo.get_file_at_commit(&oid, &path) {
        Ok(content) => Ok(FileAtCommitResult::Text { data: content }),
        Err(git_engine::GitError::Binary) => Ok(FileAtCommitResult::Binary),
        Err(git_engine::GitError::FileTooLarge { size }) => {
            Ok(FileAtCommitResult::TooLarge { size })
        }
        Err(e) => Err(e.to_string()),
    })
}

#[cfg(test)]
mod serde_shape {
    use super::FileAtCommitResult;

    #[test]
    fn text_variant_serializes_with_data_field() {
        let v = FileAtCommitResult::Text { data: "hi".into() };
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("\"data\":\"hi\""), "got: {s}");
    }

    #[test]
    fn binary_variant_serializes_with_kind_only() {
        let v = FileAtCommitResult::Binary;
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, r#"{"kind":"binary"}"#);
    }
}

/// Structured result for [`get_file_workdir`] / [`get_file_index`].
///
/// Same tagged shape as [`FileAtCommitResult`] so the frontend renders the
/// binary / too-large placeholders for the workdir and index sides exactly
/// as it does for the commit side.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FileContentResult {
    /// Text content, UTF-8 lossy decoded.
    Text {
        /// The file contents.
        data: String,
    },
    /// Content contained a NUL byte in its first 8 KB.
    Binary,
    /// Content exceeded the per-file cap and was not loaded.
    TooLarge {
        /// Byte size of the content.
        size: usize,
    },
}

/// Map a `get_file_*` result into the tagged [`FileContentResult`].
fn tag_file_content(
    result: Result<String, git_engine::GitError>,
) -> Result<FileContentResult, String> {
    match result {
        Ok(content) => Ok(FileContentResult::Text { data: content }),
        Err(git_engine::GitError::Binary) => Ok(FileContentResult::Binary),
        Err(git_engine::GitError::FileTooLarge { size }) => {
            Ok(FileContentResult::TooLarge { size })
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Returns raw file content from the working directory, or a tagged marker
/// for binary / oversized files (see [`FileContentResult`]).
///
/// # Parameters
/// - `path` – Repo-relative file path.
#[tauri::command]
pub fn get_file_workdir(
    path: String,
    state: State<'_, AppState>,
) -> Result<FileContentResult, String> {
    with_active_repo(&state, |repo| {
        tag_file_content(repo.get_file_workdir(&path))
    })
}

/// Returns raw file content from the index (staged version), or a tagged
/// marker for binary / oversized files (see [`FileContentResult`]).
///
/// # Parameters
/// - `path` – Repo-relative file path.
#[tauri::command]
pub fn get_file_index(
    path: String,
    state: State<'_, AppState>,
) -> Result<FileContentResult, String> {
    with_active_repo(&state, |repo| tag_file_content(repo.get_file_index(&path)))
}

/// Return the unstaged diff between the working tree and the index.
///
/// Equivalent to `git diff` (without `--cached`). The whole-response byte
/// budget is enforced so a working tree full of large changed files can't
/// balloon a single IPC payload — files past the budget come back with
/// empty hunks and `truncated: true`.
#[tauri::command]
pub fn get_diff_workdir(state: State<'_, AppState>) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| {
        let mut files = repo.diff_workdir().map_err(|e| e.to_string())?;
        git_engine::enforce_response_budget(&mut files, git_engine::MAX_DIFF_RESPONSE_BYTES);
        Ok(files)
    })
}

/// Return the staged diff between the index and HEAD.
///
/// Equivalent to `git diff --cached`. See [`get_diff_workdir`] for the
/// whole-response budget.
#[tauri::command]
pub fn get_diff_index(state: State<'_, AppState>) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| {
        let mut files = repo.diff_index().map_err(|e| e.to_string())?;
        git_engine::enforce_response_budget(&mut files, git_engine::MAX_DIFF_RESPONSE_BYTES);
        Ok(files)
    })
}

/// Full hunks/lines diff for a single file, fetched lazily when the user
/// opens it in the Changes view. `staged` selects the index-vs-HEAD diff
/// (`true`) or the workdir-vs-index diff (`false`). Returns `null` when the
/// file has no change on that side.
#[tauri::command]
pub fn get_diff_file(
    path: String,
    staged: bool,
    state: State<'_, AppState>,
) -> Result<Option<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_single_file(&path, staged)
            .map_err(|e| e.to_string())
    })
}

/// Cheap per-file change stats (name/status + add/del counts, no hunks) for
/// the working tree. Powers the Changes list without streaming every hunk
/// on each mutation.
#[tauri::command]
pub fn get_diff_stats_workdir(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileDiffStat>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_stats_workdir().map_err(|e| e.to_string())
    })
}

/// Cheap per-file change stats for the index (staged changes) vs HEAD.
/// See [`get_diff_stats_workdir`].
#[tauri::command]
pub fn get_diff_stats_index(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileDiffStat>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_stats_index().map_err(|e| e.to_string())
    })
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
