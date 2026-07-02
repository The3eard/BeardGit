# Spec 07 — O(limit) deep scroll and indexed viewport queries

**Priority:** P1 · **Effort:** M (3–5 days) · **Branch:** `perf/deep-scroll-viewport` · **Depends on:** Spec 02 bench harness (for the before/after numbers)

## Problem

Two linear scans in the scroll path:

1. **Chunk loading is O(offset).** Scrolling past the 20K cached window calls `load_graph_chunk` → `walk_commits_with_options(offset, limit+1, …)` (`crates/app-core/src/commands/graph.rs:275`), which enumerates the revwalk from the tip and *discards* the first `offset` commits (`crates/git-engine/src/commits.rs:203-204, 264-265`). At offset 80K that's an 80K-commit walk per 300-row page — scroll latency grows linearly with depth. The bench already measures exactly this (`crates/git-engine/benches/walk.rs:31-33`).
2. **`viewport()` rescans everything per query.** `GraphLayout::viewport` filters `lane_segments` and `merge_curves` with full linear scans on every viewport IPC (`crates/graph-builder/src/viewport.rs:52-72`). Merge curves are emitted per cross-lane parent edge (`layout.rs:492-512`) — tens of thousands on a busy 20K layout — rescanned on every scroll step. (The canvas renderer itself is fine: per-frame work is O(visible rows), `src/lib/components/graph/graph-renderer.ts:300-673`.)

## Goal (success criteria)

- Loading a chunk at offset 80K costs the same as at offset 0 (bench-verified: O(limit), not O(offset)).
- `viewport()` query cost is O(log n + overlap), bench-verified on a 20K layout with dense merges.
- Scroll behavior, ordering, and chunk contents are byte-identical to today (same sort options).

## Design

**OID-anchored pagination.** Add an anchored variant: `load_graph_chunk_after(anchor_oid, limit, options)`. Implementation: configure the same revwalk (same sort flags), `push(anchor_oid)` so enumeration starts at the anchor in identical order, skip the anchor itself, take `limit`. Because the walk starts from the anchor rather than the tips, it never re-enumerates the prefix. The frontend tracks the last OID of the previous chunk (the store already holds chunk rows) and passes it; keep the offset API as fallback for random jumps (scrollbar teleport), optionally seeding anchors at intervals from the initial 20K layout so a teleport lands near a known anchor. Caveat to verify in tests: with `TOPOLOGICAL|TIME` sort, walking from the anchor must reproduce the tail of the full walk — assert equality against the offset path in tests across merge-heavy fixtures; if a sort mode can't guarantee it, keep that mode on the offset path.

**Row-indexed viewport.** At `GraphLayout::compute` time, sort `lane_segments` and `merge_curves` by start row and build a small interval index (segments/curves bucketed by fixed row-range pages, or a sorted array + binary search on `start_row` with a max-span bound for the back-scan). `viewport()` then binary-searches the window instead of scanning all. Serde-compatible: derive the index on load (`#[serde(skip)]` + rebuild) so the on-disk cache format doesn't change.

## Files to touch

- `crates/git-engine/src/commits.rs` (anchored walk), `crates/app-core/src/commands/graph.rs` (new command or param), `src/lib/api/tauri.ts` + `src/lib/stores/graph.ts` (pass anchor).
- `crates/graph-builder/src/viewport.rs`, `layout.rs` (index build).
- `crates/git-engine/benches/walk.rs` + graph-builder bench (Spec 02).

## Verification

1. Equality tests: anchored chunk == offset chunk for the same position, across linear/merge-heavy/octopus fixtures and both sort modes.
2. `viewport()` results identical pre/post indexing (property test over random windows).
3. Bench: offset-80K chunk and viewport query, before vs after.
4. Manual: fling-scroll a 100K synthetic repo to the bottom — no progressive slowdown.

## Out of scope

- Raising `MAX_INITIAL_LAYOUT_COMMITS` or making the base layout lazy (separate decision once Spec 04 lands).
- Search-in-graph performance (already windowed via `search_commits` max_count).
