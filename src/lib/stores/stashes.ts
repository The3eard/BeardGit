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
import { runMutation } from "../api/runMutation";
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

export async function doStashPush(message: string | null, paths: string[] | null = null) {
  const count = paths?.length ?? 0;
  await runMutation({
    kind: "stash_push",
    invoke: () => apiStashPush(message, paths),
    successToast: () =>
      message
        ? `Stashed — ${message}`
        : count > 0
          ? `Stashed ${count} file${count === 1 ? "" : "s"}`
          : "Stashed changes",
    failureToastPrefix: "Stash failed",
  });
  // Stashes refresh is driven by the project-mutated event.
}

export async function doStashApply(index: number) {
  await runMutation({
    kind: "stash_apply",
    invoke: () => apiStashApply(index),
    successToast: () => "Stash applied",
    failureToastPrefix: "Stash apply failed",
  });
}

export async function doStashApplyFile(index: number, path: string) {
  await runMutation({
    kind: "stash_apply_file",
    invoke: () => apiStashApplyFile(index, path),
    failureToastPrefix: "Stash apply file failed",
  });
}

export async function doStashPop(index: number) {
  await runMutation({
    kind: "stash_pop",
    invoke: () => apiStashPop(index),
    successToast: () => "Stash popped",
    failureToastPrefix: "Stash pop failed",
  });
}

export async function doStashDrop(index: number) {
  await runMutation({
    kind: "stash_drop",
    invoke: () => apiStashDrop(index),
    successToast: () => "Stash dropped",
    failureToastPrefix: "Stash drop failed",
  });
}

/** Reset all stash selection/detail state. Called on repo switch. */
export function clearStashState() {
  stashes.set([]);
  selectedStashIndex.set(null);
  selectedStashDiff.set(null);
}
