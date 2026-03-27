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

export const stashes = writable<StashEntry[]>([]);
export const selectedStashIndex = writable<number | null>(null);
export const selectedStashDiff = writable<FileDiff[] | null>(null);

/** Refresh the stash entry list. Clears selection if the selected stash was dropped. */
export async function loadStashes() {
  const entries = await apiStashEntries();
  stashes.set(entries);

  // If selected stash no longer exists, clear selection
  const selected = get(selectedStashIndex);
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
