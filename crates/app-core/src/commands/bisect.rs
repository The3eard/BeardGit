//! Bisect workflow commands.

use tauri::State;
use tracing::instrument;

use super::helpers::get_active_project_path;
use crate::state::AppState;

/// Start a bisect session, optionally providing the initial bad and good commits.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::start")]
pub async fn bisect_start(
    bad: Option<String>,
    good: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_start(&cwd, bad.as_deref(), good.as_deref())
}

/// Mark a commit (or current HEAD) as good.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::good")]
pub async fn bisect_good(
    commit: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_good(&cwd, commit.as_deref())
}

/// Mark a commit (or current HEAD) as bad.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::bad")]
pub async fn bisect_bad(
    commit: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_bad(&cwd, commit.as_deref())
}

/// Skip the current commit.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::skip")]
pub async fn bisect_skip(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_skip(&cwd)
}

/// Reset (end) the bisect session.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::reset")]
pub async fn bisect_reset(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_reset(&cwd)
}

/// Get the current bisect session state.
#[tauri::command]
pub async fn bisect_get_state(
    state: State<'_, AppState>,
) -> Result<git_engine::BisectState, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_state(&cwd)
}

/// Get the bisect log.
#[tauri::command]
pub async fn bisect_get_log(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_log(&cwd)
}

/// Run an automated bisect with a test command.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::run_auto")]
pub async fn bisect_run_auto(
    test_command: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    git_engine::bisect::bisect_run(&cwd, &test_command)
}

#[cfg(test)]
mod tests {
    //! Drive the `git_engine::bisect` free-function surface that these
    //! commands wrap. We use repos with >=2 commits so `git bisect start`
    //! has a real range to work on.

    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn bisect_state_on_fresh_repo_reports_inactive() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let state = git_engine::bisect::bisect_state(&path).expect("bisect_state");
        assert!(!state.active, "no BISECT_START -> inactive session");
        assert!(state.good_commits.is_empty());
        assert!(state.bad_commits.is_empty());
    }

    #[test]
    fn bisect_start_then_reset_ends_session() {
        let (_tmp, path) = create_repo_with_n_commits(3);
        // Start empty (no initial good/bad) — sets up the bisect metadata.
        git_engine::bisect::bisect_start(&path, None, None).expect("bisect start");
        assert!(
            path.join(".git").join("BISECT_START").exists(),
            "BISECT_START should exist after start"
        );
        git_engine::bisect::bisect_reset(&path).expect("bisect reset");
        let state = git_engine::bisect::bisect_state(&path).unwrap();
        assert!(
            !state.active,
            "after reset, bisect_state should report inactive"
        );
    }

    #[test]
    fn bisect_log_outside_session_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        // `git bisect log` with no session in progress exits non-zero.
        let err = git_engine::bisect::bisect_log(&path).err();
        assert!(err.is_some(), "bisect log with no session should error");
    }
}
