//! Conflict detection, resolution, abort, and continue commands.

use mutation_events::MutationKind;
use tauri::{AppHandle, State};
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Return the current conflict state and list of conflicted file paths.
#[tauri::command]
#[instrument(skip(state), name = "cmd::conflict::get_status")]
pub fn get_conflict_status(
    state: State<'_, AppState>,
) -> Result<git_engine::ConflictStatus, String> {
    with_active_repo(&state, |repo| {
        repo.conflict_status().map_err(|e| e.to_string())
    })
}

/// Get the ours/theirs/base content of a conflicted file from the index.
#[tauri::command]
#[instrument(skip(state), name = "cmd::conflict::get_file_contents")]
pub fn get_conflict_file_contents(
    path: String,
    state: State<'_, AppState>,
) -> Result<git_engine::ConflictFileContents, String> {
    with_active_repo(&state, |repo| {
        repo.get_conflict_file_contents(&path)
            .map_err(|e| e.to_string())
    })
}

/// Write resolved content to disk and mark the file as resolved in the index.
///
/// Wraps the work inside a [`MutationGuard`][mutation_events::MutationGuard]
/// scope so that on success a `project-mutated` event with
/// [`MutationKind::StagingChange`] is emitted — index-only mutation.
#[tauri::command]
#[instrument(skip(state, content, app), name = "cmd::conflict::write_resolved")]
pub fn write_resolved_file(
    path: String,
    content: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    with_mutation_guard(&state, &app, MutationKind::StagingChange, || {
        with_active_repo(&state, |repo| {
            repo.write_resolved_file(&path, &content)
                .map_err(|e| e.to_string())
        })
    })
}

/// Abort the current mid-operation git state (merge/rebase/cherry-pick/revert).
#[tauri::command]
#[instrument(skip(state), name = "cmd::conflict::abort")]
pub async fn abort_operation(state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let conflict_state = repo.detect_conflict_state();
        let result = match conflict_state {
            git_engine::ConflictState::Merging => repo.abort_merge().map_err(|e| e.to_string())?,
            git_engine::ConflictState::Rebasing => {
                repo.abort_rebase().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::CherryPicking => {
                repo.abort_cherry_pick().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Reverting => {
                repo.abort_revert().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::None => {
                return Err("No operation in progress to abort".to_string());
            }
        };
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Continue the current mid-operation git state after conflicts are resolved.
#[tauri::command]
#[instrument(skip(state), name = "cmd::conflict::continue")]
pub async fn continue_operation(state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let status = repo.conflict_status().map_err(|e| e.to_string())?;
        if status.state == git_engine::ConflictState::None {
            return Err("No operation in progress to continue".to_string());
        }
        if !status.can_continue {
            return Err("Cannot continue: unresolved conflicts remain".to_string());
        }
        let result = match status.state {
            git_engine::ConflictState::Merging => {
                repo.continue_merge().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Rebasing => {
                repo.continue_rebase().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::CherryPicking => {
                repo.continue_cherry_pick().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Reverting => {
                repo.continue_revert().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::None => unreachable!(),
        };
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
    use git_engine::test_support::{create_repo_with_conflict, create_repo_with_n_commits};

    #[test]
    fn get_file_contents_returns_both_sides() {
        let (_tmp, path) =
            create_repo_with_conflict("base contents\n", "our edit\n", "their edit\n", "foo.txt");
        let repo = Repository::open(&path).unwrap();

        let contents = repo.get_conflict_file_contents("foo.txt").unwrap();
        assert_eq!(contents.base, "base contents\n");
        assert_eq!(contents.ours, "our edit\n");
        assert_eq!(contents.theirs, "their edit\n");

        let status = repo.conflict_status().unwrap();
        assert!(status.conflicted_files.contains(&"foo.txt".to_string()));
        assert_ne!(status.state, git_engine::ConflictState::None);
    }

    #[test]
    fn write_resolved_file_clears_conflict_entry() {
        let (_tmp, path) =
            create_repo_with_conflict("base\n", "ours\n", "theirs\n", "conflicted.txt");
        let repo = Repository::open(&path).unwrap();

        repo.write_resolved_file("conflicted.txt", "merged\n")
            .unwrap();

        // Working-tree content should be the merged text.
        let content = std::fs::read_to_string(path.join("conflicted.txt")).unwrap();
        assert_eq!(content, "merged\n");

        // Index should no longer flag the file as conflicted.
        let status = repo.conflict_status().unwrap();
        assert!(
            !status
                .conflicted_files
                .contains(&"conflicted.txt".to_string()),
            "conflicted_files should no longer contain the resolved file, got {:?}",
            status.conflicted_files
        );
    }

    #[test]
    fn get_file_contents_on_non_conflicted_file_errors() {
        // No merge in progress → no conflict entries at all → lookup fails.
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = repo.get_conflict_file_contents("anything.txt").err();
        assert!(
            err.is_some(),
            "reading conflict contents when no conflict exists should error"
        );
    }
}
