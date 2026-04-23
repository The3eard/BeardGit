/**
 * Shared state for the global `CreateBranchDialog`.
 *
 * Rendered once at the layout root (`+page.svelte`). Any entry point
 * that wants to open "New branch" — the Branches header, graph, reflog,
 * or the ⌘⇧B shortcut — calls `openCreateBranchDialog` with an optional
 * source. The dialog reads `open` and `source` from this store.
 *
 * Re-opening while already open is a no-op — the app only has one
 * create-branch dialog at a time. Callers should `close` before
 * changing source.
 */

import { writable, get } from "svelte/store";
import type { InitialSource } from "../components/branches/suggest-local-name";

/** The shape held in the store. */
export interface CreateBranchDialogState {
  open: boolean;
  source: InitialSource;
}

const defaultSource: InitialSource = { kind: "head" };

/** The shared dialog state. `+page.svelte` binds its `open` and `initialSource` from here. */
export const createBranchDialog = writable<CreateBranchDialogState>({
  open: false,
  source: defaultSource,
});

/**
 * Open the dialog with the given source (defaults to HEAD).
 * No-op if already open — callers must close before re-opening.
 */
export function openCreateBranchDialog(source: InitialSource = defaultSource): void {
  if (get(createBranchDialog).open) return;
  createBranchDialog.set({ open: true, source });
}

/** Close the dialog and reset the source to the default (HEAD). */
export function closeCreateBranchDialog(): void {
  createBranchDialog.set({ open: false, source: defaultSource });
}
