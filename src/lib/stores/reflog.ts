/**
 * Reflog store — manages reflog entries, selection state, and auto-refresh.
 *
 * Listens to the `repo-changed` Tauri event to auto-refresh the reflog
 * when the repository state changes.
 */

import { writable, derived } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ReflogEntry } from "../types";
import * as api from "../api/tauri";
import { debounce } from "../utils/debounce";

/** All loaded reflog entries (most recent first). */
export const reflogEntries = writable<ReflogEntry[]>([]);

/** Currently selected reflog entry OID (the new OID). */
export const selectedReflogOid = writable<string | null>(null);

/** The currently selected reflog entry object. */
export const selectedReflogEntry = derived(
  [reflogEntries, selectedReflogOid],
  ([$entries, $oid]) =>
    $oid !== null ? $entries.find((e) => e.oid === $oid) ?? null : null
);

/** Whether the reflog is currently loading. */
export const reflogLoading = writable(false);

let unlisten: UnlistenFn | null = null;

/** Load reflog entries from the backend. */
export async function loadReflog(limit = 100): Promise<void> {
  reflogLoading.set(true);
  try {
    const entries = await api.getReflog(limit);
    reflogEntries.set(entries);
  } catch (e) {
    console.error("Failed to load reflog:", e);
    reflogEntries.set([]);
  } finally {
    reflogLoading.set(false);
  }
}

/** Select a reflog entry by its OID. */
export function selectReflogEntry(oid: string): void {
  selectedReflogOid.set(oid);
}

/** Clear the reflog selection. */
export function clearReflogSelection(): void {
  selectedReflogOid.set(null);
}

/** Debounced reflog loader to avoid rapid re-fetches on burst repo-changed events. */
const debouncedLoadReflog = debounce(() => loadReflog(), 300);

/** Start listening for repo changes to auto-refresh the reflog. */
export async function initReflogWatcher(): Promise<void> {
  if (unlisten) return;
  unlisten = await listen("repo-changed", () => {
    debouncedLoadReflog();
  });
}

/** Stop the reflog auto-refresh listener. */
export function cleanupReflogWatcher(): void {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}
