//! File-editor commands: workdir CRUD + binary-aware reads with a size cap.
//!
//! All paths arriving from the frontend are repo-relative, forward-slashed.
//! The underlying `git_engine` helpers reject absolute paths, `..`
//! segments, and anything that resolves outside the working tree.
//!
//! Reads cap content at 2 MB. Larger files return [`ReadWorkdirFileResult::TooLarge`]
//! without reading their contents into memory; binary files (NUL byte in
//! the first 8 KB) return [`ReadWorkdirFileResult::Binary`].
//!
//! Every mutating command runs inside [`with_mutation_guard`] with
//! [`MutationKind::StagingChange`] so the watcher fan-out fires once on
//! success — `StagingChange` is the right fit for "the index or worktree
//! changed without touching refs", which covers writes / creates /
//! renames / deletes from the editor.

use std::io::Read;

use mutation_events::MutationKind;
use tauri::{AppHandle, State};
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Soft cap on workdir reads. Files larger than this return
/// [`ReadWorkdirFileResult::TooLarge`] without their content; the editor
/// surfaces a "file too large to edit here" placeholder instead.
const MAX_READ_BYTES: u64 = 2 * 1024 * 1024;

/// Tagged result for [`read_workdir_file`].
///
/// Three terminal shapes:
/// - `Text { data, size }` – the file is plain UTF-8(ish) and fits the cap.
/// - `Binary { size }` – first 8 KB contained a NUL byte.
/// - `TooLarge { size }` – exceeds [`MAX_READ_BYTES`]; content not loaded.
#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadWorkdirFileResult {
    /// Plain text content. Lossy UTF-8 decode of the on-disk bytes.
    Text {
        /// File content as a string (lossy UTF-8).
        data: String,
        /// File size in bytes.
        size: u64,
    },
    /// File is binary (NUL byte in the first 8 KB). Content not returned.
    Binary {
        /// File size in bytes.
        size: u64,
    },
    /// File exceeds [`MAX_READ_BYTES`]. Content not loaded into memory.
    TooLarge {
        /// File size in bytes.
        size: u64,
    },
}

/// Read a working-directory file, capped at 2 MB and binary-aware.
///
/// Returns one of the three [`ReadWorkdirFileResult`] variants. The size
/// is read first via `metadata()` so an oversized file is rejected
/// without ever loading its bytes.
#[tauri::command]
#[instrument(skip(state), name = "cmd::file_editor::read_workdir_file")]
pub fn read_workdir_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<ReadWorkdirFileResult, String> {
    with_active_repo(&state, |repo| {
        let full = git_engine::file_content::validate_repo_relative_path(repo.path(), &path)
            .map_err(|e| e.to_string())?;
        let meta = std::fs::metadata(&full).map_err(|e| e.to_string())?;
        if !meta.is_file() {
            return Err(format!("not a regular file: {path}"));
        }
        let size = meta.len();
        if size > MAX_READ_BYTES {
            return Ok(ReadWorkdirFileResult::TooLarge { size });
        }

        // Binary sniff on the first 8 KB before slurping the whole file.
        // We do this directly here (rather than going through
        // `Repository::get_file_workdir`) so the size we report and the
        // bytes we sniff come from the same `metadata()` call.
        let mut file = std::fs::File::open(&full).map_err(|e| e.to_string())?;
        let mut head = [0u8; 8192];
        let read_n = file.read(&mut head).map_err(|e| e.to_string())?;
        if head[..read_n].contains(&0u8) {
            return Ok(ReadWorkdirFileResult::Binary { size });
        }

        // Re-open + read the full content. (We can't simply continue
        // reading into a buffer because we might already have consumed
        // some bytes for the sniff and the resulting concatenation gets
        // awkward to express atomically; a fresh open is clearer.)
        let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
        let data = String::from_utf8_lossy(&bytes).into_owned();
        Ok(ReadWorkdirFileResult::Text { data, size })
    })
}

/// Write `content` to a working-directory file (atomic replace).
///
/// Wraps the work in a [`MutationGuard`][mutation_events::MutationGuard]
/// scope so a successful write emits `project-mutated` with
/// [`MutationKind::StagingChange`].
#[tauri::command]
#[instrument(
    skip(state, app, content),
    name = "cmd::file_editor::write_workdir_file"
)]
pub fn write_workdir_file(
    path: String,
    content: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::StagingChange, || {
        with_active_repo(&state, |repo| {
            repo.write_file_workdir(&path, &content)
                .map_err(|e| e.to_string())
        })
    })
}

/// List entries from the working directory.
///
/// See [`git_engine::Repository::list_workdir_tree`] for full semantics.
/// `prefix` is repo-relative; pass `None` for a full recursive walk.
#[tauri::command]
#[instrument(skip(state), name = "cmd::file_editor::list_workdir_tree")]
pub fn list_workdir_tree(
    prefix: Option<String>,
    max_entries: u32,
    respect_gitignore: bool,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::WorkdirTreeEntry>, String> {
    with_active_repo(&state, |repo| {
        repo.list_workdir_tree(prefix.as_deref(), max_entries as usize, respect_gitignore)
            .map_err(|e| e.to_string())
    })
}

/// Create a new file or directory at `path`. Errors if `path` already
/// exists. Mutating: emits a `staging_change` mutation event.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::file_editor::create_workdir_path")]
pub fn create_workdir_path(
    path: String,
    is_directory: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::StagingChange, || {
        with_active_repo(&state, |repo| {
            repo.create_workdir_path(&path, is_directory)
                .map_err(|e| e.to_string())
        })
    })
}

