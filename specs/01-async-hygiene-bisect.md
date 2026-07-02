# Spec 01 — Stop blocking the Tokio runtime: bisect + AI coordinator

**Priority:** P0 (quick win) · **Effort:** S (½–1 day) · **Branch:** `fix/async-hygiene-bisect` · **Depends on:** —

## Problem

The workspace rule ("app-core commands that do heavy git/CLI work must use `spawn_blocking` or the task-runner" — `crates/CLAUDE.md`) is followed everywhere except two places:

1. **All 8 bisect commands** in `crates/app-core/src/commands/bisect.rs` are `pub async fn` that call synchronous `git_engine::bisect::*` (which shells out to the git CLI — 10 `Command::new` sites in `crates/git-engine/src/bisect.rs`) directly on the async runtime. Worst case: `bisect_run_auto` (`bisect.rs:78-84`) executes a **user-supplied test command across many commits** — potentially minutes of synchronous work occupying a runtime worker, starving all other IPC. Audit sweep: bisect is the *sole* offender; stash/worktree/graph/advanced/branch/repository all offload correctly.

2. **AI coordinator sync/async sandwich**: `crates/app-core/src/ai_background.rs:642-643` does `tokio::task::block_in_place(|| Handle::current().block_on(async { manager.spawn_with_options(…) }))`, and `crates/app-core/src/commands/ai_background.rs:142-145` wraps the whole coordinator start in `block_in_place`. `block_in_place` panics on a current-thread runtime and burns a worker thread; re-entering the runtime via `block_on` is fragile under refactor.

## Goal (success criteria)

- No `#[tauri::command]` performs synchronous git/CLI work directly on the runtime: `grep -rn "git_engine::" crates/app-core/src/commands/bisect.rs` shows every call site inside `spawn_blocking` or task-runner.
- `bisect_run_auto` is **cancellable** and streams progress (task-runner), so a runaway test command can be stopped from the UI.
- Zero `block_in_place` in `app-core`: `grep -rn block_in_place crates/app-core/src` returns nothing.
- UI stays interactive during an auto-bisect on a large repo (manual check: scroll the graph while bisect runs).

## Design

**Bisect (mechanical):**
- Wrap each of the 7 short bisect command bodies in `tauri::async_runtime::spawn_blocking` (match the pattern in `commands/repository.rs::open_repo`).
- Route `bisect_run_auto` through the `task-runner` crate instead (same shape as `commands/submodule.rs::update_submodule`): spawn as a managed task, stream per-step output (`git bisect run` output is line-oriented), support cancellation. Frontend: the bisect store already polls state via `bisect_get_state`; add task-id plumbing so the existing task popover shows progress/cancel.

**AI coordinator:**
- Make `Coordinator::start` (and the internal spawn path at `ai_background.rs:642`) `async` end-to-end; `.await` `manager.spawn_with_options(…)` directly.
- Delete both `block_in_place` wrappers; the command handler is already `async`.

## Files to touch

- `crates/app-core/src/commands/bisect.rs` — all 8 handlers.
- `crates/app-core/src/ai_background.rs` — `start` signature + spawn site.
- `crates/app-core/src/commands/ai_background.rs` — call site.
- `src/lib/stores/bisect.ts` (or equivalent) — only if run_auto moves to task lifecycle.

## Verification

1. `cargo test --workspace` + `cargo clippy --workspace -- -D warnings` green.
2. New test: bisect_run_auto through task-runner is cancellable (unit test at the task-runner level with a `sleep` test command).
3. Manual: start auto-bisect with a slow test command in a big repo; graph scrolling and other commands stay responsive; cancel works.

## Out of scope

- The sync (non-async) read commands noted in the audit (`get_branch_commits`, `get_file_statuses` run on the main thread in Tauri 2). Profile them separately; convert only if measurements justify it.
