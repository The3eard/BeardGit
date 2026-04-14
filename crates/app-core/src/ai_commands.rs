//! Tauri command handlers for AI provider integration.
//!
//! Exposes 14 commands covering detection, headless task execution, interactive
//! terminal launch, and session/worktree/config introspection. All commands
//! follow the `Result<T, String>` IPC convention used throughout `app-core`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tauri::State;

use ai_provider::{
    AiConfigFile, AiProvider, AiProviderKind, AiSession, AiWorktree, AvailableAiProvider,
    RepoAiStatus,
};
use task_runner::{TaskId, TaskManager};
use terminal::{SessionId, TerminalConfig, TerminalManager};

use crate::commands::get_active_project_path;
use crate::state::AppState;

// ─── Provider factory ────────────────────────────────────────────────────────

/// Instantiate the correct [`AiProvider`] implementation for the given kind.
///
/// Returns `Err` for provider kinds that are not yet implemented.
fn make_provider(kind: AiProviderKind) -> Result<Box<dyn AiProvider>, String> {
    match kind {
        AiProviderKind::ClaudeCode => Ok(Box::new(claude_code::ClaudeCodeProvider::new())),
        other => Err(format!(
            "{} provider is not yet supported",
            other.display_name()
        )),
    }
}

/// Parse a provider kind from its snake_case string name.
fn parse_kind(provider: &str) -> Result<AiProviderKind, String> {
    match provider {
        "claude_code" => Ok(AiProviderKind::ClaudeCode),
        "codex" => Ok(AiProviderKind::Codex),
        "open_code" => Ok(AiProviderKind::OpenCode),
        other => Err(format!("unknown AI provider: {other}")),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Run `git diff --cached` and return the output as a string.
///
/// Used to build prompts for commit message and PR description generation
/// without needing to call into `git-engine`.
fn get_staged_diff_text(cwd: &Path) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .current_dir(cwd)
        .args(["diff", "--cached"])
        .output()
        .map_err(|e| format!("failed to run git diff: {e}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Extract the program path and argument list from a `std::process::Command`.
fn command_to_parts(cmd: &std::process::Command) -> (String, Vec<String>) {
    let program = cmd.get_program().to_string_lossy().to_string();
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    (program, args)
}

// ─── Detection ───────────────────────────────────────────────────────────────

/// Return the list of AI providers currently stored in application state.
///
/// This reflects the last call to [`ai_refresh_detection`]. Returns an empty
/// list if detection has not been run yet.
#[tauri::command]
pub fn ai_get_providers(state: State<'_, AppState>) -> Vec<AvailableAiProvider> {
    state
        .ai_providers
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

/// Return per-provider AI status for the current repository.
///
/// For each provider stored in state, checks:
/// - Whether the repo root has provider-specific config files
/// - How many sessions and worktrees are known
#[tauri::command]
pub fn ai_get_repo_status(state: State<'_, AppState>) -> Result<Vec<RepoAiStatus>, String> {
    let cwd = get_active_project_path(&state)?;

    let providers = state
        .ai_providers
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let mut statuses = Vec::with_capacity(providers.len());
    for available in &providers {
        let provider = make_provider(available.kind)?;
        let has_config = provider.detect_in_repo(&cwd);
        let session_count = provider.list_sessions(&cwd).map(|s| s.len()).unwrap_or(0);
        let worktree_count = provider.list_worktrees(&cwd).map(|w| w.len()).unwrap_or(0);
        statuses.push(RepoAiStatus {
            kind: available.kind,
            has_config,
            session_count,
            worktree_count,
        });
    }

    Ok(statuses)
}

/// Scan PATH for all supported AI tool binaries and update application state.
///
/// Replaces the current provider list in state. Cheap local operation — runs
/// `which` and `--version` for each candidate.
#[tauri::command]
pub fn ai_refresh_detection(state: State<'_, AppState>) -> Result<(), String> {
    let kinds = [
        AiProviderKind::ClaudeCode,
        AiProviderKind::Codex,
        AiProviderKind::OpenCode,
    ];

    let mut detected: Vec<AvailableAiProvider> = Vec::new();
    for kind in kinds {
        // Only ClaudeCode has a real implementation — skip unsupported silently.
        let Ok(provider) = make_provider(kind) else {
            continue;
        };
        if let Some(binary_path) = provider.detect_binary() {
            let version = provider.version().ok();
            detected.push(AvailableAiProvider {
                kind,
                binary_path,
                version,
            });
        }
    }

    let mut guard = state.ai_providers.lock().map_err(|e| e.to_string())?;
    *guard = detected;
    Ok(())
}

// ─── Headless Actions ─────────────────────────────────────────────────────────

/// Generate a commit message for the current staged diff.
///
/// Spawns a headless AI task via `TaskManager`. Returns the `TaskId` so the
/// frontend can stream output via `"task-output"` events.
#[tauri::command]
pub async fn ai_generate_commit_message(
    provider: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;
    let diff = get_staged_diff_text(&cwd)?;
    let cmd = p
        .build_commit_message_cmd(&diff, &cwd)
        .map_err(|e| e.to_string())?;
    let (program, args) = command_to_parts(&cmd);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let task_id = task_manager
        .spawn(
            "AI: generate commit message".into(),
            &program,
            &args_refs,
            &cwd,
            false,
        )
        .await;
    Ok(task_id)
}

/// Analyze code and answer a question about it.
///
/// `content` is the code snippet or file contents. `question` is the query to
/// ask the AI. Returns a `TaskId` for output streaming.
#[tauri::command]
pub async fn ai_analyze_code(
    provider: String,
    content: String,
    question: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;
    let cmd = p
        .build_analysis_cmd(&content, &question, &cwd)
        .map_err(|e| e.to_string())?;
    let (program, args) = command_to_parts(&cmd);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let task_id = task_manager
        .spawn("AI: analyze code".into(), &program, &args_refs, &cwd, false)
        .await;
    Ok(task_id)
}

/// Generate a pull request description for the current staged diff.
///
/// Returns a `TaskId` for output streaming.
#[tauri::command]
pub async fn ai_generate_pr_description(
    provider: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;
    let diff = get_staged_diff_text(&cwd)?;
    let cmd = p
        .build_pr_description_cmd(&diff, &cwd)
        .map_err(|e| e.to_string())?;
    let (program, args) = command_to_parts(&cmd);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let task_id = task_manager
        .spawn(
            "AI: generate PR description".into(),
            &program,
            &args_refs,
            &cwd,
            false,
        )
        .await;
    Ok(task_id)
}

/// Review a code diff.
///
/// `diff` is the unified diff text to review. Returns a `TaskId` for output
/// streaming.
#[tauri::command]
pub async fn ai_review_code(
    provider: String,
    diff: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;
    let cmd = p.build_review_cmd(&diff, &cwd).map_err(|e| e.to_string())?;
    let (program, args) = command_to_parts(&cmd);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let task_id = task_manager
        .spawn("AI: review code".into(), &program, &args_refs, &cwd, false)
        .await;
    Ok(task_id)
}

/// Review a pull request diff.
///
/// `diff` is the unified diff text to review. Returns a `TaskId` for output
/// streaming.
#[tauri::command]
pub async fn ai_review_pr(
    provider: String,
    diff: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;
    let cmd = p
        .build_pr_review_cmd(&diff, &cwd)
        .map_err(|e| e.to_string())?;
    let (program, args) = command_to_parts(&cmd);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let task_id = task_manager
        .spawn("AI: review PR".into(), &program, &args_refs, &cwd, false)
        .await;
    Ok(task_id)
}

// ─── Interactive ──────────────────────────────────────────────────────────────

/// Launch an interactive AI session in a new terminal tab.
///
/// Builds the provider's interactive command and spawns it via
/// `TerminalManager`. Returns the `SessionId` so the frontend can attach an
/// xterm.js panel.
#[tauri::command]
pub fn ai_launch_interactive(
    provider: String,
    state: State<'_, AppState>,
    terminal_manager: State<'_, Arc<TerminalManager>>,
) -> Result<SessionId, String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;
    let cmd = p.build_interactive_cmd(&cwd).map_err(|e| e.to_string())?;
    let (program, args) = command_to_parts(&cmd);
    let shell_cmd = if args.is_empty() {
        program
    } else {
        format!("{} {}", program, args.join(" "))
    };
    let config = TerminalConfig {
        cwd: cwd.to_path_buf(),
        shell: Some(shell_cmd),
        env: HashMap::new(),
        cols: 220,
        rows: 50,
    };
    terminal_manager.spawn(config).map_err(|e| e.to_string())
}

/// Launch an AI session with worktree isolation.
///
/// If the provider supports worktrees, creates a new worktree and opens an
/// interactive session inside it. Returns `None` if the provider does not
/// support worktrees (no error — callers can fall back to plain interactive).
#[tauri::command]
pub fn ai_launch_worktree(
    provider: String,
    name: Option<String>,
    state: State<'_, AppState>,
    terminal_manager: State<'_, Arc<TerminalManager>>,
) -> Result<Option<SessionId>, String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;

    let Some(cmd) = p.build_worktree_cmd(&cwd, name.as_deref()) else {
        return Ok(None);
    };

    let (program, args) = command_to_parts(&cmd);
    let shell_cmd = if args.is_empty() {
        program
    } else {
        format!("{} {}", program, args.join(" "))
    };
    let config = TerminalConfig {
        cwd: cwd.to_path_buf(),
        shell: Some(shell_cmd),
        env: HashMap::new(),
        cols: 220,
        rows: 50,
    };
    let session_id = terminal_manager.spawn(config).map_err(|e| e.to_string())?;
    Ok(Some(session_id))
}

// ─── Introspection ────────────────────────────────────────────────────────────

/// List AI sessions for all detected providers in the current repository.
#[tauri::command]
pub fn ai_list_sessions(state: State<'_, AppState>) -> Result<Vec<AiSession>, String> {
    let cwd = get_active_project_path(&state)?;
    let providers = state
        .ai_providers
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let mut sessions: Vec<AiSession> = Vec::new();
    for available in &providers {
        let Ok(provider) = make_provider(available.kind) else {
            continue;
        };
        if let Ok(mut s) = provider.list_sessions(&cwd) {
            sessions.append(&mut s);
        }
    }
    Ok(sessions)
}

/// List AI-created worktrees for all detected providers in the current repository.
#[tauri::command]
pub fn ai_list_worktrees(state: State<'_, AppState>) -> Result<Vec<AiWorktree>, String> {
    let cwd = get_active_project_path(&state)?;
    let providers = state
        .ai_providers
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let mut worktrees: Vec<AiWorktree> = Vec::new();
    for available in &providers {
        let Ok(provider) = make_provider(available.kind) else {
            continue;
        };
        if let Ok(mut w) = provider.list_worktrees(&cwd) {
            worktrees.append(&mut w);
        }
    }
    Ok(worktrees)
}

/// Remove an AI-created worktree and its associated branch.
///
/// `provider` is the snake_case provider name (e.g., `"claude_code"`).
/// `worktree_path` is the absolute filesystem path to the worktree root.
#[tauri::command]
pub fn ai_cleanup_worktree(
    provider: String,
    worktree_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;

    // Find the matching worktree by path.
    let worktrees = p.list_worktrees(&cwd).map_err(|e| e.to_string())?;
    let target_path = std::path::PathBuf::from(&worktree_path);
    let worktree = worktrees
        .iter()
        .find(|w| w.path == target_path)
        .ok_or_else(|| format!("worktree not found: {worktree_path}"))?;

    p.cleanup_worktree(worktree).map_err(|e| e.to_string())
}

/// List AI configuration files for all detected providers in the current repository.
#[tauri::command]
pub fn ai_get_config_files(state: State<'_, AppState>) -> Result<Vec<AiConfigFile>, String> {
    let cwd = get_active_project_path(&state)?;
    let providers = state
        .ai_providers
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let mut files: Vec<AiConfigFile> = Vec::new();
    for available in &providers {
        let Ok(provider) = make_provider(available.kind) else {
            continue;
        };
        files.extend(provider.config_files(&cwd));
    }
    Ok(files)
}
