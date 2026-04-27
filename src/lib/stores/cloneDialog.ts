import { writable } from "svelte/store";

/**
 * Visibility flag for the CloneRepoDialog.
 *
 * The dialog is opened from the "+" tab-bar menu and closed via Cancel,
 * Esc, or after a successful clone. The component is the only consumer
 * — nothing else should subscribe to or mutate this store directly.
 */
export const cloneDialogOpen = writable<boolean>(false);

export function openCloneDialog() {
  cloneDialogOpen.set(true);
}

export function closeCloneDialog() {
  cloneDialogOpen.set(false);
}
