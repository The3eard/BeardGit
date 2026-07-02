# Spec 05 — Enforce and type the IPC contract (guardrail → typed errors → drift check)

**Priority:** P1 · **Effort:** M (phased; Phase 1 is a day) · **Branch:** `fix/ipc-contract-guardrail`, then `feat/typed-ipc-errors` · **Depends on:** —

## Problem

The documented "three-file contract" (Rust handler ↔ `tauri.ts` ↔ `types/`) is pure manual discipline, and it has already failed:

1. **No enforcement.** `src/lib/api/tauri.ts` is 1 939 lines / 277 hand-written wrappers; `src/lib/types/index.ts` is 1 199 lines of hand-mirrored Rust structs. No codegen (`ts-rs`/`specta`/`typeshare` absent), no drift check in `scripts/`, no lint forbidding raw `invoke()`.
2. **Proven drift:** the entire `requests/` feature bypasses the contract — **12 files** under `src/lib/components/requests/` import `invoke` from `@tauri-apps/api/core` directly (e.g. `CollectionsTree.svelte:320,364,380`, `EnvManagerDialog.svelte:146,186`, `UrlBar.svelte:131,192`) with untyped returns, no `runMutation` toasts, no types.
3. **Stringly-typed errors:** 316 `Result<_, String>` command signatures and 631 `map_err(…to_string)` in app-core discard the typed enums the crates already define (`GitError` at `crates/git-engine/src/error.rs:10`, `CloneRepoError` `clone.rs:49`, `OpenProjectError` `project.rs:19`, `InitRepoError` `init.rs:75`, `RepoConfigError` `repo_config.rs:407`). The frontend can't branch on error kind (auth-required vs conflict vs retry) and error text can't be i18n'd.

## Goal (success criteria)

- ESLint fails on any `@tauri-apps/api/core` import outside `src/lib/api/` — and the codebase passes.
- Every `requests_*` call goes through typed `tauri.ts` wrappers; mutating ones through `runMutation`.
- A CI step fails when a `#[tauri::command]` exists without a `tauri.ts` wrapper (or vice versa).
- New/high-value commands return a structured error `{ code, message }`; frontend `errors.ts` can switch on `code`.

## Design

**Phase 1 — guardrail + fold in requests (do first, one day):**
- `no-restricted-imports` in `eslint.config.js` for `@tauri-apps/api/core` (allow only under `src/lib/api/`), message pointing at `src/CLAUDE.md`.
- Add the ~22 `requests_*` wrappers to `tauri.ts` with proper types in `types/`; migrate the 12 files; route mutations (`requests_save`, `requests_delete`, `requests_rename`, env CRUD…) through `runMutation` so they gain toasts. Behavior-only change; the untested feature makes this low-risk mechanically but verify the panel end-to-end manually.

**Phase 2 — command↔wrapper drift check (cheap, catches renames):**
- `scripts/check-ipc-contract.mjs`: extract command names from `grep -r "#\[tauri::command\]"` + the following `fn` name across `crates/app-core`, and `invoke("…")` strings from `src/lib/api/tauri.ts`; diff both directions; exit non-zero on mismatch. Wire into `ci.yml` frontend job. This is 80% of codegen's safety at 2% of its cost — names and existence, not shapes.

**Phase 3 — typed error envelope (incremental):**
- Define in app-core: `#[derive(Serialize)] struct IpcError { code: &'static str, message: String }` (+ optional `details`). `impl From<GitError>`, `From<CloneRepoError>`, etc., mapping variants to stable codes (`"auth_required"`, `"merge_conflict"`, `"not_fast_forward"`, `"binary_file"`, …).
- Migrate commands opportunistically — start where the frontend actually wants to branch: clone/auth (`provider_auth`, `cli_auth`), push/pull (auth + non-FF), open_repo. `Result<T, String>` and `Result<T, IpcError>` coexist; `src/lib/api/errors.ts` learns to parse both (string fallback keeps old paths working).
- Update `runMutation` to use `code` for i18n-able toast titles where mapped, raw message as detail.

**Deliberately deferred — full codegen (tauri-specta):** generating all 299 wrappers + types requires `specta::Type` derives across every IPC struct in a dozen crates; big-bang migration, real payoff, but Phases 1–3 remove the day-to-day risk first. Revisit as its own project once typed errors settle (specta would then also generate the error types).

## Files to touch

- `eslint.config.js`; `src/lib/api/tauri.ts`, `src/lib/api/errors.ts`, `src/lib/api/runMutation.ts`; `src/lib/types/index.ts`; 12 files under `src/lib/components/requests/`.
- `scripts/check-ipc-contract.mjs` (new); `.github/workflows/ci.yml`.
- `crates/app-core/src/` — new `ipc_error.rs` + per-command migrations.

## Verification

1. Add a raw `invoke` to a component → `npm run lint` fails. Rename a Rust command without touching `tauri.ts` → CI drift check fails.
2. Vitest for `errors.ts` parsing both error shapes; a store test asserting a mapped code surfaces (mock a `push` auth failure).
3. Manual pass over the Requests panel: create/save/rename/delete request, env switch/secret prompt, run + history — all with toasts.

## Out of scope

- Migrating all 316 signatures at once (opportunistic only).
- tauri-specta adoption (tracked in the roadmap as a follow-up decision).
