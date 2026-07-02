# Spec 08 — One per-repo state container instead of N hand-rolled caches

**Priority:** P1 · **Effort:** L (1–2 weeks, incremental per-store) · **Branch:** `fix/repo-state-container` (then one branch per migrated store) · **Depends on:** —

## Problem

The app supports multiple repo tabs, but feature stores hold the **active** repo's state in module-level singletons, each independently responsible for surviving tab switches with its own path-keyed cache:

- Singletons: `graph.ts:91-118` (`viewport`, `selectedOid`, `graphOffset`, …), `changes.ts:31-37`, `branches.ts:23-61`, `mr-pr.ts`.
- Hand-rolled per-tab caches, one per store: `graph.ts:153` (`viewportCache: Map<string,…>` + `cacheViewport`/`restoreCachedViewport`/`clearGraphState` at `:156/:164/:401`), `branches.ts:38` (`branchCache`), `mr-pr.ts:524` (`ensuredShas`), `tasks.ts:147/216`, `issues.ts:44`, `aiBackground.ts:124`.
- A third layer: `project-cache.ts` (disk-backed `ProjectSnapshot` per path), whose own comments document the failure mode — *"the previous broken fallback wrote active-project data under inactive project keys"* (`project-cache.ts:154-162`) and the pinned-status cross-tab bug (`:140`).

Every new per-repo feature must remember to wire cache-on-blur / restore-on-focus / clear-on-close in one more place; correctness is convention, and the convention has already broken twice.

## Goal (success criteria)

- Per-repo state lives in **one** container keyed by project path; switching tabs is a pointer swap, not a choreography of save/restore calls across stores.
- A regression-style test exists: open repo A and repo B, mutate B, switch to A → assert A's graph selection, branch list, and changes selection are untouched (the exact class of bug documented in `project-cache.ts`).
- No store owns a private `Map<projectPath, …>` for tab survival once migrated.

## Design

**Container:** a `RepoState` class instance per open project, held in a `Map<projectPath, RepoState>` owned by the projects store. `RepoState` aggregates per-feature slices (`graph`, `changes`, `branches`, `mrPr`, …). Implement slices as Svelte 5 rune classes in `.svelte.ts` files (`$state` fields) — this is also the natural first step of the stores→runes consolidation, giving fine-grained reactivity without `get()` gymnastics (the store layer currently has 235 imperative `get()` calls).

**Facade for compatibility:** existing consumers import `writable`s from `stores/graph.ts` etc. Keep those modules as facades during migration: each exported store proxies the *active* `RepoState` slice (a `derived` over `activeProjectPath` + the container). Components don't change until their store migrates; then they read `repoState.graph.selectedOid` directly.

**Migration order (one branch each, smallest blast radius first):**
1. `branches.ts` (small, has tests) — proves the pattern.
2. `changes.ts` (recently touched by the selection-persistence feature — the new checkbox-selection cache folds into the slice naturally).
3. `graph.ts` (largest; folds `viewportCache` in).
4. `mr-pr.ts` / `issues.ts` / `tasks.ts` / `aiBackground.ts`.
5. Fold `project-cache.ts`'s in-memory role into the container (its *disk* snapshot persistence stays, but reads/writes go through the container so the "wrote under the wrong key" class of bug becomes impossible — a `RepoState` only knows its own path).

**Lifecycle:** create `RepoState` on `open_project`, drop on `close_project` (bounding memory), persist-to-disk hooks where `project-cache.ts` does so today. `mutations.ts` dispatches refreshes to the *event's* project's `RepoState` (it already buffers events for inactive tabs — that buffering can shrink once inactive-tab state is addressable directly).

## Files to touch

- New: `src/lib/stores/repo-state/` (container + slices).
- Migrating: `src/lib/stores/{projects,branches,changes,graph,mr-pr,issues,tasks,aiBackground,project-cache,mutations}.ts` — incrementally.
- Tests: `src/lib/stores/__tests__/` — existing store tests keep passing against the facades; add the two-tab isolation test (Tauri IPC already mocked via `mockInvokeResponse`).

## Verification

1. Per-store branch: existing tests green, plus slice unit tests.
2. The two-tab isolation vitest (the success-criteria test) — add it *first*, against the current facades, so it guards every migration step.
3. Manual: two repos open, background mutations in one (external commits via terminal), tab-switch — selections, scroll position, checkbox state all isolated; memory drops when closing a tab (heap snapshot sanity check).

## Out of scope

- Migrating app-global stores (theme, settings, toasts, shortcuts) — they're correctly global.
- Full runes migration of every store (happens organically as slices migrate; don't chase the long tail).
