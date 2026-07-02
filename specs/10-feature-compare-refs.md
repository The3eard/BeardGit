# Spec 10 — Compare view: any ref against any ref

**Priority:** P1 (highest-ROI feature gap) · **Effort:** M (3–5 days) · **Branch:** `feat/compare-refs` · **Depends on:** —

## Problem

"What does my branch add over `main`?" — the pre-PR review motion — has no UI. The backend command **already exists**: `get_diff_between_commits` (`crates/app-core/src/commands/diff.rs`) with its `tauri.ts` wrapper, but its only caller computes a single commit's parent diff (`src/lib/stores/graph.ts:283`). The forge views cover *opened* PRs; nothing covers "before I open one" or ad-hoc `release-x` vs `release-y` archaeology. For a product whose pitch is closing browser tabs, users currently go to GitHub's `/compare` page.

## Goal (success criteria)

- From the graph, the branches list, or the command palette, pick two refs and get: commits unique to each side + file diff list + per-file diff, with both range semantics.
- Three-dot compare (`merge-base(A,B)..B` — "what B adds", PR semantics) is the default; two-dot (direct A↔B tree diff) available as a toggle.
- Output verified identical to `git diff A...B --stat` / `git log A..B --oneline` on fixture repos.

## Design

**Backend (small):**
- `get_merge_base(a, b) -> Option<String>` (git2 `merge_base`, trivial).
- Reuse `get_diff_between_commits(from, to)` for the tree diff: three-dot = `get_diff_between_commits(merge_base, B)`; two-dot = `(A, B)` directly. Verify it resolves arbitrary revspecs (branch names, tags, SHAs) — extend to `revparse_single` if it currently expects raw OIDs.
- `get_commits_between(from, to, limit)` — commits in `from..to` (revwalk push `to`, hide `from`), returning the same commit summary shape the graph detail uses; paginated with `limit` + anchor (100 default) so a 10K-commit divergence doesn't flood IPC.
- All read-only — no `MutationGuard` needed.

**Frontend:**
- New `CompareView` under `src/lib/components/compare/`, lazy-mounted (`LazyComponent`) like other heavy views. Layout mirrors the PR-detail structure users already know: header with two ref pickers (branch/tag/SHA autocomplete — reuse the ref search that feeds the graph filter) + swap button + dot-mode toggle; "N commits ahead / M behind" summary chips; commit list (windowed); file list with the shared status badges; per-file diff via the existing `DiffEditor` path (which already handles `too_large`/`binary` states — and inherits Spec 03's caps when those land).
- Entry points: graph node context menu ("Compare with HEAD…", "Compare with…" pre-filling side A), branches-list context menu ("Compare with current branch"), command palette provider (`src/lib/search/`), and a sidebar item is *not* added (it's a modal-ish task view, reachable contextually — keeps the rail clean).
- State: per-repo (lands naturally as a `RepoState` slice if Spec 08 has started; otherwise a store following the existing per-store cache pattern).

## Files to touch

- `crates/app-core/src/commands/diff.rs` (+ merge-base, commits-between), `crates/git-engine/src/` (revwalk helper).
- `src/lib/api/tauri.ts`, `src/lib/types/index.ts` (three-file contract).
- `src/lib/components/compare/` (new), graph + branches context menus, `src/lib/search/` palette provider.
- Vitest for the store; Rust tests for range semantics.

## Verification

1. Rust tests: fixture with diverged branches — `get_commits_between` and both dot-modes match `git log`/`git diff` output (count + file set + stats).
2. Vitest: store fetch/swap/mode-toggle logic.
3. Manual: compare two long-diverged branches in a big repo — view opens instantly, commit list windows, per-file diffs lazy-load. Playwright snapshot for the view (add to visual suite).

## Out of scope

- Cross-repo / fork compare (forge-API territory).
- "Open PR from compare" button — natural follow-up once the view exists; note it in the roadmap.
