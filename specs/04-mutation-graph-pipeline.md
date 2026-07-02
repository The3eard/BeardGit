# Spec 04 — Make the mutation → graph-refresh pipeline incremental

**Priority:** P0 · **Effort:** L (1–2 weeks, phased) · **Branch:** `perf/graph-mutation-pipeline` (one branch per phase) · **Depends on:** Spec 02's synthetic-repo bench harness for before/after numbers

## Problem

Every ref-moving action — commit, amend, checkout, fetch, pull, push, reset, rebase, and every external commit the watcher sees — pays O(20 000) work, even though most of these change one ref by one commit:

1. **Full layout rebuild per mutation.** `mutations.ts` → `refreshAndReloadGraph()` (`src/lib/stores/graph.ts:248-257`) → `refresh_graph_layout` (`crates/app-core/src/commands/graph.rs:405-447`) → `rebuild_layout_blocking` → `build_fresh_layout`: walks up to `MAX_INITIAL_LAYOUT_COMMITS = 20_000` commits (`crates/app-core/src/commands/graph_cache.rs:206,214`), runs full `GraphLayout::compute` (lane assignment + sync-state tagging + merge-curve pass, `crates/graph-builder/src/layout.rs:228-530`), then **clones the entire layout** and serializes it to disk (`graph_cache.rs:923-932`).
2. **The cache key itself walks 20K commits — even on a hit.** `load_or_build_layout` → `state_fingerprint` does `revwalk.take(MAX_INITIAL_LAYOUT_COMMITS + 1).count()` on every invocation (`graph_cache.rs:173`), so `open_repo`, `switch_project`, and every refresh pay a ~20K walk before the cache can even answer.
3. **Ahead/behind for every branch on every mutation.** `branches()` calls `graph_ahead_behind` per tracking branch (`crates/git-engine/src/repository.rs:132-187`, call at `:168`); `refreshBranches()` fires on every `head_changed || refs_changed`. Cost is O(branches × divergence) stacked on the same events.
4. **`open_repo` walks status twice** — `repo.status()` inside the spawn_blocking, then `file_statuses().len()` again for `change_count` (`crates/app-core/src/commands/repository.rs`).

## Goal (success criteria)

Measured on the Spec 02 synthetic 100K-commit repo (20K layout window), before → after:

- A plain commit's end-to-end refresh (command return → graph repainted) does **no 20K revwalk**: fingerprint check is O(refs), and layout update is O(new rows) for the fast path.
- Cache-hit `open_repo` does not perform the 20K count walk.
- Ahead/behind recomputes only for branches whose tip or upstream moved.
- No behavioral regression: layout output byte-identical to a full rebuild for the same repo state (property test below).

## Design

**Phase A — cheap fingerprint (small, high value).**
Replace the commit-count component of `state_fingerprint` with data that's already O(refs): hash of all ref name→OID pairs + HEAD symbolic target + HEAD tree OID (already included) + presence/mtime of `.git/shallow` (covers the externally-created-shallow case the current comments worry about, `graph_cache.rs:152`). Any history rewrite moves a tip, so ref-hash equality ⇒ same walk output within the window. Bump the layout-cache `SCHEMA_VERSION` so stale keys rebuild once. The mutation-events snapshot already captures a refs picture — reuse its shape where possible.

**Phase B — skip the clone; reuse status.**
Persist the layout by reference (serialize from a borrow) instead of `clone()` (`graph_cache.rs:923-932`). In `open_repo`, derive `change_count` from the `repo.status()` result already computed instead of a second `file_statuses()` walk.

**Phase C — incremental prepend for the common case.**
Detect the "simple advance" shape from the mutation diff: exactly one ref moved, `old_tip` is an ancestor of `new_tip` (first-parent chain), no other refs changed. Then: walk only `new_tip..old_tip` (the new commits, typically 1), prepend rows to the cached `GraphLayout`, extend the affected lane segment, shift row indices (or store a row-offset base to avoid O(n) renumbering), and update sync-state tags for that lane only. Anything that doesn't match the shape falls back to the existing full rebuild — correctness never depends on the fast path. This is the hard phase; land it behind a debug-assertable flag that cross-checks against a full rebuild in dev builds.

**Phase D — ahead/behind cache.**
Cache `(branch_tip, upstream_tip) → (ahead, behind)` in-memory per repo; on refresh, recompute only entries whose key changed (the mutation snapshot knows which refs moved). Optionally make the UI request ahead/behind lazily for visible rows only — but the keyed cache alone removes the per-mutation stack.

## Files to touch

- `crates/app-core/src/commands/graph_cache.rs` (fingerprint, clone, incremental entry point), `commands/graph.rs`, `commands/repository.rs`.
- `crates/graph-builder/src/layout.rs` (prepend support on `GraphLayout`), `crates/storage` layout-cache schema bump.
- `crates/git-engine/src/repository.rs` (ahead/behind caching hook) or a small cache in app-core `AppState`.
- No frontend changes required (same commands, same payloads).

## Verification

1. **Property test:** for a randomized synthetic repo, apply N random "commit on branch" mutations; after each, incremental layout == full rebuild (structural equality). This is the gate for Phase C.
2. Bench deltas (Spec 02 harness): fingerprint-on-hit, commit-refresh end-to-end, branches() with 500 tracking branches.
3. Existing graph tests (`cargo test -p graph-builder -p app-core graph`) green; layout-cache invalidation tests still pass (reset --soft case in `graph_cache.rs` comments).
4. Manual: 100K-repo commit feels instant; external commit via terminal reflects without jank.

## Out of scope

- Raising the 20K window or redesigning chunked deep scroll (Spec 07).
- Incremental handling of rebases/force-pushes (they legitimately full-rebuild).
