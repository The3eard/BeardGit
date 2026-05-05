//! AI background worktree coordinator.
//!
//! Glue layer between the `ai-provider` trait (builds [`std::process::Command`]),
//! `git-engine` (creates the worktree), `task-runner` (spawns, streams, cancels
//! the process), and the [`AppState`](crate::state::AppState) session registry
//! that the frontend reads.
//!
//! ## Lifecycle
//!
//! 1. [`AiBackgroundCoordinator::start`] resolves the worktree root, slugs the
//!    prompt, creates a worktree (`git worktree add -b ai/<provider>/<slug>
//!    <worktree_root>/<slug> <base_branch>`), inlines any skill / saved
//!    prompt content into the prompt, then asks the provider for a
//!    [`Command`].
//! 2. A new [`AiSession`] record is inserted into the coordinator's registry
//!    with `background_status = Queued`. If the concurrency cap is reached,
//!    it stays in `Queued` and sits on a FIFO queue; otherwise the session
//!    is dispatched immediately.
//! 3. Dispatch calls [`TaskManager::spawn_with_options`] with
//!    [`TaskKind::AiBackground`] and (if the provider reads prompts from
//!    stdin) the inlined prompt as the stdin payload. The returned
//!    [`TaskId`] is stored on the session and the status flips to `Running`.
//! 4. The [`AiBackgroundSink`] forwards output lines (parsed for token-usage
//!    metadata when possible), then sets `Completed` / `Failed` / `Cancelled`
//!    on lifecycle transitions.
//! 5. [`AiBackgroundCoordinator::cancel`] cancels the task via
//!    [`TaskManager::cancel`]; the sink handles the resulting terminal state.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use ai_provider::{
    AiBackgroundRunInput, AiBackgroundRunStatus, AiError, AiProvider, AiProviderKind, AiSession,
    AiTokenUsage, SessionKind,
};
use mutation_events::{AiSource, MutationFlags, Snapshot};
use task_runner::{SpawnOptions, TaskId, TaskKind, TaskManager};
use tracing::{info, warn};

/// Default sub-path under the repo root used for AI-created worktrees.
pub const DEFAULT_AI_WORKTREE_ROOT: &str = ".beardgit/ai-worktrees";

/// Sub-path under the repo root where AI background runs are asked to
/// drop a final markdown report. Lives alongside the worktree root so
/// `.gitignore`'s `.beardgit/` rule covers it without extra config, and
/// stays in the *parent* repo so the report survives a `Discard
/// worktree` action.
pub const DEFAULT_AI_REPORTS_ROOT: &str = ".beardgit/ai-reports";

/// Arguments accepted by [`AiBackgroundCoordinator::start`].
///
/// All fields use `snake_case` to match what the Tauri command layer passes
/// after JSON deserialisation.
#[derive(Debug, Clone)]
pub struct StartArgs {
    /// Absolute path to the repository root.
    pub repo_root: PathBuf,
    /// Which AI provider should run the prompt.
    pub provider: AiProviderKind,
    /// Base branch name to branch the new worktree from (e.g. "main").
    pub base_branch: String,
    /// Free-text prompt (may be empty if `saved_prompt_path` or `skill` set).
    pub prompt: String,
    /// Optional skill name (Claude Code `--skill`). For providers that don't
    /// support skills natively, the coordinator inlines the skill name into
    /// the prompt text as a fallback.
    pub skill: Option<String>,
    /// Optional path to a saved prompt file whose contents should be
    /// prefixed to the free-text prompt.
    pub saved_prompt_path: Option<PathBuf>,
    /// Optional existing session ID to resume (Claude Code `--resume`).
    pub resume_session_id: Option<String>,
    /// Optional user override for the worktree slug (the directory basename).
    /// When `None`, the slug is derived from the prompt.
    pub worktree_slug_override: Option<String>,
    /// Root directory (absolute OR repo-relative) where AI worktrees live.
    /// `None` means use [`DEFAULT_AI_WORKTREE_ROOT`] under `repo_root`.
    pub worktree_root_override: Option<String>,
    /// If `true`, pass the provider's permission-skip flag. Default `false`.
    pub auto_accept_permissions: bool,
    /// Maximum concurrent runs allowed at the time of this call. The
    /// coordinator clamps this to at least 1.
    pub concurrency_cap: u32,
    /// Shared handle to the active project's [`watcher::RepoWatcher`] cached
    /// [`Snapshot`].
    ///
    /// When provided, [`AiBackgroundCoordinator::start`] holds this lock
    /// across the `git worktree add` call and overwrites the snapshot with
    /// a fresh capture before releasing — so the next debounced batch in
    /// the watcher thread diffs against the post-creation state and emits
    /// nothing for our own change. `None` falls back to letting the
    /// watcher emit naturally (used by tests that never start a watcher).
    pub watcher_cached_snapshot: Option<Arc<Mutex<Snapshot>>>,
}

/// The output returned by a successful [`AiBackgroundCoordinator::start`]
/// call.
#[derive(Debug, Clone)]
pub struct StartOutput {
    pub session_id: String,
    pub task_id: Option<TaskId>,
    pub worktree_path: PathBuf,
    pub status: AiBackgroundRunStatus,
}

/// Shared, thread-safe registry + dispatch queue for background AI runs.
///
/// Held inside [`AppState`](crate::state::AppState) as an
/// `Arc<AiBackgroundCoordinator>`.
pub struct AiBackgroundCoordinator {
    inner: Mutex<CoordinatorInner>,
    task_manager: Arc<TaskManager>,
    /// Called on every status transition so the Tauri layer can re-emit
    /// the update to the frontend.
    event_sink: Arc<dyn AiBackgroundEventSink>,
    /// Monotonically increasing counter to guarantee unique session IDs.
    next_session_counter: AtomicU64,
    /// Factory that resolves an [`AiProviderKind`] to a concrete provider.
    /// Defaults to the real Claude/Codex/OpenCode implementations; tests
    /// swap it out with a stub.
    provider_factory: Arc<dyn Fn(AiProviderKind) -> Box<dyn AiProvider> + Send + Sync>,
}

/// Sink interface used by the coordinator to emit lifecycle events to the
/// Tauri frontend. Kept as a trait so unit tests can collect events in a
/// `Vec` without wiring up a real Tauri handle.
pub trait AiBackgroundEventSink: Send + Sync {
    /// Emitted on every status transition + once per session on creation.
    fn on_status(&self, session: &AiSession);
    /// Emitted on each captured stdout/stderr line.
    fn on_output(&self, session_id: &str, line: &str);
    /// Emitted once when an AI background run reaches a terminal state
    /// and the worktree state diff is non-empty.
    ///
    /// Implementations should forward this to the Tauri `project-mutated`
    /// bus via [`mutation_events::emit_mutation`] with
    /// [`mutation_events::MutationKind::Ai`] and the given `source` so
    /// the frontend refreshes the right stores. Default impl is a no-op
    /// so non-Tauri sinks (tests, `NoopAiBackgroundEventSink`) ignore it.
    fn on_repo_mutated(
        &self,
        _worktree_path: &std::path::Path,
        _source: AiSource,
        _flags: MutationFlags,
    ) {
    }
}

/// A no-op sink — useful for unit tests that only care about state, not events.
pub struct NoopAiBackgroundEventSink;
impl AiBackgroundEventSink for NoopAiBackgroundEventSink {
    fn on_status(&self, _session: &AiSession) {}
    fn on_output(&self, _session_id: &str, _line: &str) {}
}

struct CoordinatorInner {
    /// All sessions — terminal states are retained so the UI can show history.
    sessions: Vec<AiSession>,
    /// FIFO queue of session IDs waiting for a free concurrency slot.
    queue: VecDeque<String>,
    /// Snapshot of the last-configured concurrency cap.
    concurrency_cap: u32,
    /// Maps the session_id to its input payload so queued runs can be
    /// dispatched later (the prompt / stdin / flags were already computed).
    pending: std::collections::HashMap<String, PendingDispatch>,
    /// Worktree [`Snapshot`] captured just before the subprocess was
    /// spawned, keyed by session id. Consumed on terminal transition in
    /// [`AiBackgroundCoordinator::on_task_finished`] so the diff can be
    /// emitted as a `MutationKind::Ai { source }` event.
    before_snapshots: std::collections::HashMap<String, Snapshot>,
    /// Direct `TaskId → session_id` lookup populated by [`Self::dispatch`]
    /// the instant `spawn_with_options` returns. The `on_task_output`
    /// path consults this BEFORE walking `sessions` so output is routed
    /// even when the reader task fires its first line before the
    /// session's `task_id` field has been written by `update_session`.
    /// Without this map, fast-emitting providers (Claude Code's
    /// `--output-format stream-json` starts dumping hook events the
    /// instant stdin closes) lost their entire transcript on the
    /// initial line burst.
    task_id_to_session: std::collections::HashMap<TaskId, String>,
    /// Output lines that arrived at `on_task_output` *before* their
    /// `task_id → session_id` mapping was registered. Drained as soon
    /// as the dispatch completes its insert into `task_id_to_session`,
    /// so the user sees the full transcript even when there's a
    /// fraction-of-a-millisecond race between the spawn returning and
    /// the reader task's first read.
    pending_lines: std::collections::HashMap<TaskId, Vec<String>>,
}

