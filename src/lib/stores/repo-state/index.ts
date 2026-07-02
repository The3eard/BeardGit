/**
 * RepoState container — one per-repo state object keyed by project path.
 *
 * The app has multiple repo tabs, but feature stores historically held the
 * *active* repo's state in module-level singletons, each responsible for
 * surviving tab switches with its own path-keyed cache (branchCache,
 * viewportCache, …). Correctness was convention, and the convention broke
 * (see the isolation bug documented in `project-cache.ts` + spec 08).
 *
 * Here, per-repo state lives in **one** container keyed by path. Switching
 * tabs is a pointer swap (`setActiveRepoPath`), not a choreography of
 * save/restore calls across stores. Feature stores (`branches.ts`,
 * `changes.ts`, …) become thin facades that proxy the *active* RepoState's
 * slice via {@link activeField}, so components don't change.
 *
 * ── Runes vs writable fallback ────────────────────────────────────────
 * The spec suggests slices as Svelte 5 `$state` rune classes in `.svelte.ts`
 * files. We instead use plain classes whose fields are Svelte `writable`s
 * because the facade MUST expose the `Writable` store contract (components
 * and the 30+ existing store tests do `$store`, `store.set()`, `get(store)`
 * with synchronous semantics). Bridging `$state` back to that contract needs
 * per-field `toStore`/`$effect.root` wrappers whose flush timing under
 * jsdom+vitest doesn't match `writable`'s synchronous `get()`. writable-backed
 * slices give per-repo isolation trivially (each RepoState owns distinct
 * writable instances) and stay 100% testable. The runes consolidation can
 * happen later, per the spec's "don't chase the long tail".
 */

import { writable, derived, type Writable } from "svelte/store";
import { BranchesSlice } from "./BranchesSlice";
import { ChangesSlice } from "./ChangesSlice";

export { BranchesSlice } from "./BranchesSlice";
export { ChangesSlice } from "./ChangesSlice";

/** All per-repo state for a single open project. */
export class RepoState {
  readonly path: string;
  readonly branches = new BranchesSlice();
  readonly changes = new ChangesSlice();

  constructor(path: string) {
    this.path = path;
  }
}

/**
 * The container. Entries are created on `open_project` and dropped on
 * `close_project` (see `projects.ts`), bounding memory to open tabs.
 */
const container = new Map<string, RepoState>();

/**
 * Backing state for the "no active repo" case (terminal tab, welcome
 * screen, or a test that never opened a project). The facades always have
 * a live slice to read/write, so store tests that just `set()`/`get()`
 * behave exactly like the old module-level writables.
 */
const detachedRepoState = new RepoState("");

/** Path of the currently active project, or `null` when detached. */
export const activeRepoPath = writable<string | null>(null);

/** Bumped on create/drop so `activeRepoState` recomputes if the active
 *  path's entry appears or disappears out of order. */
const containerVersion = writable(0);

/** The active RepoState — the container entry for the active path, or the
 *  detached fallback. Never null, so facades never need a null branch. */
export const activeRepoState = derived(
  [activeRepoPath, containerVersion],
  ([$path]) => (($path && container.get($path)) || detachedRepoState),
);

// Mirror the active RepoState into a plain variable so facade `set`/`update`
// (and the `clear*` helpers) resolve it in O(1) without a get()-subscribe
// round-trip on every write. Kept in sync by the same source of truth.
let current: RepoState = detachedRepoState;
activeRepoState.subscribe((rs) => {
  current = rs;
});

/** The active RepoState, resolved synchronously. */
export function getActiveRepoState(): RepoState {
  return current;
}

/** Create (or return the existing) RepoState for `path`. Idempotent. */
export function createRepoState(path: string): RepoState {
  let rs = container.get(path);
  if (!rs) {
    rs = new RepoState(path);
    container.set(path, rs);
    containerVersion.update((n) => n + 1);
  }
  return rs;
}

/** Drop the RepoState for `path`, freeing its per-repo state. */
export function dropRepoState(path: string): void {
  if (container.delete(path)) {
    containerVersion.update((n) => n + 1);
  }
}

/** Point the facades at `path`'s slice (or the detached fallback). */
export function setActiveRepoPath(path: string | null): void {
  activeRepoPath.set(path);
}

/**
 * Build a `Writable<T>` facade over a field of the *active* RepoState.
 * Reads reflect the active slice and re-emit on tab switch; writes route to
 * the active slice. This is what lets `stores/branches.ts` etc. keep their
 * public store exports while the data lives per-repo.
 */
export function activeField<T>(select: (rs: RepoState) => Writable<T>): Writable<T> {
  const { subscribe } = derived<typeof activeRepoState, T>(
    activeRepoState,
    ($rs, set) => select($rs).subscribe(set),
  );
  return {
    subscribe,
    set: (value: T) => select(current).set(value),
    update: (updater: (value: T) => T) => select(current).update(updater),
  };
}

/**
 * Test seam — reset the container, active path, and detached fallback so
 * cases don't leak across each other. Not used in production.
 */
export function __resetRepoStateForTests(): void {
  container.clear();
  detachedRepoState.branches.clear();
  detachedRepoState.branches.list.set([]);
  detachedRepoState.changes.clear();
  detachedRepoState.changes.commitMessage.set("");
  activeRepoPath.set(null);
  containerVersion.set(0);
}