/// Rename a file or directory. Errors if the source does not exist or
/// the destination already exists. Mutating: emits a `staging_change`
/// mutation event.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::file_editor::rename_workdir_path")]
pub fn rename_workdir_path(
    from_path: String,
    to_path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::StagingChange, || {
        with_active_repo(&state, |repo| {
            repo.rename_workdir_path(&from_path, &to_path)
                .map_err(|e| e.to_string())
        })
    })
}

/// Delete a file or directory. Errors if the path does not exist.
/// Mutating: emits a `staging_change` mutation event.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::file_editor::delete_workdir_path")]
pub fn delete_workdir_path(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::StagingChange, || {
        with_active_repo(&state, |repo| {
            repo.delete_workdir_path(&path).map_err(|e| e.to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    //! Unit tests for the file-editor module.
    //!
    //! These exercise the underlying `git_engine` repo methods rather
    //! than the Tauri commands themselves — the commands are thin
    //! `with_active_repo` / `with_mutation_guard` wrappers, and
    //! constructing a real `State<AppState>` in a unit test is more
    //! plumbing than signal. The end-to-end shape is covered by the
    //! Vitest spec on the TS wrappers + the `git-engine` tests.
    use super::{MAX_READ_BYTES, ReadWorkdirFileResult};
    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;
    use std::fs;
    use std::io::Read;

    /// Mirror of the read-side of [`super::read_workdir_file`] — we lift
    /// the body out so tests can drive it directly without a live
    /// `AppState`. Any change to the public command must keep this in
    /// step.
    fn read_helper(repo: &Repository, path: &str) -> Result<ReadWorkdirFileResult, String> {
        let full = git_engine::file_content::validate_repo_relative_path(repo.path(), path)
            .map_err(|e| e.to_string())?;
        let meta = std::fs::metadata(&full).map_err(|e| e.to_string())?;
        if !meta.is_file() {
            return Err(format!("not a regular file: {path}"));
        }
        let size = meta.len();
        if size > MAX_READ_BYTES {
            return Ok(ReadWorkdirFileResult::TooLarge { size });
        }
        let mut file = std::fs::File::open(&full).map_err(|e| e.to_string())?;
        let mut head = [0u8; 8192];
        let n = file.read(&mut head).map_err(|e| e.to_string())?;
        if head[..n].contains(&0u8) {
            return Ok(ReadWorkdirFileResult::Binary { size });
        }
        let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
        let data = String::from_utf8_lossy(&bytes).into_owned();
        Ok(ReadWorkdirFileResult::Text { data, size })
    }

    #[test]
    fn round_trip_write_then_read_text() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        repo.write_file_workdir("notes.md", "# hello\n").unwrap();
        let result = read_helper(&repo, "notes.md").unwrap();
        match result {
            ReadWorkdirFileResult::Text { data, size } => {
                assert_eq!(data, "# hello\n");
                assert_eq!(size, b"# hello\n".len() as u64);
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn read_returns_binary_for_files_with_nul_byte() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::write(path.join("blob.bin"), [0x89, b'P', b'N', b'G', 0x00, 0x01]).unwrap();
        let repo = Repository::open(&path).unwrap();
        let result = read_helper(&repo, "blob.bin").unwrap();
        assert!(
            matches!(result, ReadWorkdirFileResult::Binary { .. }),
            "expected Binary, got {result:?}"
        );
    }

    #[test]
    fn read_returns_too_large_for_oversized_files() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        // Just over the cap.
        let big = vec![b'a'; (MAX_READ_BYTES as usize) + 1];
        fs::write(path.join("huge.txt"), &big).unwrap();
        let repo = Repository::open(&path).unwrap();
        let result = read_helper(&repo, "huge.txt").unwrap();
        match result {
            ReadWorkdirFileResult::TooLarge { size } => {
                assert_eq!(size, (MAX_READ_BYTES) + 1);
            }
            other => panic!("expected TooLarge, got {other:?}"),
        }
    }

    #[test]
    fn read_rejects_path_traversal() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = read_helper(&repo, "../etc/passwd").unwrap_err();
        assert!(err.contains("invalid path") || err.contains(".."));
    }

    #[test]
    fn list_tree_happy_path() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::write(path.join("a.txt"), "a").unwrap();
        let repo = Repository::open(&path).unwrap();
        let entries = repo.list_workdir_tree(None, 100, false).unwrap();
        assert!(entries.iter().any(|e| e.name == "a.txt"));
    }

    #[test]
    fn create_rename_delete_cycle() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        repo.create_workdir_path("dir/inner.txt", false).unwrap();
        assert!(path.join("dir/inner.txt").is_file());

        repo.rename_workdir_path("dir/inner.txt", "dir/renamed.txt")
            .unwrap();
        assert!(!path.join("dir/inner.txt").exists());
        assert!(path.join("dir/renamed.txt").is_file());

        repo.delete_workdir_path("dir/renamed.txt").unwrap();
        assert!(!path.join("dir/renamed.txt").exists());
        // Containing directory remains.
        assert!(path.join("dir").is_dir());

        repo.delete_workdir_path("dir").unwrap();
        assert!(!path.join("dir").exists());
    }
}
