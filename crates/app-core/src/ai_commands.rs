//! Tauri command handlers for AI provider integration.
//!
//! Exposes 17 commands covering detection, headless task execution, interactive
//! terminal launch, session/worktree/config introspection, and config file
//! management. All commands follow the `Result<T, String>` IPC convention used
//! throughout `app-core`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use ai_provider::{
    AiConfigFile, AiConversation, AiProvider, AiProviderKind, AiWorktree, AvailableAiProvider,
    ConfigKind, ConfigScope, RepoAiStatus,
};
use task_runner::{SpawnOptions, TaskId, TaskKind, TaskManager};
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
        AiProviderKind::Codex => Ok(Box::new(codex::CodexProvider::new())),
        AiProviderKind::OpenCode => Ok(Box::new(opencode::OpenCodeProvider::new())),
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

/// Run `git diff --no-ext-diff --cached` and return the output as a string.
///
/// Used to build prompts for commit message and PR description generation
/// without needing to call into `git-engine`. The `--no-ext-diff` flag
/// bypasses any user-configured `diff.external` (e.g. `difftastic`) so the
/// AI provider always receives canonical unified-diff text.
///
/// Runs through `tokio::process::Command` so the IPC executor is not
/// blocked while `git` walks the index — large staged diffs in big repos
/// can take hundreds of milliseconds.
async fn get_staged_diff_text(cwd: &Path) -> Result<String, String> {
    let output = tokio::process::Command::new("git")
        .current_dir(cwd)
        .args(["diff", "--no-ext-diff", "--cached"])
        .output()
        .await
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
        // `session_count` is the per-provider badge the AI Settings panel
        // renders; after the transcript-first rewrite the source of truth
        // is `list_conversations`, which counts on-disk transcripts
        // regardless of live-process state. The field name stays the same
        // for IPC back-compat.
        let session_count = provider
            .list_conversations(&cwd)
            .map(|c| c.len())
            .unwrap_or(0);
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
/// Replaces the current provider list in state. Runs `which` + `--version`
/// per candidate — cheap on a warm cache but `--version` can stall for ~1 s
/// on cold first launches while Claude's V8 spins up.
///
/// Runs on the blocking pool via `spawn_blocking` so the IPC thread stays
/// free and Settings → AI paints the spinner frame without waiting for the
/// probes. Same pattern as `ai_list_conversations`.
#[tauri::command]
pub async fn ai_refresh_detection(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let detected = tokio::task::spawn_blocking(|| {
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
        detected
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut guard = state.ai_providers.lock().map_err(|e| e.to_string())?;
    *guard = detected;
    drop(guard);

    // Start the AI transcript directory watcher (once) so the frontend
    // receives live updates when rollout / transcript files change on
    // disk. Each provider stores transcripts in its own tree:
    //
    // - Claude Code: `~/.claude/projects/{cwd-slug}/*.jsonl`
    // - Codex: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
    // - OpenCode: SQLite DB — no fs watch, that provider's list refreshes
    //   on the existing `terminal-closed` trigger instead.
    //
    // We watch the parent directory of each tree so a provider writing
    // its very first transcript still fires a change event. The
    // `ai-sessions-changed` event name is kept for back-compat with the
    // TypeScript listeners in `aiConversations.ts`.
    {
        let mut watcher_guard = state.ai_session_watcher.lock().map_err(|e| e.to_string())?;
        if watcher_guard.is_none() {
            let transcript_dirs: Vec<std::path::PathBuf> = [
                dirs::home_dir().map(|h| h.join(".claude").join("projects")),
                dirs::home_dir().map(|h| h.join(".codex").join("sessions")),
            ]
            .into_iter()
            .flatten()
            .collect();

            let handle = app_handle.clone();
            *watcher_guard = watcher::AiSessionWatcher::start(&transcript_dirs, move || {
                let _ = handle.emit("ai-sessions-changed", ());
            });
        }
    }

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
    let diff = get_staged_diff_text(&cwd).await?;
    let cmd = p
        .build_commit_message_cmd(&diff, &cwd)
        .map_err(|e| e.to_string())?;
    let (program, args) = command_to_parts(&cmd);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let task_id = task_manager
        .spawn_with_options(SpawnOptions {
            label: "AI: generate commit message".into(),
            command: &program,
            args: &args_refs,
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::AiHeadless,
            stdin: None,
        })
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
        .spawn_with_options(SpawnOptions {
            label: "AI: analyze code".into(),
            command: &program,
            args: &args_refs,
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::AiHeadless,
            stdin: None,
        })
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
    let diff = get_staged_diff_text(&cwd).await?;
    let cmd = p
        .build_pr_description_cmd(&diff, &cwd)
        .map_err(|e| e.to_string())?;
    let (program, args) = command_to_parts(&cmd);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let task_id = task_manager
        .spawn_with_options(SpawnOptions {
            label: "AI: generate PR description".into(),
            command: &program,
            args: &args_refs,
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::AiHeadless,
            stdin: None,
        })
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
    // Spawn with `TaskKind::AiHeadless` (rather than the default Generic)
    // so the task surfaces in the unified drawer. Generic tasks are
    // intentionally suppressed by `kind_from_runtime` and would never
    // emit `task://update`, leaving the drawer's list empty even while
    // output streamed to `taskOutput`. See `task_events::kind_from_runtime`.
    let task_id = task_manager
        .spawn_with_options(SpawnOptions {
            label: "AI: review code".into(),
            command: &program,
            args: &args_refs,
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::AiHeadless,
            stdin: None,
        })
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
        .spawn_with_options(SpawnOptions {
            label: "AI: review PR".into(),
            command: &program,
            args: &args_refs,
            cwd: &cwd,
            cancellable: true,
            kind: TaskKind::AiHeadless,
            stdin: None,
        })
        .await;
    Ok(task_id)
}

// ─── Persisted reviews ────────────────────────────────────────────────────────

/// Result of [`save_ai_review`]. The frontend uses `relative_path` for the
/// toast message and `path` for the "Open" action.
#[derive(Debug, serde::Serialize)]
pub struct SaveAiReviewResult {
    /// Absolute path of the saved review file. Suitable for `openUrl`.
    pub path: String,
    /// Path relative to the active project root (always under
    /// `.beardgit/reviews/`). Convenient for the toast copy so the user
    /// can grep their working tree without seeing the full home prefix.
    pub relative_path: String,
}

/// Persist an AI-generated code review to
/// `<active project>/.beardgit/reviews/review-<utc-stamp>-<short-head>.md`.
///
/// `.beardgit/` is gitignored by convention (and recommended in BeardGit's
/// own `.gitignore` template), so reviews stay local to the developer's
/// machine — same posture as the AI background-session worktrees that live
/// next door at `.beardgit/ai-worktrees/`.
///
/// The header (date, repository path, short HEAD) is composed here so the
/// frontend hands us only the AI's raw output. Best-effort: when HEAD
/// cannot be resolved (unborn branch, non-git folder) the filename falls
/// back to `no-head` rather than failing.
#[tauri::command]
pub fn save_ai_review(
    content: String,
    state: State<'_, AppState>,
) -> Result<SaveAiReviewResult, String> {
    let cwd = get_active_project_path(&state)?;
    let dir = cwd.join(".beardgit").join("reviews");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create reviews dir: {e}"))?;

    let head_short = git2::Repository::open(&cwd)
        .ok()
        .and_then(|r| r.head().ok().and_then(|h| h.target()))
        .map(|oid| {
            let s = oid.to_string();
            s.chars().take(7).collect::<String>()
        })
        .unwrap_or_else(|| "no-head".to_string());

    let now = chrono::Utc::now();
    let filename = format!("review-{}-{}.md", now.format("%Y-%m-%d-%H%M%S"), head_short);
    let abs_path = dir.join(&filename);

    let body = format!(
        "# Code review — {timestamp}\n\n\
         - **Repository:** `{cwd}`\n\
         - **HEAD:** `{head}`\n\n\
         ---\n\n\
         {content}\n",
        timestamp = now.format("%Y-%m-%d %H:%M:%S UTC"),
        cwd = cwd.display(),
        head = head_short,
        content = content.trim_end(),
    );

    std::fs::write(&abs_path, body).map_err(|e| format!("write review: {e}"))?;

    let relative_path = format!(".beardgit/reviews/{filename}");
    Ok(SaveAiReviewResult {
        path: abs_path.to_string_lossy().into_owned(),
        relative_path,
    })
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
    verify_executable(&program)?;
    let config = TerminalConfig {
        cwd: cwd.to_path_buf(),
        shell: None,
        args: Vec::new(),
        env: HashMap::new(),
        cols: 220,
        rows: 50,
    };
    // Trusted, app-built command (binary resolved server-side) — use the
    // trusted spawn path so the webview shell/arg allowlist doesn't reject it.
    terminal_manager
        .spawn_program(&program, &args, config)
        .map_err(|e| e.to_string())
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
    verify_executable(&program)?;
    let config = TerminalConfig {
        cwd: cwd.to_path_buf(),
        shell: None,
        args: Vec::new(),
        env: HashMap::new(),
        cols: 220,
        rows: 50,
    };
    let session_id = terminal_manager
        .spawn_program(&program, &args, config)
        .map_err(|e| e.to_string())?;
    Ok(Some(session_id))
}

/// Spawn a new terminal resuming a conversation by id.
///
/// Note: with Claude Code, every `--resume` creates a NEW conversation UUID
/// that forks from the named one. The frontend labels the button
/// "Resume in new terminal" so this forking semantics is obvious.
#[tauri::command]
pub fn ai_resume_conversation(
    provider: String,
    conversation_id: String,
    state: State<'_, AppState>,
    terminal_manager: State<'_, Arc<TerminalManager>>,
) -> Result<Option<SessionId>, String> {
    let cwd = get_active_project_path(&state)?;
    let kind = parse_kind(&provider)?;
    let p = make_provider(kind)?;

    let Some(cmd) = p.build_resume_session_cmd(&conversation_id, &cwd) else {
        return Ok(None);
    };

    let (program, args) = command_to_parts(&cmd);
    verify_executable(&program)?;
    let config = TerminalConfig {
        cwd: cwd.to_path_buf(),
        shell: None,
        args: Vec::new(),
        env: HashMap::new(),
        cols: 220,
        rows: 50,
    };
    let session = terminal_manager
        .spawn_program(&program, &args, config)
        .map_err(|e| e.to_string())?;
    Ok(Some(session))
}

/// Verify a detected CLI binary is still present and executable.
///
/// `which::which` can return stale PATH entries (broken symlinks from a
/// previous installer, files removed since last cache refresh). Turning
/// those into a clear error here is strictly better than letting
/// `execvp` fail deep inside `portable-pty` with a bare `ENOENT`.
fn verify_executable(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    let Ok(meta) = std::fs::metadata(p) else {
        return Err(format!(
            "{path} resolved from PATH but no longer exists. The binary \
             may have been moved or uninstalled. Re-run the installer, or \
             remove the stale entry from your shell PATH and relaunch BeardGit."
        ));
    };
    if !meta.is_file() {
        return Err(format!("{path} is not a regular file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return Err(format!("{path} is not executable"));
        }
    }
    Ok(())
}

// ─── Introspection ────────────────────────────────────────────────────────────

/// List AI conversation transcripts for all detected providers in the current repo.
///
/// Runs the per-provider `list_conversations` calls off the IPC thread via
/// `spawn_blocking` — OpenCode still shells out synchronously under the
/// hood, so a PATH-walking provider can't stall the Tauri runtime long
/// enough to back up other IPC calls behind it.
#[tauri::command]
pub async fn ai_list_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<AiConversation>, String> {
    let cwd = get_active_project_path(&state)?;
    let providers = state
        .ai_providers
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    tokio::task::spawn_blocking(move || {
        let mut conversations: Vec<AiConversation> = Vec::new();
        for available in &providers {
            let Ok(provider) = make_provider(available.kind) else {
                continue;
            };
            if let Ok(mut c) = provider.list_conversations(&cwd) {
                conversations.append(&mut c);
            }
        }
        Ok(conversations)
    })
    .await
    .map_err(|e| e.to_string())?
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

// ─── Provider Preference ─────────────────────────────────────────────────────

/// Return the preferred AI provider kind from settings, or `None` for auto-detect.
#[tauri::command]
pub fn ai_get_preferred_provider(state: State<'_, AppState>) -> Option<String> {
    let config = state.config.lock().unwrap();
    config.preferred_ai_provider.clone()
}

/// Set the preferred AI provider kind. Pass `None` to reset to auto-detect.
#[tauri::command]
pub fn ai_set_preferred_provider(
    provider: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.preferred_ai_provider = provider;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Start watching AI config directories for the active project.
///
/// Watches `<project>/.claude/` and `~/.claude/` for file changes.
/// Emits `"ai-config-changed"` Tauri events with `{ path, scope }`.
/// Only one watcher is active at a time — calling again replaces the previous.
#[tauri::command]
pub fn ai_watch_config_dirs(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cwd = get_active_project_path(&state)?;
    let handle = app_handle.clone();

    let watcher = watcher::AiConfigWatcher::start(&cwd, move |change| {
        let _ = handle.emit("ai-config-changed", change);
    });

    let mut guard = state.ai_config_watcher.lock().map_err(|e| e.to_string())?;
    *guard = watcher;
    Ok(())
}

/// Stop the AI config directory watcher.
#[tauri::command]
pub fn ai_stop_config_watcher(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.ai_config_watcher.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
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

// ─── Config File Management ──────────────────────────────────────────────────

/// Validate that a config file path is within allowed boundaries.
///
/// Allowed: project repo root (and children) or `~/.claude/` (and children).
fn validate_config_path(path: &str, repo_root: &Path) -> Result<PathBuf, String> {
    // Resolve `.`/`..` lexically FIRST (so a `..` can't later defeat the
    // `starts_with` scope check, which is component-based and would otherwise
    // accept `<repo>/../../etc/passwd`), then canonicalize the longest existing
    // prefix to resolve symlinks. Crucially this NEVER creates directories —
    // the previous implementation ran `create_dir_all` before the scope check,
    // which polluted the filesystem for rejected paths and turned the read
    // command into a silent mkdir.
    let lexical = normalize_lexical(&PathBuf::from(path));
    let canonical = canonicalize_existing_prefix(&lexical)?;

    let repo_canon = repo_root
        .canonicalize()
        .map_err(|e| format!("cannot resolve repo: {e}"))?;
    if canonical.starts_with(&repo_canon) {
        return Ok(canonical);
    }

    if let Some(home) = dirs::home_dir() {
        let claude_dir = home.join(".claude");
        if canonical.starts_with(&claude_dir) {
            return Ok(canonical);
        }
    }

    Err(format!("path outside allowed scope: {path}"))
}

/// Lexically resolve `.` and `..` components without touching the filesystem.
/// Leading `..` on an absolute path are dropped (can't ascend past root).
pub(crate) fn normalize_lexical(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Canonicalize the deepest existing ancestor of `p` (resolving symlinks),
/// then re-append the remaining components. `p` must already be lexically
/// normalized so the non-existing tail contains no `..`/`.` that could escape
/// the canonicalized prefix. Never creates anything on disk.
fn canonicalize_existing_prefix(p: &Path) -> Result<PathBuf, String> {
    let mut existing = p;
    while !existing.exists() {
        match existing.parent() {
            Some(parent) => existing = parent,
            None => return Err(format!("invalid path: {}", p.display())),
        }
    }
    let canon_base = existing
        .canonicalize()
        .map_err(|e| format!("invalid path: {e}"))?;
    let tail = p
        .strip_prefix(existing)
        .map_err(|e| format!("invalid path: {e}"))?;
    Ok(canon_base.join(tail))
}

/// Resolve the filesystem path for a new config file from its kind, scope, and name.
fn resolve_new_config_path(
    kind: &str,
    scope: &str,
    name: &str,
    repo_root: &Path,
) -> Result<PathBuf, String> {
    let base = match scope {
        "project" => repo_root.to_path_buf(),
        "user" => dirs::home_dir().ok_or("cannot determine home directory".to_string())?,
        _ => return Err(format!("unknown scope: {scope}")),
    };
    let claude_dir = base.join(".claude");
    match kind {
        "agent" => Ok(claude_dir.join("agents").join(format!("{name}.md"))),
        "skill" => Ok(claude_dir.join("skills").join(name).join("SKILL.md")),
        "prompt" => Ok(claude_dir.join("prompts").join(format!("{name}.md"))),
        _ => Err(format!("unknown config kind: {kind}")),
    }
}

/// Read the contents of an AI configuration file.
///
/// The path must be within the active project root or `~/.claude/`.
#[tauri::command]
pub async fn ai_read_config_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;
    let validated = validate_config_path(&path, &repo_path)?;
    std::fs::read_to_string(&validated)
        .map_err(|e| format!("failed to read {}: {e}", validated.display()))
}

/// Write content to an AI configuration file.
///
/// The path must be within the active project root or `~/.claude/`.
/// Parent directories are created automatically.
#[tauri::command]
pub async fn ai_write_config_file(
    path: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;
    let validated = validate_config_path(&path, &repo_path)?;
    if let Some(parent) = validated.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directories: {e}"))?;
    }
    std::fs::write(&validated, content)
        .map_err(|e| format!("failed to write {}: {e}", validated.display()))
}

/// Create a new AI configuration file from a template.
///
/// `kind` is one of `"agent"`, `"skill"`, or `"prompt"`. `scope` is `"project"`
/// or `"user"`. `name` is the file/directory base name. Returns the created
/// [`AiConfigFile`] with the resolved path.
#[tauri::command]
pub async fn ai_create_config_file(
    kind: String,
    scope: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<AiConfigFile, String> {
    let repo_path = get_active_project_path(&state)?;
    let file_path = resolve_new_config_path(&kind, &scope, &name, &repo_path)?;

    if file_path.exists() {
        return Err(format!("file already exists: {}", file_path.display()));
    }

    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directories: {e}"))?;
    }

    let template = match kind.as_str() {
        "agent" => format!("---\nname: {name}\ndescription: \n---\n\n"),
        "skill" => format!("---\nname: {name}\ndescription: \n---\n\n"),
        "prompt" => String::from("# \n\n"),
        _ => String::new(),
    };

    std::fs::write(&file_path, &template).map_err(|e| format!("failed to write template: {e}"))?;

    let config_kind = match kind.as_str() {
        "agent" => ConfigKind::Agent,
        "skill" => ConfigKind::Skill,
        _ => ConfigKind::Instructions,
    };

    let config_scope = match scope.as_str() {
        "user" => ConfigScope::User,
        "local" => ConfigScope::Local,
        _ => ConfigScope::Project,
    };

    Ok(AiConfigFile {
        path: file_path,
        kind: config_kind,
        scope: config_scope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_path_accepts_project_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        let claude_dir = repo.join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let path = claude_dir.join("settings.json");
        std::fs::write(&path, "{}").unwrap();
        assert!(validate_config_path(path.to_str().unwrap(), repo).is_ok());
    }

    #[test]
    fn validate_path_rejects_outside_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        assert!(validate_config_path("/tmp/evil.json", repo).is_err());
    }

    #[test]
    fn validate_path_rejects_dotdot_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        // A `..` chain under the repo must not pass the scope check after
        // lexical normalization.
        let escape = format!("{}/.claude/../../../../../../etc/passwd", repo.display());
        assert!(validate_config_path(&escape, repo).is_err());
    }

    #[test]
    fn validate_path_does_not_create_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        // An in-scope but non-existent nested path must validate WITHOUT
        // materializing its parent directories (the read command relied on
        // validation being side-effect free).
        let target = repo.join(".claude/new/deep/file.json");
        assert!(validate_config_path(target.to_str().unwrap(), repo).is_ok());
        assert!(
            !repo.join(".claude/new").exists(),
            "validation must not create directories"
        );
    }

    #[test]
    fn resolve_config_creates_agent_path() {
        let tmp = tempfile::tempdir().unwrap();
        let result = resolve_new_config_path("agent", "project", "code-reviewer", tmp.path());
        let expected = tmp.path().join(".claude/agents/code-reviewer.md");
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn resolve_config_creates_skill_path() {
        let tmp = tempfile::tempdir().unwrap();
        let result = resolve_new_config_path("skill", "project", "deploy", tmp.path());
        let expected = tmp.path().join(".claude/skills/deploy/SKILL.md");
        assert_eq!(result.unwrap(), expected);
    }
}
