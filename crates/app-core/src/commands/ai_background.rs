//! Tauri commands for the AI Background Worktree feature.
//!
//! Six commands: start, cancel, list, get, discard_worktree, open_terminal.
//! All delegate to [`AiBackgroundCoordinator`] which holds the shared state.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use ai_provider::{AiBackgroundRunStatus, AiProviderKind, AiSession};
use serde::{Deserialize, Serialize};
use tauri::State;
use terminal::{SessionId, TerminalConfig, TerminalManager};

use super::helpers::get_active_project_path;
use crate::ai_background::{AiBackgroundCoordinator, StartArgs};
use crate::state::AppState;

// ─── Argument & return types ─────────────────────────────────────────────────

/// Request payload for [`ai_start_background_run`].
///
/// All fields come from the Create Background Run dialog. Only `provider`
/// and at least one of `prompt` / `skill` / `saved_prompt_path` are required
/// by the frontend; the backend treats them all as optional and enforces
/// validation in the coordinator.
#[derive(Debug, Clone, Deserialize)]
pub struct StartBackgroundRunRequest {
    pub provider: String,
    pub base_branch: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub skill: Option<String>,
    #[serde(default)]
    pub saved_prompt_path: Option<String>,
    #[serde(default)]
    pub resume_session_id: Option<String>,
    #[serde(default)]
    pub worktree_slug_override: Option<String>,
}

/// Response returned from [`ai_start_background_run`].
#[derive(Debug, Clone, Serialize)]
pub struct StartBackgroundRunResponse {
    pub session_id: String,
    pub task_id: Option<u64>,
    pub worktree_path: String,
    pub status: AiBackgroundRunStatus,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_kind(provider: &str) -> Result<AiProviderKind, String> {
    match provider {
        "claude_code" => Ok(AiProviderKind::ClaudeCode),
        "codex" => Ok(AiProviderKind::Codex),
        "open_code" => Ok(AiProviderKind::OpenCode),
        other => Err(format!("unknown AI provider: {other}")),
    }
}

fn make_provider(kind: AiProviderKind) -> Box<dyn ai_provider::AiProvider> {
    match kind {
        AiProviderKind::ClaudeCode => Box::new(claude_code::ClaudeCodeProvider::new()),
        AiProviderKind::Codex => Box::new(codex::CodexProvider::new()),
        AiProviderKind::OpenCode => Box::new(opencode::OpenCodeProvider::new()),
    }
}

fn coordinator(state: &State<'_, AppState>) -> Result<Arc<AiBackgroundCoordinator>, String> {
    state
        .ai_background_coordinator
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "AI background coordinator not initialised".to_string())
}

// ─── Commands ────────────────────────────────────────────────────────────────

/// Start a new AI background run.
///
/// Creates a worktree, registers an [`AiSession`] in the coordinator, and
/// (unless the concurrency cap is reached) dispatches the headless provider
/// CLI via [`task_runner::TaskManager`]. Returns immediately — progress is
/// surfaced via the `ai-background-status` and `ai-background-output`
/// events.
#[tauri::command]
pub async fn ai_start_background_run(
    request: StartBackgroundRunRequest,
    state: State<'_, AppState>,
) -> Result<StartBackgroundRunResponse, String> {
    let repo_root = get_active_project_path(&state)?;
    let kind = parse_kind(&request.provider)?;

    let (worktree_root_override, concurrency_cap, auto_accept) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        (
            config.ai_worktree_root.clone(),
            config.ai_background_concurrency_cap.max(1),
            config.ai_prompt_auto_accept,
        )
    };

    let args = StartArgs {
        repo_root,
        provider: kind,
        base_branch: request.base_branch,
        prompt: request.prompt,
        skill: request.skill,
        saved_prompt_path: request.saved_prompt_path.map(PathBuf::from),
        resume_session_id: request.resume_session_id,
        worktree_slug_override: request.worktree_slug_override,
        worktree_root_override,
        auto_accept_permissions: auto_accept,
        concurrency_cap,
    };

    let coord = coordinator(&state)?;
    let provider = make_provider(kind);

    // The start call is synchronous but internally calls block_in_place to
    // spawn the subprocess via TaskManager. That's fine — this Tauri command
    // itself runs on a tokio worker.
    let out = tokio::task::block_in_place(|| coord.start(args, provider.as_ref()))
        .map_err(|e| e.to_string())?;

    Ok(StartBackgroundRunResponse {
        session_id: out.session_id,
        task_id: out.task_id,
        worktree_path: out.worktree_path.to_string_lossy().into_owned(),
        status: out.status,
    })
}

/// Request cancellation of a running background session.
#[tauri::command]
pub async fn ai_cancel_background_run(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let coord = coordinator(&state)?;
    coord.cancel(&session_id).map_err(|e| e.to_string())
}

/// Return all known background runs (queued, running, terminal).
#[tauri::command]
pub async fn ai_list_background_runs(state: State<'_, AppState>) -> Result<Vec<AiSession>, String> {
    let coord = coordinator(&state)?;
    Ok(coord.list())
}

/// Return a single background run by session id, or `None`.
#[tauri::command]
pub async fn ai_get_background_run(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Option<AiSession>, String> {
    let coord = coordinator(&state)?;
    Ok(coord.get(&session_id))
}

/// Remove the worktree for a terminal-state run and scrub the session
/// record.
#[tauri::command]
pub async fn ai_discard_background_run_worktree(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let coord = coordinator(&state)?;
    tokio::task::spawn_blocking(move || coord.discard_worktree(&session_id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Attach a new PTY terminal to the worktree of a background run.
///
/// If the provider supports `--resume <session>` (Claude Code), the terminal
/// is launched with that flag. Otherwise a bare interactive session starts.
/// Returns the new terminal's session id so the frontend can focus it.
#[tauri::command]
pub fn ai_open_background_terminal(
    session_id: String,
    state: State<'_, AppState>,
    terminal_manager: State<'_, Arc<TerminalManager>>,
) -> Result<SessionId, String> {
    let coord = coordinator(&state)?;
    let session = coord
        .get(&session_id)
        .ok_or_else(|| format!("session not found: {session_id}"))?;

    let worktree_path = session
        .worktree_path
        .clone()
        .ok_or_else(|| "session has no worktree path".to_string())?;

    let provider = make_provider(session.provider);

    // Prefer --resume when the underlying provider session id is known.
    let command = provider
        .build_resume_session_cmd(&session.id, &worktree_path)
        .or_else(|| provider.build_interactive_cmd(&worktree_path).ok())
        .ok_or_else(|| "provider cannot launch an interactive terminal".to_string())?;

    let program = command.get_program().to_string_lossy().to_string();
    let args: Vec<String> = command
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    let shell_cmd = if args.is_empty() {
        program
    } else {
        format!("{} {}", program, args.join(" "))
    };

    let config = TerminalConfig {
        cwd: worktree_path,
        shell: Some(shell_cmd),
        env: HashMap::new(),
        cols: 220,
        rows: 50,
    };
    terminal_manager.spawn(config).map_err(|e| e.to_string())
}