/// Serialisable payload kept around while a run is queued.
struct PendingDispatch {
    provider_kind: AiProviderKind,
    input: AiBackgroundRunInput,
}

impl AiBackgroundCoordinator {
    /// Create a new coordinator sharing the same [`TaskManager`] as the rest
    /// of the app. Uses the real `claude-code` / `codex` / `opencode`
    /// providers.
    pub fn new(task_manager: Arc<TaskManager>, event_sink: Arc<dyn AiBackgroundEventSink>) -> Self {
        Self::with_provider_factory(
            task_manager,
            event_sink,
            Arc::new(|kind| match kind {
                AiProviderKind::ClaudeCode => {
                    Box::new(claude_code::ClaudeCodeProvider::new()) as Box<dyn AiProvider>
                }
                AiProviderKind::Codex => Box::new(codex::CodexProvider::new()),
                AiProviderKind::OpenCode => Box::new(opencode::OpenCodeProvider::new()),
            }),
        )
    }

    /// Create a new coordinator with a custom provider factory — intended
    /// for unit tests that need to swap in a stub provider. Production code
    /// should use [`Self::new`].
    pub fn with_provider_factory(
        task_manager: Arc<TaskManager>,
        event_sink: Arc<dyn AiBackgroundEventSink>,
        provider_factory: Arc<dyn Fn(AiProviderKind) -> Box<dyn AiProvider> + Send + Sync>,
    ) -> Self {
        Self {
            inner: Mutex::new(CoordinatorInner {
                sessions: Vec::new(),
                queue: VecDeque::new(),
                concurrency_cap: 3,
                pending: std::collections::HashMap::new(),
                before_snapshots: std::collections::HashMap::new(),
                task_id_to_session: std::collections::HashMap::new(),
                pending_lines: std::collections::HashMap::new(),
            }),
            task_manager,
            event_sink,
            next_session_counter: AtomicU64::new(1),
            provider_factory,
        }
    }

    /// Snapshot of all known sessions (queued, running, terminal).
    pub fn list(&self) -> Vec<AiSession> {
        let guard = self.inner.lock().expect("coordinator mutex poisoned");
        guard.sessions.clone()
    }

    /// Return a single session by id.
    pub fn get(&self, session_id: &str) -> Option<AiSession> {
        let guard = self.inner.lock().expect("coordinator mutex poisoned");
        guard.sessions.iter().find(|s| s.id == session_id).cloned()
    }

    /// Kick off a new background run.
    ///
    /// On success the session is registered and either dispatched immediately
    /// (status becomes `Running`) or queued behind the concurrency cap
    /// (status stays `Queued`).
    ///
    /// Errors from worktree creation, command building, or task spawning are
    /// propagated back as [`AiError`].
    pub fn start(
        self: &Arc<Self>,
        args: StartArgs,
        provider: &dyn AiProvider,
    ) -> Result<StartOutput, AiError> {
        let effective_cap = args.concurrency_cap.max(1);

        // 1. Resolve worktree root + slug.
        let worktree_root =
            resolve_worktree_root(&args.repo_root, args.worktree_root_override.as_deref());
        let slug = args
            .worktree_slug_override
            .clone()
            .unwrap_or_else(|| slug_from_prompt(&args.prompt, &args.skill));
        let slug = unique_slug(&slug, &worktree_root);
        let worktree_path = worktree_root.join(&slug);

        // 1b. Mint the session id up-front so the report-writing
        //     instruction in step 2 can interpolate it (the AI is asked
        //     to write `<repo>/.beardgit/ai-reports/<session_id>.md`),
        //     and ensure the parent directory exists so the AI's Write
        //     tool doesn't fail on a missing dir.
        let session_id = format!(
            "aibg-{}-{}-{}",
            provider_slug(args.provider),
            now_millis().unwrap_or_default(),
            self.next_session_counter.fetch_add(1, Ordering::SeqCst),
        );
        let report_path = report_path_for(&args.repo_root, &session_id);
        if let Some(reports_dir) = report_path.parent() {
            std::fs::create_dir_all(reports_dir).map_err(AiError::Io)?;
        }

        // 2. Build the combined prompt text — saved prompt + free text, plus
        //    a skill marker for providers that don't support --skill natively,
        //    plus a fixed suffix asking the AI to leave a markdown report at
        //    `report_path` so the user has a readable summary even when the
        //    raw stream-json is noisy or empty.
        let mut combined_prompt = String::new();
        if let Some(ref saved) = args.saved_prompt_path {
            match std::fs::read_to_string(saved) {
                Ok(content) => {
                    combined_prompt.push_str(&content);
                    if !combined_prompt.ends_with('\n') {
                        combined_prompt.push('\n');
                    }
                    combined_prompt.push('\n');
                }
                Err(e) => {
                    return Err(AiError::CommandBuild(format!(
                        "failed to read saved prompt {}: {e}",
                        saved.display()
                    )));
                }
            }
        }
        // For Codex / OpenCode we can't pass `--skill`, so embed a marker in
        // the prompt so the agent sees it.
        let skill_is_native = matches!(args.provider, AiProviderKind::ClaudeCode);
        if let Some(ref skill) = args.skill
            && !skill_is_native
        {
            combined_prompt.push_str(&format!("[use skill: {skill}]\n\n"));
        }
        combined_prompt.push_str(&args.prompt);
        combined_prompt.push_str(&build_report_instruction(&report_path));

        // 3. Create the worktree. Parent directories must exist.
        if let Some(parent) = worktree_path.parent() {
            std::fs::create_dir_all(parent).map_err(AiError::Io)?;
        }
        let repo = git_engine::Repository::open(&args.repo_root).map_err(|e| {
            AiError::CommandBuild(format!(
                "failed to open repo {}: {e}",
                args.repo_root.display()
            ))
        })?;
        let new_branch = format!("ai/{}/{}", provider_slug(args.provider), slug);
        let worktree_path_str = worktree_path
            .to_str()
            .ok_or_else(|| AiError::CommandBuild("worktree path is not valid UTF-8".into()))?;
        // Hold the watcher's cached-snapshot lock across the
        // `git worktree add` so the watcher thread cannot run a diff/emit
        // cycle for the new branch ref + worktree count it would otherwise
        // observe. After the worktree exists, overwrite the cached
        // snapshot with a fresh capture so the next debounced batch sees
        // an empty diff and the spurious `project-mutated` (which would
        // trigger a heavy `refresh_graph_layout` rebuild on the frontend)
        // is suppressed. `watcher_cached_snapshot = None` falls through
        // to the unsynchronised path used by tests.
        if let Some(ref cache_arc) = args.watcher_cached_snapshot {
            let mut guard = cache_arc.lock().map_err(|e| {
                AiError::CommandBuild(format!("watcher snapshot mutex poisoned: {e}"))
            })?;
            repo.create_worktree_at(worktree_path_str, &new_branch, &args.base_branch)
                .map_err(|e| AiError::CommandBuild(format!("failed to create worktree: {e}")))?;
            match Snapshot::capture(&args.repo_root) {
                Ok(after) => *guard = after,
                Err(err) => warn!(
                    error = %err,
                    "post-worktree-creation snapshot capture failed; \
                     watcher may emit a redundant project-mutated event"
                ),
            }
        } else {
            repo.create_worktree_at(worktree_path_str, &new_branch, &args.base_branch)
                .map_err(|e| AiError::CommandBuild(format!("failed to create worktree: {e}")))?;
        }

        // 4. Build the provider input and the AiSession record. (The
        //    session id was minted up-front in step 1b so the report
        //    suffix could reference it.)
        let input = AiBackgroundRunInput {
            provider: args.provider,
            worktree_path: worktree_path.clone(),
            prompt: combined_prompt,
            skill: if skill_is_native { args.skill } else { None },
            saved_prompt_path: None, // already inlined
            resume_session_id: args.resume_session_id,
            auto_accept_permissions: args.auto_accept_permissions,
        };

        let started_at = now_millis();
        // Echo the user-typed prompt back through the session record so
        // the detail pane can show "what I asked" alongside the captured
        // output. Empty prompts → `None` (skill-only / saved-prompt-only
        // runs don't have a free-text command worth surfacing).
        let display_prompt = if args.prompt.trim().is_empty() {
            None
        } else {
            Some(args.prompt.clone())
        };
        let mut session = AiSession {
            id: session_id.clone(),
            provider: args.provider,
            cwd: worktree_path.clone(),
            started_at,
            kind: SessionKind::Headless,
            is_active: true,
            worktree_path: Some(worktree_path.clone()),
            background_status: Some(AiBackgroundRunStatus::Queued),
            task_id: None,
            prompt: display_prompt,
        };

        // 5. Store it + decide whether to dispatch now or queue.
        let should_dispatch = {
            let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
            guard.concurrency_cap = effective_cap;
            let running_count = guard
                .sessions
                .iter()
                .filter(|s| {
                    matches!(
                        s.background_status.as_ref(),
                        Some(AiBackgroundRunStatus::Running)
                    )
                })
                .count() as u32;
            let dispatch_now = running_count < effective_cap;
            if !dispatch_now {
                info!(
                    %session_id,
                    running_count,
                    cap = effective_cap,
                    "AI background run queued behind cap"
                );
                guard.queue.push_back(session_id.clone());
            }
            guard.pending.insert(
                session_id.clone(),
                PendingDispatch {
                    provider_kind: args.provider,
                    input: input.clone(),
                },
            );
            guard.sessions.push(session.clone());
            dispatch_now
        };

        self.event_sink.on_status(&session);

        if should_dispatch {
            let task_id = self.dispatch(&session_id, provider, &input)?;
            session.task_id = Some(task_id);
            session.background_status = Some(AiBackgroundRunStatus::Running);
            self.update_session(&session_id, |s| {
                s.task_id = Some(task_id);
                s.background_status = Some(AiBackgroundRunStatus::Running);
            });
            self.event_sink.on_status(&session);
        }

        Ok(StartOutput {
            session_id,
            task_id: session.task_id,
            worktree_path,
            status: session
                .background_status
                .clone()
                .unwrap_or(AiBackgroundRunStatus::Queued),
        })
    }

