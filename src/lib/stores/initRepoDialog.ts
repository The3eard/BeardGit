import { writable } from "svelte/store";

/**
 * Request to open the InitRepoDialog. Set by `openProjectTab` when the
 * backend reports `not_a_repo`; cleared when the dialog closes.
 *
 * The dialog component subscribes to this; nothing else should.
 */
export const initRepoRequest = writable<{ path: string } | null>(null);

export function requestOpenInitRepoDialog(path: string) {
  initRepoRequest.set({ path });
}

export function closeInitRepoDialog() {
  initRepoRequest.set(null);
}
