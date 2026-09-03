/**
 * Session memory for per-view UI state.
 *
 * Every section of the app is one `{#if activeView === …}` branch in
 * `routes/+page.svelte`, so leaving a section unmounts its component and
 * any `$state` it held — the filter you typed, the folder you collapsed,
 * the comment you were drafting, the split you dragged. Users read that as
 * the app forgetting where they were.
 *
 * `remembered(key, initial)` is a writable whose value outlives the
 * component: the first call for a key creates it, later calls return the
 * same store. Bind to it exactly where a `$state` used to be. RAM only, on
 * purpose — it has to survive a section switch, not a relaunch.
 *
 * Two kinds of key:
 *
 * - **Global** (`"branches.splitWidth"`): layout the user tunes once,
 *   independent of which repository is open.
 * - **Per repository** via {@link scoped}: filters, selections, drafts —
 *   anything that names a path, ref or item in one repo. The scope is the
 *   active `RepoState` path, so switching project tabs swaps the memory
 *   along with the rest of the repo's state, and closing a tab drops it
 *   ({@link forgetScope}).
 *
 * Not for anything that already has a home: repo data belongs in a
 * `RepoState` slice, persisted preferences in `AppConfig`. This is for the
 * long tail of component-local UI state that had no store because no one
 * needed it to survive.
 */
import { get, writable, type Writable } from "svelte/store";
import { activeRepoPath } from "./repo-state";

const cells = new Map<string, Writable<unknown>>();

/** Separator between the repo path and the view key in a scoped key. */
const SCOPE_SEP = "::";

/**
 * The writable remembered under `key`, created with `initial` on first use.
 *
 * `initial` is ignored once the cell exists — the remembered value wins,
 * which is the point.
 */
export function remembered<T>(key: string, initial: T): Writable<T> {
  let cell = cells.get(key);
  if (!cell) {
    cell = writable<T>(initial);
    cells.set(key, cell);
  }
  return cell as Writable<T>;
}

/**
 * Prefix `key` with the active repository path.
 *
 * Read at call time: components call it during init, and a project switch
 * forces the graph view and remounts everything, so the scope a component
 * sees is the repo it renders. With no active repo the key is global.
 */
export function scoped(key: string): string {
  const repo = get(activeRepoPath);
  return repo ? `${repo}${SCOPE_SEP}${key}` : key;
}

/** Drop every cell scoped to `repoPath` — the project tab was closed. */
export function forgetScope(repoPath: string): void {
  const prefix = `${repoPath}${SCOPE_SEP}`;
  for (const key of [...cells.keys()]) {
    if (key.startsWith(prefix)) cells.delete(key);
  }
}

/** Test helper — forget everything. */
export function __resetViewMemory(): void {
  cells.clear();
}
