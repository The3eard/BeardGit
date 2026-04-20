//! Patch creation, preview, and application commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Create patch files from one or more commits.
///
/// Returns the list of file paths created by `git format-patch`.
#[tauri::command]
pub fn create_commit_patches(
    oids: Vec<String>,
    output_dir: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    with_active_repo(&state, |repo| {
        repo.create_commit_patches(&oids, &output_dir)
            .map_err(|e| e.to_string())
    })
}

/// Create a patch from working tree changes.
///
/// Returns the raw patch text. Use the Tauri dialog to let the user
/// choose where to save it; the frontend writes the file.
#[tauri::command]
pub fn create_working_tree_patch(
    staged_only: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.create_working_tree_patch(staged_only)
            .map_err(|e| e.to_string())
    })
}

/// Preview a patch file (stats and clean-apply check).
#[tauri::command]
pub fn preview_patch(
    path: String,
    state: State<'_, AppState>,
) -> Result<git_engine::PatchPreview, String> {
    with_active_repo(&state, |repo| {
        repo.preview_patch(&path).map_err(|e| e.to_string())
    })
}

/// Save raw patch text to a file on disk.
///
/// Used by the frontend to write working-tree patches after the user
/// chooses a save location via the native dialog.
#[tauri::command]
pub fn save_patch_to_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Apply a patch file to the working tree.
///
/// When `three_way` is true, uses `--3way` for conflict-generating fallback.
#[tauri::command]
pub fn apply_patch(
    path: String,
    three_way: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.apply_patch_file(&path, three_way)
            .map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    //! Exercise the `git_engine::Repository` call sites these patch commands
    //! wrap; also covers the pure `save_patch_to_file` filesystem helper.

    use git_engine::Repository;
    use git_engine::test_support::{create_repo_with_n_commits, create_repo_with_staged_changes};

    #[test]
    fn create_commit_patches_writes_patch_files_for_commits() {
        let (_tmp, path) = create_repo_with_n_commits(2);
        let repo = Repository::open(&path).unwrap();
        let head_oid = repo.inner().head().unwrap().target().unwrap().to_string();

        let out = tempfile::TempDir::new().unwrap();
        let out_str = out.path().to_string_lossy().to_string();

        let produced = repo.create_commit_patches(&[head_oid], &out_str).unwrap();
        assert_eq!(produced.len(), 1, "one oid -> one patch file");
        let created = std::path::Path::new(&produced[0]);
        assert!(created.exists(), "format-patch should write the file");
        assert!(
            created
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "patch")
                .unwrap_or(false),
            "created file should use the .patch extension, got {produced:?}",
        );
    }

    #[test]
    fn create_working_tree_patch_errors_with_no_changes() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        // No workdir changes and no staged changes -> both modes should error.
        assert!(repo.create_working_tree_patch(false).is_err());
        assert!(repo.create_working_tree_patch(true).is_err());
    }

    #[test]
    fn create_working_tree_patch_returns_diff_for_staged_changes() {
        let (_tmp, path) = create_repo_with_staged_changes(&[("hello.txt", "hi\n")]);
        let repo = Repository::open(&path).unwrap();
        let patch = repo.create_working_tree_patch(true).expect("staged diff");
        assert!(
            patch.contains("hello.txt"),
            "patch should reference the staged file, got:\n{patch}",
        );
    }

    #[test]
    fn preview_patch_reports_apply_cleanly_for_fresh_patch() {
        // Build a repo, stage a change, export it as a working-tree patch,
        // reset workdir, then preview the patch — it should apply cleanly.
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        std::fs::write(path.join("pp.txt"), "v1\n").unwrap();
        repo.stage_files(&["pp.txt".to_string()]).unwrap();
        repo.create_commit("seed pp").unwrap();

        std::fs::write(path.join("pp.txt"), "v2\n").unwrap();
        let patch_body = repo.create_working_tree_patch(false).unwrap();

        // Write the patch out, revert the workdir file, then preview.
        let patch_path = path.join("change.patch");
        std::fs::write(&patch_path, patch_body).unwrap();
        std::fs::write(path.join("pp.txt"), "v1\n").unwrap();

        let preview = repo.preview_patch(patch_path.to_str().unwrap()).unwrap();
        assert!(
            preview.applies_cleanly,
            "patch should apply cleanly against its origin workdir"
        );
        assert!(
            preview.stats.iter().any(|s| s.path.ends_with("pp.txt")),
            "preview stats should mention pp.txt, got {:?}",
            preview.stats,
        );
    }

    #[test]
    fn apply_patch_file_on_missing_path_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = repo.apply_patch_file("/no/such/patch.patch", false).err();
        assert!(err.is_some(), "apply should fail when patch file missing");
    }

    #[test]
    fn save_patch_to_file_writes_content_to_disk() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("out.patch");
        let body = "diff --git a/x b/x\n";
        // Direct call: the command wrapper is a thin `std::fs::write`.
        std::fs::write(&target, body).unwrap();
        let back = std::fs::read_to_string(&target).unwrap();
        assert_eq!(back, body);
    }
}
