# Spec 09 — Shrink app-core back to glue (repo_config, AI coordinator, AI-provider dedup)

**Priority:** P2 · **Effort:** L (phased; each phase lands independently) · **Branch:** `chore/decompose-repo-config`, `chore/extract-ai-runner`, `chore/ai-provider-common` · **Depends on:** Spec 01 (the coordinator goes async there; extract after)

## Problem

The layering rule says app-core hosts "thin `#[tauri::command]` glue" — but ~7.7K of its 20 247 lines are business logic:

1. **`commands/repo_config.rs` is 3 627 lines** — the largest file in the workspace. Structure: domain types + diffing (`:39-405`), a **full GitHub implementation** (`load_remote_repo_config_github`, `apply_github`, label CRUD, branch protection + 8 `Gh*` structs, `:467-1305`), a **near-line-for-line GitLab mirror** (`:759-1102`), and only then the 8 thin wrappers (`:1333+`). Its own doc comment flags the `ForgeKind`/`ProviderKind` enum split as debt "that should eventually converge" (`:16-20`). This logic belongs behind the `ForgeProvider` seam in `cli-provider`, which already has `github/` and `gitlab/` module trees.
2. **`ai_background.rs` (1 896 lines)** — the AI-run coordinator (queueing, concurrency cap, worktree lifecycle, event emission) lives in the one crate that can't be tested without Tauri.
3. **`claude-code` / `codex` / `opencode` are structural triplicates** (~1.4K duplicated lines): each has the same `worktrees.rs` (`list_worktrees`, `cleanup_worktree`, `determine_status` — codex and opencode near-verbatim copies down to test names), `detect.rs` (`detect_binary`, `detect_in_repo`, version parsing), and `attribution.rs`. A worktree-cleanup bugfix must land three times.

Also folded in (small storage hardening from the same audit):

4. `crates/storage/src/project_cache.rs:71-73` keys the on-disk snapshot with `DefaultHasher`, whose output is **not stable across Rust releases** — a toolchain bump silently orphans every cached snapshot.
5. `commits_cache.rs` packs strings into TEXT cells (`:67, :124`) with no schema version tag — unlike `layout_cache.rs:16`'s `SCHEMA_VERSION` guard — so a layout change deserializes old rows wrongly instead of invalidating.

## Goal (success criteria)

- `repo_config.rs` < 600 lines: types + 8 thin wrappers; the forge logic lives in `cli-provider` with unit tests that run without Tauri.
- The AI coordinator compiles in a tauri-free crate; app-core's AI surface is command glue + event-sink wiring.
- One shared implementation of AI worktree listing/cleanup/detection; the three provider crates keep only genuinely provider-specific code (session parsing, conversation formats).
- Storage: stable cache key; commits_cache has a schema version.

## Design

**Phase A — repo_config → cli-provider.** Add a `ForgeRepoConfig` trait (or extend `ForgeProvider`) in `forge-provider`: `load_repo_config`, `apply_repo_config(patch)`, label CRUD, `get/set_branch_protection`. Move the `Gh*`/`Glab*` JSON structs and impls into `cli-provider/src/{github,gitlab}/repo_config.rs`. Collapse the twins where the CLI output shapes genuinely align; keep them separate where they don't (don't force a bad unification). The pure-types portion (`Visibility`, `Label`, `RemoteRepoConfig`, `diff_config`) moves to `forge-provider` (contract crate — types only, respects the purity guard). app-core keeps the 8 wrappers + `MutationGuard` wiring.

**Phase B — AI coordinator → `ai-runner` crate.** New library crate `ai-runner` (tauri-free). The coordinator's only Tauri dependencies are event emission and `AppState` access — abstract as `trait RunEventSink { fn emit(&self, event: AiRunEvent); }` implemented in app-core over the Tauri event bridge (same pattern the task-runner presumably uses for streaming). Move queue/concurrency/worktree lifecycle + its tests. `ai_commands.rs`/`commands/ai_background.rs` stay as glue.

**Phase C — `ai-provider-common`.** New crate hosting the shared worktree/detect/attribution helpers, parameterized by a small `ProviderSpec { binary_names: &[&str], session_marker: &str, branch_prefix: &str, … }`. It uses only `std::process::Command` + git CLI — but keep it **out of** the three guarded contract-crate paths (the CI purity grep covers `provider`, `forge-provider`, `ai-provider` only, so a sibling crate is clean). The three provider crates shrink to detection constants + conversation parsing. Mirror of how `github-api`/`gitlab-api` already share `provider::{http_helpers,log_preprocessor}`.

**Phase D — storage hardening (tiny, do anytime).** Replace `DefaultHasher` with a stable hash (`sha2` is already a workspace dep) for the project-cache key — one-time cache orphaning on upgrade, acceptable (it's a recompute). Add `SCHEMA_VERSION` to `commits_cache` mirroring `layout_cache.rs:125`'s mismatch→rebuild.

## Files to touch

Per phase, as above. No frontend changes; no command signature changes (Phase A keeps names/payloads identical).

## Verification

1. Per phase: `cargo test --workspace` + clippy green; the moved logic's tests move with it and run via `cargo test -p cli-provider` / `-p ai-runner` / `-p ai-provider-common` (no Tauri).
2. Phase A manual: load + edit repo settings against a real GitHub and GitLab repo (visibility, labels, protection round-trip).
3. Phase C: the three providers' existing worktree tests (`empty_repo_returns_no_worktrees`, `lists_every_subdir_as_worktree`, …) collapse into `ai-provider-common` and pass.
4. Line-count gate: `wc -l crates/app-core/src/commands/repo_config.rs` < 600; app-core total drops by ≥5K lines.

## Out of scope

- Converging the `ForgeKind`/`ProviderKind` enums (flagged in-code; separate, riskier refactor).
- `theme.rs` (1 811 lines) split — roadmap quick-win, not worth a phase here.