    /// Request cancellation of a running session.
    ///
    /// Does nothing if the session is already terminal. For still-queued
    /// sessions the status is flipped to `Cancelled` without touching the
    /// task manager (no process spawned yet).
    pub fn cancel(self: &Arc<Self>, session_id: &str) -> Result<(), AiError> {
        let (task_id, was_queued) = {
            let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
            let session = guard.sessions.iter_mut().find(|s| s.id == session_id);
            let Some(session) = session else {
                return Err(AiError::CommandBuild(format!(
                    "session not found: {session_id}"
                )));
            };
            match session.background_status.as_ref() {
                Some(AiBackgroundRunStatus::Queued) => {
                    session.background_status = Some(AiBackgroundRunStatus::Cancelled);
                    session.is_active = false;
                    guard.queue.retain(|id| id != session_id);
                    guard.pending.remove(session_id);
                    (None, true)
                }
                Some(AiBackgroundRunStatus::Running) => (session.task_id, false),
                _ => return Ok(()),
            }
        };

        if let Some(task_id) = task_id {
            let manager = Arc::clone(&self.task_manager);
            let sid = session_id.to_string();
            // Cancel via the task manager; the sink will flip the status to
            // Cancelled when the subprocess actually exits.
            tokio::spawn(async move {
                if let Err(e) = manager.cancel(task_id).await {
                    warn!(session_id = %sid, error = %e, "failed to cancel AI background task");
                }
            });
        }

        if was_queued {
            let snapshot = self.get(session_id);
            if let Some(s) = snapshot {
                self.event_sink.on_status(&s);
            }
        }

        Ok(())
    }

    /// Remove the worktree created for a session and scrub the session
    /// record. The session must already be in a terminal state.
    pub fn discard_worktree(self: &Arc<Self>, session_id: &str) -> Result<(), AiError> {
        let (worktree_path, repo_root, provider, slug) = {
            let guard = self.inner.lock().expect("coordinator mutex poisoned");
            let session = guard
                .sessions
                .iter()
                .find(|s| s.id == session_id)
                .ok_or_else(|| AiError::CommandBuild(format!("session not found: {session_id}")))?;
            let status = session.background_status.as_ref();
            if !status.is_some_and(|s| s.is_terminal()) {
                return Err(AiError::CommandBuild(
                    "cannot discard worktree for a still-running run".into(),
                ));
            }
            let wt = session
                .worktree_path
                .clone()
                .ok_or_else(|| AiError::CommandBuild("session has no worktree path".into()))?;
            let repo_root = wt
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .unwrap_or(wt.as_path())
                .to_path_buf();
            let slug = wt
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| session_id.to_string());
            (wt, repo_root, session.provider, slug)
        };

        // The worktree path lives under <repo>/<ai_worktree_root>/<slug> so
        // we can't perfectly recover the *repo* root from the worktree alone
        // if the user set a custom absolute root. We still try to locate a
        // .git above the worktree; if we can, we use libgit to remove.
        if let Ok(repo) = git_engine::Repository::open(&repo_root) {
            let path_str = worktree_path
                .to_str()
                .ok_or_else(|| AiError::CommandBuild("worktree path is not UTF-8".into()))?;
            let _ = repo.remove_worktree(path_str, true);
            let branch_name = format!("ai/{}/{}", provider_slug(provider), slug);
            // Best-effort branch delete via the system git binary — keeps
            // app-core free of a git2 dependency.
            let _ = std::process::Command::new("git")
                .current_dir(&repo_root)
                .args(["branch", "-D", &branch_name])
                .output();
        } else {
            // Fall back to best-effort filesystem removal.
            let _ = std::fs::remove_dir_all(&worktree_path);
        }

