/**
 * Remotes store — cached list of configured remotes for the active
 * repository.
 *
 * The store is populated by `refreshRemotes()` on demand and kept in
 * sync via the `remotes_changed` bit on `project-mutated` events (the
 * mutations dispatcher calls `refreshRemotes()` when that flag fires).
 *
 * Consumers in this slice: `BranchList.svelte` (push / force-push
 * submenu fan-out).
 */

import { writable, derived } from "svelte/store";
import { getRemotes } from "../api/tauri";
import type { RemoteInfo } from "../types";

export const remotes = writable<RemoteInfo[]>([]);

/** Just the names, in the order reported by `git remote`. */
export const remoteNames = derived(remotes, ($r) => $r.map((r) => r.name));

/**
 * Re-fetch the remote list from the backend.
 *
 * Errors are swallowed (store falls back to the last known list /
 * empty) — remote listing is a best-effort UX convenience, never
 * critical path.
 */
export async function refreshRemotes(): Promise<void> {
  try {
    const list = await getRemotes();
    // Defensive: a misbehaving backend (or a test harness without a
    // get_remotes mock) must not poison the store — a non-array here
    // would crash every derived that does `$remotes.find(...)`.
    remotes.set(Array.isArray(list) ? list : []);
  } catch {
    // Leave the previous value in place; UI will render with stale data.
  }
}

/** Test helper — reset store state between cases. */
export function __resetRemotesForTests(): void {
  remotes.set([]);
}
