# Spec 03 — Cap every diff/file-content path (the worst IPC offender)

**Priority:** P0 · **Effort:** M (2–3 days) · **Branch:** `perf/diff-size-caps` · **Depends on:** —

## Problem

The 5 MB guard (`MAX_COMMIT_DIFF_BYTES`, `crates/git-engine/src/diff.rs:66`) is applied on exactly **one** path — the single-file CLI diff (`diff.rs:233`). Everything else is uncapped:

- `collect_file_diffs` (`diff.rs:291-401`) materializes **every hunk and every line of every changed file** (a `String` per line) with no size guard; `diff_workdir` (`diff.rs:107`) additionally sets `show_untracked_content(true)` (`diff.rs:113-121`), so a large untracked file's *entire contents* become diff lines. `diff_index` (`diff.rs:126`) and `commit_full_diff` (`diff.rs:259`) are equally unguarded.
- The frontend auto-fetches the whole set on **every mutation**: `src/lib/stores/mutations.ts:98-104` calls `refreshDiffs()` on `head_changed || status_changed`, and `refreshDiffs` (`src/lib/stores/changes.ts:52-56`) invokes `getDiffWorkdir()` + `getDiffIndex()` — full `FileDiff[]` for all changed files, serialized over IPC, on every watcher tick that flips status.
- Asymmetry in file content: `get_file_at_commit` enforces 5 MB + NUL binary detection (`crates/git-engine/src/file_content.rs:86-101`, surfaced as the `too_large`/`binary` tagged enum in `crates/app-core/src/commands/diff.rs:96-108`) — but `get_file_workdir` (`file_content.rs:112-116`) is a bare `read_to_string` and `get_file_index` (`file_content.rs:126-133`) is a whole-blob `from_utf8_lossy`. These feed CodeMirror MergeView (`src/lib/components/editor/DiffEditor.svelte:27-33`) whose diff alignment runs on the webview main thread.

Concrete failure: a repo with a regenerated lockfile or a minified bundle in the working tree → every file save triggers a multi-MB IPC payload and a main-thread MergeView diff. This directly contradicts the performance mandate and hits repos of *any* commit count.

## Goal (success criteria)

- No IPC diff/file-content payload can exceed a fixed budget regardless of working-tree contents.
- A 50 MB untracked file in the working tree: saving other files keeps mutation-refresh IPC under ~100 KB, and opening that file's diff shows the existing "truncated/binary" UI instead of freezing.
- `FileDiff.truncated` (`diff.rs:54-59`, already honored by the UI) is set on every capped path.

## Design

**Phase 1 — backend caps (safe, no UX change):**
- Inside `collect_file_diffs`: per-file byte + line budget (reuse `MAX_COMMIT_DIFF_BYTES` for bytes; add `MAX_FILE_DIFF_LINES`, e.g. 10 000). On breach: stop collecting hunks for that file, set `truncated: true`. Binary short-circuit before line collection (libgit2 flags binary deltas; also NUL-check first chunk, mirroring `file_content.rs:86-101`).
- Consider a whole-response budget for the `FileDiff[]` endpoints (e.g. 20 MB) → mark remaining files `truncated` with hunks empty; UI already renders per-file placeholders.
- `get_file_workdir` / `get_file_index`: apply the identical 5 MB + binary guard and return the same tagged `too_large`/`binary` result the commit path uses (`commands/diff.rs:96-108`). Frontend `DiffEditor`/`StagingDiffEditor` already know how to render those states for the commit path — wire the same branches.

**Phase 2 — fetch less, not just smaller:**
- Change `refreshDiffs` to refresh *statuses* eagerly but fetch a file's diff **lazily on selection** (and re-fetch the currently-open file on mutation). The Changes list needs only statuses + per-file add/del counts, not hunks. Requires a light `get_diff_stats_workdir/index` (name-status + counts — cheap in libgit2, no line materialization) and moving hunk fetch to the existing single-file path.
- Keep the checkbox/hunk-staging flows working: staging operates on selections, and `stage_hunks` takes explicit hunk selections fetched for the open file only.

## Files to touch

- `crates/git-engine/src/diff.rs` (caps in `collect_file_diffs`; stats variant), `crates/git-engine/src/file_content.rs` (workdir/index guards).
- `crates/app-core/src/commands/diff.rs` (tagged results for workdir/index; stats command).
- `src/lib/stores/changes.ts`, `src/lib/stores/mutations.ts`, `src/lib/api/tauri.ts`, `src/lib/types/index.ts`.
- `src/lib/components/editor/DiffEditor.svelte`, `StagingDiffEditor.svelte` (tagged-state branches).

## Verification

1. Unit tests in `git-engine`: oversized file → `truncated: true`, binary file → short-circuit, budget respected (fixture-generated large content, no checked-in blobs).
2. Vitest: `refreshDiffs` fetches stats only; diff hunks requested on selection.
3. Manual: repo with a 50 MB generated file — save loop while watching IPC (add a `tracing` span on payload size); Changes stays instant; opening the big file shows truncated/binary UI.
4. Existing hunk/line-staging tests pass (`cargo test -p git-engine hunk`).

## Out of scope

- Streaming diffs chunk-by-chunk over a Tauri channel (bigger redesign; the caps + lazy fetch remove the pain first).
- Image diff previews (Spec 12 territory / roadmap).