        // Drop the session from the registry entirely.
        {
            let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
            // Reverse-find the task_id pointing at this session so the
            // routing map doesn't keep a dangling key after discard.
            let stale_task_ids: Vec<TaskId> = guard
                .task_id_to_session
                .iter()
                .filter_map(|(tid, sid)| (sid == session_id).then_some(*tid))
                .collect();
            for tid in stale_task_ids {
                guard.task_id_to_session.remove(&tid);
                guard.pending_lines.remove(&tid);
            }
            guard.sessions.retain(|s| s.id != session_id);
            guard.pending.remove(session_id);
            guard.before_snapshots.remove(session_id);
        }
        Ok(())
    }

    // ── Internal dispatch & sink callbacks ───────────────────────────────────

    fn dispatch(
        self: &Arc<Self>,
        session_id: &str,
        provider: &dyn AiProvider,
        input: &AiBackgroundRunInput,
    ) -> Result<TaskId, AiError> {
        let cmd = provider.launch_background(input.clone())?;
        let (program, args) = command_to_parts(&cmd);
        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let kind = TaskKind::AiBackground {
            session_id: session_id.to_string(),
            provider: provider_slug(input.provider).to_string(),
            worktree_path: input.worktree_path.to_string_lossy().to_string(),
        };

        let stdin = if provider.background_uses_stdin_prompt() {
            Some(input.prompt.clone())
        } else {
            None
        };

        let cwd = input.worktree_path.clone();
        let label = format!("AI background: {}", session_id);
        let manager = Arc::clone(&self.task_manager);

        // Capture a baseline snapshot of the worktree *right before* we
        // spawn the subprocess. Stored on the coordinator so
        // `on_task_finished` can diff against it and emit a
        // `MutationKind::Ai { source }` event when the run actually
        // touched refs / HEAD / status. Snapshot failures are logged
        // but non-fatal — we simply skip the emit at the end.
        match Snapshot::capture(&cwd) {
            Ok(snap) => {
                let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
                guard.before_snapshots.insert(session_id.to_string(), snap);
            }
            Err(err) => {
                warn!(
                    session_id,
                    error = %err,
                    "failed to capture AI background baseline snapshot; \
                     will skip project-mutated emit on completion"
                );
            }
        }

        // task-runner is async only, so we wrap the spawn call in a blocking
        // call — this function is sync and invoked from a Tauri command that
        // itself runs in a tokio worker, so we re-enter the runtime.
        let task_id = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                manager
                    .spawn_with_options(SpawnOptions {
                        label,
                        command: &program,
                        args: &args_refs,
                        cwd: cwd.as_path(),
                        cancellable: true,
                        kind,
                        stdin,
                    })
                    .await
            })
        });

        // Register the task_id → session_id mapping immediately, then
        // drain any output that already arrived (the reader task spawned
        // by `spawn_with_options` runs on a different tokio worker and
        // can fire `on_task_output` before this point — see the
        // CoordinatorInner doc on `pending_lines`). Without this, the
        // first burst of stream-json events from a fast provider was
        // silently dropped.
        let drained: Vec<String> = {
            let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
            guard
                .task_id_to_session
                .insert(task_id, session_id.to_string());
            guard.pending_lines.remove(&task_id).unwrap_or_default()
        };
        for line in drained {
            self.event_sink.on_output(session_id, &line);
            if let Some(usage) = parse_claude_usage(&line) {
                self.update_session(session_id, |s| {
                    if let Some(AiBackgroundRunStatus::Completed { token_usage, .. }) =
                        s.background_status.as_mut()
                    {
                        *token_usage = Some(usage);
                    }
                });
            }
        }
        Ok(task_id)
    }

    fn update_session<F: FnOnce(&mut AiSession)>(&self, session_id: &str, f: F) {
        let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
        if let Some(session) = guard.sessions.iter_mut().find(|s| s.id == session_id) {
            f(session);
        }
    }

    /// Called by the Tauri-side sink for every output line produced by an
    /// AI background task. Parses Claude Code's JSON stream for token
    /// usage and forwards the raw line to the frontend.
    ///
    /// Lookup order:
    /// 1. `task_id_to_session` — direct map populated the moment
    ///    `dispatch` returns. Hits in the steady state.
    /// 2. `sessions` walk by `task_id` field — kept as a fallback in
    ///    case the map miss is structural (e.g. a stale event after
    ///    discard).
    /// 3. Buffer the line on `pending_lines[task_id]` — handles the
    ///    race where the reader task fires its first read before the
    ///    dispatch could insert into the map. The buffer is drained at
    ///    the tail of `dispatch` so no output is permanently lost.
    pub fn on_task_output(self: &Arc<Self>, task_id: TaskId, line: &str) {
        let session_id = {
            let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
            if let Some(sid) = guard.task_id_to_session.get(&task_id).cloned() {
                Some(sid)
            } else if let Some(sid) = guard
                .sessions
                .iter()
                .find(|s| s.task_id == Some(task_id))
                .map(|s| s.id.clone())
            {
                Some(sid)
            } else {
                guard
                    .pending_lines
                    .entry(task_id)
                    .or_default()
                    .push(line.to_string());
                None
            }
        };
        let Some(session_id) = session_id else {
            return;
        };
        self.event_sink.on_output(&session_id, line);

        // Try to extract usage metadata opportunistically.
        if let Some(usage) = parse_claude_usage(line) {
            self.update_session(&session_id, |s| {
                if let Some(AiBackgroundRunStatus::Completed { token_usage, .. }) =
                    s.background_status.as_mut()
                {
                    *token_usage = Some(usage);
                }
            });
        }
    }

    /// Called by the Tauri-side sink when a task finishes (any status).
    pub fn on_task_finished(
        self: &Arc<Self>,
        task_id: TaskId,
        exit_code: Option<i32>,
        was_cancelled: bool,
        error_text: Option<String>,
    ) {
        let session_id = {
            let guard = self.inner.lock().expect("coordinator mutex poisoned");
            guard
                .sessions
                .iter()
                .find(|s| s.task_id == Some(task_id))
                .map(|s| s.id.clone())
        };
        let Some(session_id) = session_id else {
            return;
        };

        let status = if was_cancelled {
            AiBackgroundRunStatus::Cancelled
        } else if matches!(exit_code, Some(0)) {
            AiBackgroundRunStatus::Completed {
                exit_code: 0,
                token_usage: None,
            }
        } else if let Some(msg) = error_text {
            AiBackgroundRunStatus::Failed { message: msg }
        } else {
            AiBackgroundRunStatus::Failed {
                message: format!("exit code {}", exit_code.unwrap_or(-1)),
            }
        };

        self.update_session(&session_id, |s| {
            s.background_status = Some(status.clone());
            s.is_active = false;
        });
        if let Some(snapshot) = self.get(&session_id) {
            self.event_sink.on_status(&snapshot);
        }

        // Diff the pre-spawn snapshot against current worktree state and
        // emit a `MutationKind::Ai { source }` event when refs/HEAD/status/
        // stashes/worktrees/remotes moved. The emit runs for every terminal
        // state (completed, failed, cancelled) because a partially-applied
        // AI run still produces real mutations the UI must refresh.
        let (before, worktree_path, provider_kind) = {
            let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
            let before = guard.before_snapshots.remove(&session_id);
            let session = guard.sessions.iter().find(|s| s.id == session_id).cloned();
            let worktree_path = session.as_ref().and_then(|s| s.worktree_path.clone());
            let provider_kind = session.as_ref().map(|s| s.provider);
            (before, worktree_path, provider_kind)
        };
        if let (Some(before), Some(worktree_path), Some(provider_kind)) =
            (before, worktree_path, provider_kind)
        {
            match Snapshot::capture(&worktree_path) {
                Ok(after) => {
                    let flags = before.diff(&after);
                    if !flags.is_empty() {
                        let source = ai_source_for(provider_kind);
                        self.event_sink
                            .on_repo_mutated(&worktree_path, source, flags);
                    }
                }
                Err(err) => warn!(
                    session_id = %session_id,
                    error = %err,
                    "failed to capture AI background post-run snapshot; \
                     skipping project-mutated emit"
                ),
            }
        }

        // Try to drain the queue.
        self.try_dispatch_next();
    }

    fn try_dispatch_next(self: &Arc<Self>) {
        let next_id = {
            let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
            let running_count = guard
                .sessions
                .iter()
                .filter(|s| {
                    matches!(
                        s.background_status.as_ref(),
                        Some(AiBackgroundRunStatus::Running)
                    )
                })
                .count() as u32;
            if running_count >= guard.concurrency_cap {
                return;
            }
            guard.queue.pop_front()
        };

        let Some(session_id) = next_id else {
            return;
        };

        let pending = {
            let mut guard = self.inner.lock().expect("coordinator mutex poisoned");
            guard.pending.remove(&session_id)
        };
        let Some(pending) = pending else {
            return;
        };

        let provider: Box<dyn AiProvider> = (self.provider_factory)(pending.provider_kind);

        match self.dispatch(&session_id, provider.as_ref(), &pending.input) {
            Ok(task_id) => {
                self.update_session(&session_id, |s| {
                    s.task_id = Some(task_id);
                    s.background_status = Some(AiBackgroundRunStatus::Running);
                });
                if let Some(snapshot) = self.get(&session_id) {
                    self.event_sink.on_status(&snapshot);
                }
            }
            Err(e) => {
                self.update_session(&session_id, |s| {
                    s.background_status = Some(AiBackgroundRunStatus::Failed {
                        message: e.to_string(),
                    });
                    s.is_active = false;
                });
                if let Some(snapshot) = self.get(&session_id) {
                    self.event_sink.on_status(&snapshot);
                }
            }
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Resolve the absolute worktree-root directory for AI runs.
///
/// If `override_` is absolute it is returned as-is. Otherwise it is joined
/// onto `repo_root`. When `override_` is `None`, the default
/// [`DEFAULT_AI_WORKTREE_ROOT`] is used.
pub fn resolve_worktree_root(repo_root: &Path, override_: Option<&str>) -> PathBuf {
    match override_ {
        Some(raw) => {
            let candidate = PathBuf::from(raw);
            if candidate.is_absolute() {
                candidate
            } else {
                repo_root.join(candidate)
            }
        }
        None => repo_root.join(DEFAULT_AI_WORKTREE_ROOT),
    }
}

/// Slug the first 4 prompt tokens (lowercase, filesystem-safe).
pub fn slug_from_prompt(prompt: &str, skill: &Option<String>) -> String {
    let base = if prompt.trim().is_empty() {
        skill.clone().unwrap_or_else(|| "ai-run".to_string())
    } else {
        prompt.to_string()
    };
    let slug: String = base
        .split_whitespace()
        .take(4)
        .map(|tok| {
            tok.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|tok| !tok.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "ai-run".into()
    } else {
        slug
    }
}

/// Append `-2`, `-3`, ... to the slug until the resulting path doesn't exist.
pub fn unique_slug(base: &str, worktree_root: &Path) -> String {
    if !worktree_root.join(base).exists() {
        return base.to_string();
    }
    for n in 2..1_000 {
        let candidate = format!("{base}-{n}");
        if !worktree_root.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{base}-{}", now_millis().unwrap_or_default())
}

/// Resolve the absolute path the AI background coordinator asks the
/// model to write its post-run report to.
///
/// `<repo>/.beardgit/ai-reports/<session_id>.md`. Stays in the parent
/// repo (not the worktree) so the file survives a `Discard worktree`
/// action — that's the whole point of having a report separate from
/// the worktree's transient state.
pub fn report_path_for(repo_root: &Path, session_id: &str) -> PathBuf {
    repo_root
        .join(DEFAULT_AI_REPORTS_ROOT)
        .join(format!("{session_id}.md"))
}

/// Build the trailing instruction the coordinator appends to every bg
/// run prompt asking the model to drop a markdown report at
/// `report_path` once it's done.
///
/// The wording is deliberately tool-agnostic — we don't reference
/// Claude's `Write` tool by name so Codex / OpenCode honour the same
/// instruction. We do mention the `--dangerously-skip-permissions`
/// case because without it a permissions-bound provider can't actually
/// write the file and the user will end up with an empty report.
fn build_report_instruction(report_path: &Path) -> String {
    format!(
        "\n\n---\n\
         When you finish this task, write a brief markdown report to:\n\
         \n  {}\n\n\
         The report should cover, in this order:\n\
         - One-line summary of what was attempted.\n\
         - Outcome — success / partial / failure.\n\
         - Files changed (paths only; no need for diffs).\n\
         - Any errors or blockers hit.\n\
         - Suggested follow-up, if any.\n\
         \n\
         Write the report even if you couldn't complete the task. Keep \
         it under 500 words. If you don't have permission to write \
         files, skip the report — the BeardGit UI surfaces the absence \
         as a hint to rerun with permissions enabled.\n",
        report_path.display(),
    )
}

fn provider_slug(kind: AiProviderKind) -> &'static str {
    match kind {
        AiProviderKind::ClaudeCode => "claude-code",
        AiProviderKind::Codex => "codex",
        AiProviderKind::OpenCode => "opencode",
    }
}

/// Map an [`AiProviderKind`] (domain enum) onto the
/// [`mutation_events::AiSource`] emitted alongside
/// [`mutation_events::MutationKind::Ai`]. Centralised so the coordinator
/// and any future caller stay consistent.
fn ai_source_for(kind: AiProviderKind) -> AiSource {
    match kind {
        AiProviderKind::ClaudeCode => AiSource::ClaudeCode,
        AiProviderKind::Codex => AiSource::Codex,
        AiProviderKind::OpenCode => AiSource::OpenCode,
    }
}

fn command_to_parts(cmd: &std::process::Command) -> (String, Vec<String>) {
    let program = cmd.get_program().to_string_lossy().to_string();
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    (program, args)
}

fn now_millis() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

/// Best-effort Claude Code stream-json parser for usage tallies.
///
/// Looks for a top-level `"usage"` object on `type == "result"` lines.
/// Returns `None` for any line that doesn't match that shape.
fn parse_claude_usage(line: &str) -> Option<AiTokenUsage> {
    let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    if value.get("type")?.as_str()? != "result" {
        return None;
    }
    let usage = value.get("usage")?;
    let input = usage.get("input_tokens").and_then(|v| v.as_u64())?;
    let output = usage.get("output_tokens").and_then(|v| v.as_u64())?;
    let total_cost_usd = value
        .get("total_cost_usd")
        .and_then(|v| v.as_f64())
        .or_else(|| value.get("cost_usd").and_then(|v| v.as_f64()));
    Some(AiTokenUsage {
        input,
        output,
        total_cost_usd,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_provider::ExecuteOptions;
    use std::process::Command as StdCommand;
    use std::sync::Mutex;
    use task_runner::{TaskEventSink, TaskManager};

    /// Stub provider that returns `sh -c "<script>"` as the background command.
    struct StubProvider {
        script: String,
        binary: PathBuf,
        uses_stdin: bool,
    }
    impl AiProvider for StubProvider {
        fn provider_kind(&self) -> AiProviderKind {
            AiProviderKind::ClaudeCode
        }
        fn binary_name(&self) -> &str {
            "stub"
        }
        fn detect_binary(&self) -> Option<PathBuf> {
            Some(self.binary.clone())
        }
        fn version(&self) -> Result<String, AiError> {
            Ok("stub-1.0".into())
        }
        fn detect_in_repo(&self, _repo_path: &Path) -> bool {
            true
        }
        fn build_execute_command(
            &self,
            _prompt: &str,
            cwd: &Path,
            _options: &ExecuteOptions,
        ) -> Result<StdCommand, AiError> {
            let mut cmd = StdCommand::new("sh");
            cmd.arg("-c").arg(&self.script).current_dir(cwd);
            Ok(cmd)
        }
        fn build_interactive_cmd(&self, cwd: &Path) -> Result<StdCommand, AiError> {
            let mut cmd = StdCommand::new("sh");
            cmd.current_dir(cwd);
            Ok(cmd)
        }
        fn launch_background(&self, input: AiBackgroundRunInput) -> Result<StdCommand, AiError> {
            let mut cmd = StdCommand::new("sh");
            cmd.arg("-c").arg(&self.script);
            cmd.current_dir(&input.worktree_path);
            Ok(cmd)
        }
        fn background_uses_stdin_prompt(&self) -> bool {
            self.uses_stdin
        }
        fn config_files(&self, _repo_path: &Path) -> Vec<ai_provider::AiConfigFile> {
            vec![]
        }
        fn instruction_files(&self, _repo_path: &Path) -> Vec<PathBuf> {
            vec![]
        }
    }

    /// Records every session event + output line for assertions.
    #[derive(Default)]
    struct RecordingSink {
        statuses: Mutex<Vec<(String, Option<AiBackgroundRunStatus>)>>,
        outputs: Mutex<Vec<(String, String)>>,
        mutations: Mutex<Vec<(PathBuf, AiSource, MutationFlags)>>,
    }
    impl AiBackgroundEventSink for RecordingSink {
        fn on_status(&self, session: &AiSession) {
            self.statuses
                .lock()
                .unwrap()
                .push((session.id.clone(), session.background_status.clone()));
        }
        fn on_output(&self, session_id: &str, line: &str) {
            self.outputs
                .lock()
                .unwrap()
                .push((session_id.to_string(), line.to_string()));
        }
        fn on_repo_mutated(&self, worktree_path: &Path, source: AiSource, flags: MutationFlags) {
            self.mutations
                .lock()
                .unwrap()
                .push((worktree_path.to_path_buf(), source, flags));
        }
    }

    /// Minimal TaskEventSink that bridges task-runner events back into the
    /// coordinator, so tests exercise the same glue that production uses.
    struct CoordinatorTestSink {
        coord: Mutex<Option<Arc<AiBackgroundCoordinator>>>,
    }
    impl CoordinatorTestSink {
        fn install(&self, coord: Arc<AiBackgroundCoordinator>) {
            *self.coord.lock().unwrap() = Some(coord);
        }
    }
    #[async_trait::async_trait]
    impl TaskEventSink for CoordinatorTestSink {
        async fn on_task_started(&self, _info: task_runner::TaskInfo) {}
        async fn on_task_output(
            &self,
            task_id: task_runner::TaskId,
            line: task_runner::OutputLine,
        ) {
            if let Some(ref coord) = *self.coord.lock().unwrap() {
                coord.on_task_output(task_id, &line.text);
            }
        }
        async fn on_task_completed(&self, info: task_runner::TaskInfo) {
            if let Some(ref coord) = *self.coord.lock().unwrap() {
                coord.on_task_finished(info.id, info.exit_code, false, None);
            }
        }
        async fn on_task_failed(&self, info: task_runner::TaskInfo) {
            if let Some(ref coord) = *self.coord.lock().unwrap() {
                let err_text = match &info.status {
                    task_runner::TaskStatus::Failed { error } => Some(error.clone()),
                    _ => None,
                };
                coord.on_task_finished(info.id, info.exit_code, false, err_text);
            }
        }
        async fn on_task_cancelled(&self, info: task_runner::TaskInfo) {
            if let Some(ref coord) = *self.coord.lock().unwrap() {
                coord.on_task_finished(info.id, info.exit_code, true, None);
            }
        }
    }

    fn init_test_repo() -> (tempfile::TempDir, String) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        {
            let mut cfg = repo.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@test.com").unwrap();
        }
        std::fs::write(tmp.path().join("README"), "hi").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("README")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        let head_branch = repo.head().unwrap().shorthand().unwrap().to_string();
        (tmp, head_branch)
    }

    async fn wait_terminal(coord: &AiBackgroundCoordinator, session_id: &str) {
        // 30 s budget — locally the spawn-and-finish round-trip lands in
        // <100 ms, but GitHub Actions runners under contention have been
        // observed to take several seconds to schedule the spawned task,
        // and the previous 5 s window flaked the workspace test job on
        // beta after release-day's compile-cache invalidation. Returning
        // as soon as the session is terminal means the upper bound only
        // ever applies to *failure* paths, not to the happy path.
        for _ in 0..3000 {
            if let Some(s) = coord.get(session_id)
                && s.background_status
                    .as_ref()
                    .is_some_and(|st| st.is_terminal())
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("session {session_id} did not reach terminal state in time");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn start_completes_and_emits_status_and_output() {
        let (tmp, base_branch) = init_test_repo();
        let sink_bridge = Arc::new(CoordinatorTestSink {
            coord: Mutex::new(None),
        });
        let task_manager = Arc::new(TaskManager::new(sink_bridge.clone()));
        let recorder = Arc::new(RecordingSink::default());
        let coord = Arc::new(AiBackgroundCoordinator::new(
            Arc::clone(&task_manager),
            recorder.clone(),
        ));
        sink_bridge.install(Arc::clone(&coord));

        let provider = StubProvider {
            script: "echo hello-world".into(),
            binary: PathBuf::from("/usr/bin/false"), // not used by launch_background
            uses_stdin: false,
        };

        let args = StartArgs {
            repo_root: tmp.path().to_path_buf(),
            provider: AiProviderKind::ClaudeCode,
            base_branch,
            prompt: "say hi".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            worktree_slug_override: None,
            worktree_root_override: None,
            auto_accept_permissions: false,
            concurrency_cap: 3,
            watcher_cached_snapshot: None,
        };
        let out = coord.start(args, &provider).expect("start must succeed");
        assert!(out.worktree_path.exists(), "worktree should be on disk");
        assert!(matches!(
            out.status,
            AiBackgroundRunStatus::Running | AiBackgroundRunStatus::Queued
        ));
        wait_terminal(&coord, &out.session_id).await;

        let finished = coord.get(&out.session_id).unwrap();
        match finished.background_status.unwrap() {
            AiBackgroundRunStatus::Completed { exit_code, .. } => assert_eq!(exit_code, 0),
            other => panic!("expected Completed, got {other:?}"),
        }

        let outputs = recorder.outputs.lock().unwrap().clone();
        assert!(
            outputs.iter().any(|(_, line)| line == "hello-world"),
            "expected stdout line, got {outputs:?}"
        );

        // Cleanup: discard the worktree.
        coord
            .discard_worktree(&out.session_id)
            .expect("discard should succeed");
        assert!(coord.get(&out.session_id).is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrency_cap_queues_extra_runs() {
        let (tmp, base_branch) = init_test_repo();
        let sink_bridge = Arc::new(CoordinatorTestSink {
            coord: Mutex::new(None),
        });
        let task_manager = Arc::new(TaskManager::new(sink_bridge.clone()));
        let recorder = Arc::new(RecordingSink::default());

        // Factory returns our stub so that queued runs also get the stub
        // when they finally dispatch.
        let factory: Arc<dyn Fn(AiProviderKind) -> Box<dyn AiProvider> + Send + Sync> =
            Arc::new(|_| {
                Box::new(StubProvider {
                    script: "sleep 0.3; echo done".into(),
                    binary: PathBuf::from("/usr/bin/false"),
                    uses_stdin: false,
                })
            });
        let coord = Arc::new(AiBackgroundCoordinator::with_provider_factory(
            Arc::clone(&task_manager),
            recorder.clone(),
            factory.clone(),
        ));
        sink_bridge.install(Arc::clone(&coord));

        // Use the factory result for the initial start calls too.
        let slow_box = factory(AiProviderKind::ClaudeCode);
        let slow = slow_box.as_ref();

        let args_for = |slug: &str, prompt: &str| StartArgs {
            repo_root: tmp.path().to_path_buf(),
            provider: AiProviderKind::ClaudeCode,
            base_branch: base_branch.clone(),
            prompt: prompt.to_string(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            worktree_slug_override: Some(slug.to_string()),
            worktree_root_override: None,
            auto_accept_permissions: false,
            concurrency_cap: 1, // tight cap forces queueing
            watcher_cached_snapshot: None,
        };

        let first = coord.start(args_for("r1", "first"), slow).unwrap();
        let second = coord.start(args_for("r2", "second"), slow).unwrap();

        assert!(matches!(first.status, AiBackgroundRunStatus::Running));
        assert!(matches!(second.status, AiBackgroundRunStatus::Queued));

        wait_terminal(&coord, &first.session_id).await;
        wait_terminal(&coord, &second.session_id).await;

        let s2 = coord.get(&second.session_id).unwrap();
        assert!(matches!(
            s2.background_status,
            Some(AiBackgroundRunStatus::Completed { .. })
        ));
        // Cleanup both worktrees so the tempdir can be dropped.
        let _ = coord.discard_worktree(&first.session_id);
        let _ = coord.discard_worktree(&second.session_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancel_queued_run_marks_it_cancelled_without_spawning() {
        let (tmp, base_branch) = init_test_repo();
        let sink_bridge = Arc::new(CoordinatorTestSink {
            coord: Mutex::new(None),
        });
        let task_manager = Arc::new(TaskManager::new(sink_bridge.clone()));
        let recorder = Arc::new(RecordingSink::default());
        let coord = Arc::new(AiBackgroundCoordinator::new(
            Arc::clone(&task_manager),
            recorder.clone(),
        ));
        sink_bridge.install(Arc::clone(&coord));

        let provider = StubProvider {
            script: "sleep 5".into(),
            binary: PathBuf::from("/usr/bin/false"),
            uses_stdin: false,
        };

        // Cap=1, so the second submission is queued.
        let first = coord
            .start(
                StartArgs {
                    repo_root: tmp.path().to_path_buf(),
                    provider: AiProviderKind::ClaudeCode,
                    base_branch: base_branch.clone(),
                    prompt: "first".into(),
                    skill: None,
                    saved_prompt_path: None,
                    resume_session_id: None,
                    worktree_slug_override: Some("a".into()),
                    worktree_root_override: None,
                    auto_accept_permissions: false,
                    concurrency_cap: 1,
                    watcher_cached_snapshot: None,
                },
                &provider,
            )
            .unwrap();
        let queued = coord
            .start(
                StartArgs {
                    repo_root: tmp.path().to_path_buf(),
                    provider: AiProviderKind::ClaudeCode,
                    base_branch: base_branch.clone(),
                    prompt: "second".into(),
                    skill: None,
                    saved_prompt_path: None,
                    resume_session_id: None,
                    worktree_slug_override: Some("b".into()),
                    worktree_root_override: None,
                    auto_accept_permissions: false,
                    concurrency_cap: 1,
                    watcher_cached_snapshot: None,
                },
                &provider,
            )
            .unwrap();
        assert!(matches!(queued.status, AiBackgroundRunStatus::Queued));

        coord.cancel(&queued.session_id).unwrap();
        let s = coord.get(&queued.session_id).unwrap();
        assert!(matches!(
            s.background_status,
            Some(AiBackgroundRunStatus::Cancelled)
        ));

        // Cancel the running one too so the test ends quickly.
        coord.cancel(&first.session_id).unwrap();
        wait_terminal(&coord, &first.session_id).await;
        let _ = coord.discard_worktree(&first.session_id);
    }

    #[test]
    fn slug_from_prompt_takes_first_four_tokens() {
        let slug = slug_from_prompt(
            "Refactor the logger module to async and remove globals",
            &None,
        );
        assert_eq!(slug, "refactor-the-logger-module");
    }

    #[test]
    fn slug_from_prompt_strips_punctuation() {
        let slug = slug_from_prompt("Fix bug #123 in parser!", &None);
        assert_eq!(slug, "fix-bug-123-in");
    }

    #[test]
    fn slug_from_prompt_falls_back_on_empty() {
        let slug = slug_from_prompt("", &None);
        assert_eq!(slug, "ai-run");
        let with_skill = slug_from_prompt("", &Some("review".into()));
        assert_eq!(with_skill, "review");
    }

    #[test]
    fn resolve_worktree_root_uses_default_when_none() {
        let repo = PathBuf::from("/tmp/repo");
        let root = resolve_worktree_root(&repo, None);
        assert_eq!(root, repo.join(".beardgit/ai-worktrees"));
    }

    #[test]
    fn resolve_worktree_root_absolute_override() {
        let repo = PathBuf::from("/tmp/repo");
        let root = resolve_worktree_root(&repo, Some("/var/ai-wt"));
        assert_eq!(root, PathBuf::from("/var/ai-wt"));
    }

    #[test]
    fn resolve_worktree_root_relative_override() {
        let repo = PathBuf::from("/tmp/repo");
        let root = resolve_worktree_root(&repo, Some("ai-runs"));
        assert_eq!(root, repo.join("ai-runs"));
    }

    #[test]
    fn unique_slug_untouched_when_free() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(unique_slug("feat", tmp.path()), "feat");
    }

    #[test]
    fn unique_slug_bumps_on_collision() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("feat")).unwrap();
        assert_eq!(unique_slug("feat", tmp.path()), "feat-2");
    }

    #[test]
    fn parse_claude_usage_extracts_tokens() {
        let line = r#"{"type":"result","subtype":"success","usage":{"input_tokens":123,"output_tokens":45},"total_cost_usd":0.0012}"#;
        let usage = parse_claude_usage(line).unwrap();
        assert_eq!(usage.input, 123);
        assert_eq!(usage.output, 45);
        assert_eq!(usage.total_cost_usd, Some(0.0012));
    }

    #[test]
    fn parse_claude_usage_ignores_non_result_lines() {
        assert!(parse_claude_usage(r#"{"type":"assistant","text":"hi"}"#).is_none());
        assert!(parse_claude_usage("not json at all").is_none());
    }

    #[test]
    fn ai_source_for_maps_each_provider_kind() {
        assert_eq!(
            ai_source_for(AiProviderKind::ClaudeCode),
            AiSource::ClaudeCode
        );
        assert_eq!(ai_source_for(AiProviderKind::Codex), AiSource::Codex);
        assert_eq!(ai_source_for(AiProviderKind::OpenCode), AiSource::OpenCode);
    }

    /// End-to-end integration: when an AI background run creates a commit
    /// inside its worktree, the coordinator fires `on_repo_mutated` with
    /// `MutationKind::Ai { source }` for the matching provider and
    /// non-empty flags that cover HEAD + refs + status.
    #[tokio::test(flavor = "multi_thread")]
    async fn mutation_emitted_when_ai_run_commits_inside_worktree() {
        let (tmp, base_branch) = init_test_repo();
        let sink_bridge = Arc::new(CoordinatorTestSink {
            coord: Mutex::new(None),
        });
        let task_manager = Arc::new(TaskManager::new(sink_bridge.clone()));
        let recorder = Arc::new(RecordingSink::default());
        let coord = Arc::new(AiBackgroundCoordinator::new(
            Arc::clone(&task_manager),
            recorder.clone(),
        ));
        sink_bridge.install(Arc::clone(&coord));

        // Shell script that creates a tracked file and commits — this is
        // what a real Claude Code / Codex run would eventually do.
        let script = "\
            set -e; \
            git config user.email test@test.com; \
            git config user.name Test; \
            echo ai-payload > ai.txt; \
            git add ai.txt; \
            git commit -m 'ai commit'";

        let provider = StubProvider {
            script: script.into(),
            binary: PathBuf::from("/usr/bin/false"),
            uses_stdin: false,
        };
        let args = StartArgs {
            repo_root: tmp.path().to_path_buf(),
            provider: AiProviderKind::Codex,
            base_branch,
            prompt: "commit something".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            worktree_slug_override: Some("mut-commit".into()),
            worktree_root_override: None,
            auto_accept_permissions: false,
            concurrency_cap: 3,
            watcher_cached_snapshot: None,
        };
        let out = coord.start(args, &provider).expect("start must succeed");
        wait_terminal(&coord, &out.session_id).await;

        let mutations = recorder.mutations.lock().unwrap().clone();
        let hit = mutations
            .iter()
            .find(|(p, _, _)| p == &out.worktree_path)
            .cloned()
            .expect("expected on_repo_mutated for the worktree path");
        let (_, source, flags) = hit;
        assert_eq!(source, AiSource::Codex, "source should track provider kind");
        assert!(
            flags.head_changed,
            "HEAD moved due to the commit — flags.head_changed must be true"
        );
        assert!(
            flags.refs_changed,
            "refs moved due to the commit — flags.refs_changed must be true"
        );

        let _ = coord.discard_worktree(&out.session_id);
    }

    /// A background run that touches nothing must not emit a mutation
    /// event — the TS listener would otherwise reload the graph for no
    /// reason.
    #[tokio::test(flavor = "multi_thread")]
    async fn no_mutation_emitted_when_worktree_state_unchanged() {
        let (tmp, base_branch) = init_test_repo();
        let sink_bridge = Arc::new(CoordinatorTestSink {
            coord: Mutex::new(None),
        });
        let task_manager = Arc::new(TaskManager::new(sink_bridge.clone()));
        let recorder = Arc::new(RecordingSink::default());
        let coord = Arc::new(AiBackgroundCoordinator::new(
            Arc::clone(&task_manager),
            recorder.clone(),
        ));
        sink_bridge.install(Arc::clone(&coord));

        let provider = StubProvider {
            script: "echo no-op".into(),
            binary: PathBuf::from("/usr/bin/false"),
            uses_stdin: false,
        };
        let args = StartArgs {
            repo_root: tmp.path().to_path_buf(),
            provider: AiProviderKind::ClaudeCode,
            base_branch,
            prompt: "noop".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            worktree_slug_override: Some("mut-noop".into()),
            worktree_root_override: None,
            auto_accept_permissions: false,
            concurrency_cap: 3,
            watcher_cached_snapshot: None,
        };
        let out = coord.start(args, &provider).expect("start must succeed");
        wait_terminal(&coord, &out.session_id).await;

        let mutations = recorder.mutations.lock().unwrap().clone();
        assert!(
            mutations.is_empty(),
            "expected no mutation events for a no-op AI run, got {mutations:?}"
        );

        let _ = coord.discard_worktree(&out.session_id);
    }

    /// When the caller hands a watcher's cached-snapshot Arc to
    /// `start()`, the coordinator must overwrite it with a fresh capture
    /// covering the new ai/* ref + bumped worktree count. The watcher's
    /// next debounced batch then diffs CURRENT vs that updated cache and
    /// finds an empty diff, so no `project-mutated` is emitted for the
    /// worktree creation we just performed.
    #[tokio::test(flavor = "multi_thread")]
    async fn start_resyncs_provided_watcher_snapshot() {
        use std::sync::Mutex as StdMutex;

        let (tmp, base_branch) = init_test_repo();
        let sink_bridge = Arc::new(CoordinatorTestSink {
            coord: Mutex::new(None),
        });
        let task_manager = Arc::new(TaskManager::new(sink_bridge.clone()));
        let recorder = Arc::new(RecordingSink::default());
        let coord = Arc::new(AiBackgroundCoordinator::new(
            Arc::clone(&task_manager),
            recorder.clone(),
        ));
        sink_bridge.install(Arc::clone(&coord));

        // Seed the shared cache the way `RepoWatcher::start` would: a
        // capture of the repo right before the AI flow runs.
        let pre_snapshot = Snapshot::capture(tmp.path()).expect("pre snapshot");
        let cache = Arc::new(StdMutex::new(pre_snapshot.clone()));

        let provider = StubProvider {
            script: "true".into(),
            binary: PathBuf::from("/usr/bin/false"),
            uses_stdin: false,
        };
        let args = StartArgs {
            repo_root: tmp.path().to_path_buf(),
            provider: AiProviderKind::ClaudeCode,
            base_branch,
            prompt: "resync test".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            worktree_slug_override: Some("resync".into()),
            worktree_root_override: None,
            auto_accept_permissions: false,
            concurrency_cap: 3,
            watcher_cached_snapshot: Some(Arc::clone(&cache)),
        };
        let out = coord.start(args, &provider).expect("start must succeed");

        // The cache the coordinator stored must now match the post-creation
        // state — same refs (with the new ai/claude-code/resync entry), same
        // worktree count (one more than before).
        let cached_after = cache.lock().unwrap().clone();
        assert_ne!(
            cached_after, pre_snapshot,
            "cache should have been overwritten with a fresh capture"
        );
        assert_eq!(
            cached_after.worktree_count,
            pre_snapshot.worktree_count + 1,
            "post-start cache should reflect the new worktree"
        );
        assert!(
            cached_after
                .refs
                .keys()
                .any(|r| r == "refs/heads/ai/claude-code/resync"),
            "post-start cache should include the new ai branch ref"
        );

        // Most important: a watcher-style debounce that fires *now* would
        // diff CURRENT vs `cached_after`. We simulate that — if the diff
        // is non-empty, the watcher would emit and trigger the spurious
        // graph reload we're trying to avoid.
        let current = Snapshot::capture(tmp.path()).expect("post-start capture");
        let flags = cached_after.diff(&current);
        assert!(
            flags.is_empty(),
            "watcher debounce after start() should diff to empty (got {flags:?})"
        );

        wait_terminal(&coord, &out.session_id).await;
        let _ = coord.discard_worktree(&out.session_id);
    }

    /// Every bg run gets a fixed suffix asking the AI to write its
    /// post-run report at `<repo>/.beardgit/ai-reports/<session>.md`.
    /// That contract is what powers the Report pane in the UI, so guard
    /// against accidental regressions: the prompt fed to the provider
    /// must contain (a) the absolute report path, and (b) the
    /// instructional sentence asking the AI to "write a brief markdown
    /// report".
    #[tokio::test(flavor = "multi_thread")]
    async fn start_appends_report_instruction_to_prompt() {
        use std::sync::Mutex as StdMutex;

        let (tmp, base_branch) = init_test_repo();

        // Capture the prompt the coordinator hands to `launch_background`
        // so we can assert the suffix is present.
        let captured: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));
        struct CaptureProvider {
            seen: Arc<StdMutex<Option<String>>>,
        }
        impl AiProvider for CaptureProvider {
            fn provider_kind(&self) -> AiProviderKind {
                AiProviderKind::ClaudeCode
            }
            fn binary_name(&self) -> &str {
                "stub"
            }
            fn detect_binary(&self) -> Option<PathBuf> {
                Some(PathBuf::from("/usr/bin/false"))
            }
            fn version(&self) -> Result<String, AiError> {
                Ok("stub".into())
            }
            fn detect_in_repo(&self, _repo_path: &Path) -> bool {
                true
            }
            fn build_execute_command(
                &self,
                _prompt: &str,
                cwd: &Path,
                _options: &ExecuteOptions,
            ) -> Result<StdCommand, AiError> {
                let mut cmd = StdCommand::new("true");
                cmd.current_dir(cwd);
                Ok(cmd)
            }
            fn build_interactive_cmd(&self, cwd: &Path) -> Result<StdCommand, AiError> {
                let mut cmd = StdCommand::new("true");
                cmd.current_dir(cwd);
                Ok(cmd)
            }
            fn launch_background(
                &self,
                input: AiBackgroundRunInput,
            ) -> Result<StdCommand, AiError> {
                *self.seen.lock().unwrap() = Some(input.prompt.clone());
                let mut cmd = StdCommand::new("true");
                cmd.current_dir(&input.worktree_path);
                Ok(cmd)
            }
            fn background_uses_stdin_prompt(&self) -> bool {
                false
            }
            fn config_files(&self, _repo_path: &Path) -> Vec<ai_provider::AiConfigFile> {
                vec![]
            }
            fn instruction_files(&self, _repo_path: &Path) -> Vec<PathBuf> {
                vec![]
            }
        }

        let sink_bridge = Arc::new(CoordinatorTestSink {
            coord: Mutex::new(None),
        });
        let task_manager = Arc::new(TaskManager::new(sink_bridge.clone()));
        let recorder = Arc::new(RecordingSink::default());
        let coord = Arc::new(AiBackgroundCoordinator::new(
            Arc::clone(&task_manager),
            recorder.clone(),
        ));
        sink_bridge.install(Arc::clone(&coord));

        let provider = CaptureProvider {
            seen: Arc::clone(&captured),
        };
        let args = StartArgs {
            repo_root: tmp.path().to_path_buf(),
            provider: AiProviderKind::ClaudeCode,
            base_branch,
            prompt: "Refactor the logger".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            worktree_slug_override: Some("report-test".into()),
            worktree_root_override: None,
            auto_accept_permissions: false,
            concurrency_cap: 3,
            watcher_cached_snapshot: None,
        };
        let out = coord.start(args, &provider).expect("start must succeed");

        let prompt = captured
            .lock()
            .unwrap()
            .clone()
            .expect("launch_background should have been called");
        assert!(
            prompt.contains("Refactor the logger"),
            "user prompt must still be present, got: {prompt}"
        );
        assert!(
            prompt.contains("write a brief markdown report"),
            "report instruction must be appended, got: {prompt}"
        );
        let expected_path = report_path_for(tmp.path(), &out.session_id);
        assert!(
            prompt.contains(expected_path.to_str().unwrap()),
            "absolute report path must be in the prompt, got: {prompt}"
        );

        // Reports dir must exist on disk before the AI runs so its
        // Write tool doesn't fail on a missing parent.
        assert!(
            expected_path.parent().unwrap().is_dir(),
            "reports dir should be created up-front"
        );

        let _ = coord.discard_worktree(&out.session_id);
    }

    /// Output that arrives before `dispatch` registers the
    /// `task_id → session_id` mapping must be buffered + drained, not
    /// silently dropped. We exercise this directly by hitting
    /// `on_task_output` with an unknown task_id (mimicking a reader
    /// task that beat dispatch's insert), then completing the
    /// registration via `dispatch`-equivalent state mutation, and
    /// asserting the recorder saw the buffered line.
    #[tokio::test(flavor = "multi_thread")]
    async fn early_task_output_is_buffered_and_drained() {
        let (_tmp, _base_branch) = init_test_repo();
        let sink_bridge = Arc::new(CoordinatorTestSink {
            coord: Mutex::new(None),
        });
        let task_manager = Arc::new(TaskManager::new(sink_bridge.clone()));
        let recorder = Arc::new(RecordingSink::default());
        let coord = Arc::new(AiBackgroundCoordinator::new(
            Arc::clone(&task_manager),
            recorder.clone(),
        ));
        sink_bridge.install(Arc::clone(&coord));

        let task_id: TaskId = 9999;
        let session_id = "aibg-early-output-test";

        // 1. Reader task fires before dispatch registered the mapping.
        coord.on_task_output(task_id, "early-line-1");
        coord.on_task_output(task_id, "early-line-2");

        // No output should have reached the sink yet — the lines are
        // buffered on `pending_lines`.
        assert!(recorder.outputs.lock().unwrap().is_empty());

        // 2. Simulate dispatch's registration + drain. We poke directly
        //    at the inner state because we don't want to spin up a real
        //    process here — the goal is to verify the routing logic.
        {
            let mut guard = coord.inner.lock().expect("coordinator mutex");
            guard
                .task_id_to_session
                .insert(task_id, session_id.to_string());
            let drained = guard.pending_lines.remove(&task_id).unwrap_or_default();
            drop(guard);
            for line in drained {
                coord.event_sink.on_output(session_id, &line);
            }
        }

        // 3. Lines that arrive after registration go through the fast
        //    path — and the previously-buffered lines should already be
        //    on the recorder.
        coord.on_task_output(task_id, "later-line-3");

        let outputs = recorder.outputs.lock().unwrap().clone();
        let lines: Vec<&str> = outputs.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            lines,
            vec!["early-line-1", "early-line-2", "later-line-3"],
            "buffered lines must be delivered in arrival order alongside live ones"
        );
    }
}
