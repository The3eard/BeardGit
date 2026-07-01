/**
 * Checkbox selection for the working-tree file lists, split by list.
 *
 * The selection lives outside `ChangesList` so it PERSISTS across leaving
 * and re-entering the Changes view (the component unmounts, the store does
 * not). It is reset on project switch via {@link clearChangesSelection}
 * (wired into `clearChangesState`). Paths that disappear from a list are
 * pruned by `ChangesList` on refresh rather than clearing the whole set.
 */

import { writable } from "svelte/store";

export const unstagedSelection = writable<Set<string>>(new Set());
export const stagedSelection = writable<Set<string>>(new Set());

/** Reset both selections. Called on project switch. */
export function clearChangesSelection(): void {
  unstagedSelection.set(new Set());
  stagedSelection.set(new Set());
}
