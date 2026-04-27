//! Gitignore read, write, and pattern management commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Read the content of the repository's root `.gitignore` file.
///
/// Returns an empty string if the file does not exist.
#[tauri::command]
pub fn read_gitignore(state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.read_gitignore().map_err(|e| e.to_string())
    })
}

/// Write the full content of the repository's `.gitignore` file.
///
/// Creates the file if it does not exist.
#[tauri::command]
pub fn write_gitignore(content: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.write_gitignore(&content).map_err(|e| e.to_string())
    })
}

/// Add a single pattern to the repository's `.gitignore` file.
///
/// Checks for duplicates before appending. Creates the file if needed.
#[tauri::command]
pub fn add_gitignore_pattern(pattern: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.add_gitignore_pattern(&pattern)
            .map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    //! Drive the `Repository::*_gitignore*` helpers through fixture repos.

    use git_engine::Repository;
    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn read_gitignore_on_missing_file_returns_empty_string() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let body = repo.read_gitignore().unwrap();
        assert!(
            body.is_empty(),
            "no .gitignore present -> empty string, got {body:?}"
        );
    }

    #[test]
    fn write_then_read_gitignore_roundtrips() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        repo.write_gitignore("*.log\ntarget/\n").unwrap();
        let body = repo.read_gitignore().unwrap();
        assert_eq!(body, "*.log\ntarget/\n");
    }

    #[test]
    fn add_gitignore_pattern_appends_new_pattern() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        repo.add_gitignore_pattern("target/").unwrap();
        let body = repo.read_gitignore().unwrap();
        assert!(
            body.lines().any(|l| l.trim() == "target/"),
            "pattern should be present, got {body:?}"
        );
    }

    #[test]
    fn add_gitignore_pattern_is_idempotent() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        repo.add_gitignore_pattern("node_modules/").unwrap();
        repo.add_gitignore_pattern("node_modules/").unwrap();
        let body = repo.read_gitignore().unwrap();
        let count = body.lines().filter(|l| l.trim() == "node_modules/").count();
        assert_eq!(
            count, 1,
            "adding the same pattern twice should not duplicate, got body {body:?}"
        );
    }
}
