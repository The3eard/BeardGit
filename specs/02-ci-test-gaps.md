# Spec 02 — CI actually runs the tests it claims to run

**Priority:** P0 (quick win) · **Effort:** S (1 day) · **Branch:** `chore/ci-test-gaps` · **Depends on:** —

## Problem

1. **Vitest never runs in CI.** `.github/workflows/ci.yml` runs `svelte-check`, stylelint, eslint, and `cargo test` — but no `npm test`. The repo has **157 frontend test files** that only run on a developer machine. Root `CLAUDE.md` claims "CI runs all of the above", which is currently false.
2. **Rust tests run only on Ubuntu** for a three-platform desktop app. The worst shipped bug in the project's history (the `2026.x` NSIS updater loop, see CHANGELOG 26.6.2) was Windows-only. Path handling, PTY (`portable-pty`), watcher (FSEvents/inotify/ReadDirectoryChangesW), and the git CLI shelling all have platform-specific behavior with zero platform coverage.
3. **Benchmarks are inert.** The only bench (`crates/git-engine/benches/walk.rs:19-25`) exits early unless `BEARDGIT_BENCH_REPO` is set — which no workflow sets. The product promise ("fast at 100K commits") has no automated guard; the O(offset) deep-scroll cost (Spec 07) would sail through CI today.

## Goal (success criteria)

- A PR that breaks a Vitest test fails CI.
- `cargo test --workspace` runs green on ubuntu + windows + macos in CI (at minimum on pushes to `beta`/`main`).
- At least one Criterion bench over a **synthetic** repo (no env-var-gated external fixture) runs on a schedule with a visible trend, covering a Tier-1 hot path (graph layout compute or chunk walk).
- Root `CLAUDE.md` statement about CI matches reality.

## Design

**Job: frontend-tests** (ubuntu) — mirror the existing `frontend-checks` steps (checkout, node 22, `npm ci`, paraglide compile), then `npm test` (vitest run). Keep it a separate job so lint/type failures and test failures are distinguishable at a glance.

**Rust matrix** — extend `rust-checks` with `strategy.matrix.os: [ubuntu-latest, windows-latest, macos-latest]`:
- The Tauri build script needs `node_modules` + CLI sidecars per-target — reuse the existing `scripts/download-cli-binaries.js --target <triple>` step with per-OS triples (build.yml already has this matrix wiring to copy from).
- Cost control: run clippy/fmt only on ubuntu; run the full matrix on push to `beta`/`main` and keep `feature/**` pushes ubuntu-only (`if:` conditions), so day-to-day iteration stays cheap.
- Expect a first round of genuinely-failing platform tests (that's the point); fix or `#[cfg]`-gate deliberately.

**Perf smoke** — new workflow `perf.yml` (weekly schedule + manual dispatch):
- A test-support helper builds a synthetic repo (N commits, M branches, merge topology) with git2 in a temp dir — no external fixture. Grow `git-engine/benches/` to cover: `walk_commits` at offset 0/10K/80K, `GraphLayout::compute` at 20K, `viewport()` slicing.
- Start with trend visibility (upload Criterion output as artifact); add a hard threshold gate only once baselines are stable. Don't block PRs on runner noise.

**Docs** — update root `CLAUDE.md` CI sentence.

## Files to touch

- `.github/workflows/ci.yml` — new job + matrix.
- `.github/workflows/perf.yml` — new.
- `crates/git-engine/benches/walk.rs` + new bench targets; a `synthetic_repo` helper in `git-engine`'s test support.
- `CLAUDE.md`.

## Verification

1. Push a branch with a deliberately broken vitest test → CI red; revert → green.
2. Matrix run green on all three OSes (after fixing whatever it flushes out — track those as their own fixes).
3. `gh workflow run perf.yml` produces Criterion artifacts.

## Out of scope

- Playwright visual tests in CI (font/GPU rendering makes cross-machine snapshots flaky; revisit with a containerized runner + committed Linux baselines as a separate effort).
- Coverage reporting.
