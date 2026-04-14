/**
 * Reflog store — manages reflog entries, selection state, and auto-refresh.
 */

import { writable, derived } from "svelte/store";
import type { ReflogEntry } from "../types";
import type { RawDiffContent } from "./graph";
import * as api from "../api/tauri";

/** All loaded reflog entries (most recent first). */
export const reflogEntries = writable<ReflogEntry[]>([]);

/** Currently selected reflog entry index (position in the entries array). */
export const selectedReflogIndex = writable<number | null>(null);

/** The currently selected reflog entry object. */
export const selectedReflogEntry = derived(
  [reflogEntries, selectedReflogIndex],
  ([$entries, $idx]) =>
    $idx !== null && $idx >= 0 && $idx < $entries.length
      ? $entries[$idx]
      : null
);

/** Whether the reflog is currently loading. */
export const reflogLoading = writable(false);

/** File diff content for the selected reflog commit. */
export const reflogFileDiff = writable<RawDiffContent | null>(null);

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

/** Select a reflog entry by its index. */
export function selectReflogEntry(index: number): void {
  selectedReflogIndex.set(index);
}

/** Clear the reflog selection. */
export function clearReflogSelection(): void {
  selectedReflogIndex.set(null);
  reflogFileDiff.set(null);
}

/** Clear all reflog state (e.g., on project switch). */
export function clearReflogState(): void {
  reflogEntries.set([]);
  selectedReflogIndex.set(null);
  reflogFileDiff.set(null);
}
