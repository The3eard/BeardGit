//! Clean (untracked/ignored file removal) commands.

use mutation_events::MutationKind;
use tauri::{AppHandle, State};
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Preview untracked/ignored files that would be removed by `git clean`.
#[tauri::command]
#[instrument(skip(state), name = "cmd::clean::dry_run")]
pub fn clean_dry_run(
    include_directories: bool,
    include_ignored: bool,
    only_ignored: bool,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CleanItem>, String> {
    with_active_repo(&state, |repo| {
        repo.clean_dry_run(include_directories, include_ignored, only_ignored)
            .map_err(|e| e.to_string())
    })
}

/// Permanently remove the specified paths from the working directory.
///
/// Returns the number of items successfully deleted. Wraps the work inside a
/// [`MutationGuard`][mutation_events::MutationGuard] scope so that on success
/// a `project-mutated` event with [`MutationKind::StagingChange`] is emitted.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::clean::paths")]
pub fn clean_paths(
    paths: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<u32, String> {
    with_mutation_guard(&state, &app, MutationKind::StagingChange, || {
        with_active_repo(&state, |repo| {
            repo.clean_paths(&paths).map_err(|e| e.to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    //! Exercise `Repository::clean_dry_run` and `clean_paths` directly.

    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn clean_dry_run_on_clean_repo_returns_empty() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let items = repo.clean_dry_run(false, false, false).unwrap();
        assert!(
            items.is_empty(),
            "repo with no untracked files should have nothing to clean, got {items:?}"
        );
    }

    #[test]
    fn clean_dry_run_lists_untracked_file() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        std::fs::write(path.join("junk.txt"), "garbage\n").unwrap();
        let repo = Repository::open(&path).unwrap();
        let items = repo.clean_dry_run(false, false, false).unwrap();
        assert!(
            items.iter().any(|i| i.path.ends_with("junk.txt")),
            "clean -n should list junk.txt, got {items:?}"
        );
    }

    #[test]
    fn clean_paths_removes_listed_untracked_files() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        std::fs::write(path.join("a.tmp"), "1\n").unwrap();
        std::fs::write(path.join("b.tmp"), "2\n").unwrap();
        let repo = Repository::open(&path).unwrap();
        let removed = repo
            .clean_paths(&["a.tmp".to_string(), "b.tmp".to_string()])
            .unwrap();
        assert_eq!(removed, 2, "both files should be counted as removed");
        assert!(!path.join("a.tmp").exists(), "a.tmp should be gone");
        assert!(!path.join("b.tmp").exists(), "b.tmp should be gone");
    }

    #[test]
    fn clean_paths_rejects_escape_outside_repo() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        // Paths that canonicalize outside the repo must be filtered out;
        // result is `Ok(0)` rather than a crash / escape.
        let removed = repo.clean_paths(&["../outside".to_string()]).unwrap();
        assert_eq!(removed, 0, "paths outside the repo should be skipped");
    }
}
