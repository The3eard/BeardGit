/**
 * Stashes store — list, select, and mutate git stash entries.
 *
 * Selection uses a last-wins guard to avoid stale diffs when
 * the user clicks rapidly between entries.
 */

import { writable, get } from "svelte/store";
import type { StashEntry, FileDiff } from "../types";
import {
  stashEntries as apiStashEntries,
  stashShowParsed as apiStashShowParsed,
  stashPush as apiStashPush,
  stashApply as apiStashApply,
  stashApplyFile as apiStashApplyFile,
  stashPop as apiStashPop,
  stashDrop as apiStashDrop,
} from "../api/tauri";
import { fetchIntoStore } from "../utils/store-helpers";

export const stashes = writable<StashEntry[]>([]);
export const stashesLoading = writable(false);
export const selectedStashIndex = writable<number | null>(null);
export const selectedStashDiff = writable<FileDiff[] | null>(null);

/**
 * Re-fetch the stash list for the active project. Thin wrapper over
 * {@link loadStashes}; exists so the mutation dispatcher can call a
 * named refresher that matches the other stores' naming pattern.
 */
export async function refreshStashes(): Promise<void> {
  await loadStashes();
}

/** Refresh the stash entry list. Clears selection if the selected stash was dropped. */
export async function loadStashes() {
  await fetchIntoStore(stashes, stashesLoading, apiStashEntries, []);

  // If selected stash no longer exists, clear selection
  const selected = get(selectedStashIndex);
  const entries = get(stashes);
  if (selected !== null && !entries.some((e) => e.index === selected)) {
    selectedStashIndex.set(null);
    selectedStashDiff.set(null);
  }
}

/** Select a stash entry and fetch its parsed diff. Uses last-wins guard. */
export async function selectStash(index: number) {
  selectedStashIndex.set(index);
  selectedStashDiff.set(null);
  const diffs = await apiStashShowParsed(index);
  if (get(selectedStashIndex) !== index) return;
  selectedStashDiff.set(diffs);
}

export async function doStashPush(message: string | null) {
  await apiStashPush(message);
  await loadStashes();
}

export async function doStashApply(index: number) {
  await apiStashApply(index);
  await loadStashes();
}

export async function doStashApplyFile(index: number, path: string) {
  await apiStashApplyFile(index, path);
}

export async function doStashPop(index: number) {
  await apiStashPop(index);
  await loadStashes();
}

export async function doStashDrop(index: number) {
  await apiStashDrop(index);
  await loadStashes();
}

/** Reset all stash selection/detail state. Called on repo switch. */
export function clearStashState() {
  stashes.set([]);
  selectedStashIndex.set(null);
  selectedStashDiff.set(null);
}
