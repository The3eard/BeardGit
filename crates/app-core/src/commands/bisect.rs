//! Bisect workflow commands.

use std::sync::Arc;

use task_runner::{TaskId, TaskManager};
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
    tokio::task::spawn_blocking(move || {
        git_engine::bisect::bisect_start(&cwd, bad.as_deref(), good.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Mark a commit (or current HEAD) as good.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::good")]
pub async fn bisect_good(
    commit: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || git_engine::bisect::bisect_good(&cwd, commit.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

/// Mark a commit (or current HEAD) as bad.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::bad")]
pub async fn bisect_bad(
    commit: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || git_engine::bisect::bisect_bad(&cwd, commit.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

/// Skip the current commit.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::skip")]
pub async fn bisect_skip(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || git_engine::bisect::bisect_skip(&cwd))
        .await
        .map_err(|e| e.to_string())?
}

/// Reset (end) the bisect session.
#[tauri::command]
#[instrument(skip(state), name = "cmd::bisect::reset")]
pub async fn bisect_reset(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || git_engine::bisect::bisect_reset(&cwd))
        .await
        .map_err(|e| e.to_string())?
}

/// Get the current bisect session state.
#[tauri::command]
pub async fn bisect_get_state(
    state: State<'_, AppState>,
) -> Result<git_engine::BisectState, String> {
    let cwd = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || git_engine::bisect::bisect_state(&cwd))
        .await
        .map_err(|e| e.to_string())?
}

/// Get the bisect log.
#[tauri::command]
pub async fn bisect_get_log(state: State<'_, AppState>) -> Result<String, String> {
    let cwd = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || git_engine::bisect::bisect_log(&cwd))
        .await
        .map_err(|e| e.to_string())?
}

/// Run an automated bisect with a test command (background task, returns TaskId).
///
/// `git bisect run` executes the user-supplied test command across many
/// commits — potentially minutes of work — so it runs as a managed,
/// cancellable [`TaskManager`] task that streams its line-oriented output
/// through the task lifecycle events instead of blocking the runtime.
#[tauri::command]
#[instrument(skip(state, task_manager), name = "cmd::bisect::run_auto")]
pub async fn bisect_run_auto(
    test_command: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    // Mirror `git_engine::bisect::bisect_run`: split on whitespace and pass
    // the parts as the command `git bisect run` should invoke per step.
    let parts: Vec<&str> = test_command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty test command".into());
    }
    let mut args: Vec<&str> = vec!["bisect", "run"];
    args.extend_from_slice(&parts);

    let label = format!("Bisect run: {test_command}");
    let id = task_manager.spawn(label, "git", &args, &cwd, true).await;

    Ok(id)
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

#[cfg(test)]
mod run_auto_task_tests {
    //! `bisect_run_auto` is a thin `TaskManager::spawn` wrapper (returns a
    //! `TaskId` for a cancellable background task). Constructing the full
    //! `AppState` here is impractical, so — like `submodule::update_*` — we
    //! cover the run-auto contract at the task-runner level: a long-running,
    //! cancellable task standing in for `git bisect run <slow test command>`
    //! must stop when cancelled from the UI.

    use std::sync::Arc;
    use std::time::Duration;

    use task_runner::{OutputLine, TaskEventSink, TaskId, TaskInfo, TaskManager, TaskStatus};

    struct NoopSink;
    #[async_trait::async_trait]
    impl TaskEventSink for NoopSink {
        async fn on_task_started(&self, _info: TaskInfo) {}
        async fn on_task_output(&self, _task_id: TaskId, _line: OutputLine) {}
        async fn on_task_completed(&self, _info: TaskInfo) {}
        async fn on_task_failed(&self, _info: TaskInfo) {}
        async fn on_task_cancelled(&self, _info: TaskInfo) {}
    }

    #[tokio::test]
    async fn bisect_run_auto_task_is_cancellable() {
        let manager = Arc::new(TaskManager::new(Arc::new(NoopSink)));
        let cwd = std::env::temp_dir();

        // `sleep 30` stands in for a runaway `git bisect run` test command:
        // spawned cancellable, exactly as `bisect_run_auto` does.
        let id = manager
            .spawn("Bisect run: sleep 30".into(), "sleep", &["30"], &cwd, true)
            .await;

        // Let the child start before cancelling.
        tokio::time::sleep(Duration::from_millis(200)).await;
        manager.cancel(id).await.expect("cancel should succeed");

        let status = tokio::time::timeout(Duration::from_secs(5), manager.wait_for_terminal(id))
            .await
            .expect("cancelled task should reach a terminal state promptly")
            .expect("task should still be in the registry");
        assert!(
            matches!(status, TaskStatus::Cancelled),
            "expected Cancelled, got {status:?}"
        );
    }
}
