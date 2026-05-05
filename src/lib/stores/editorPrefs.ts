/**
 * Editor-preferences store.
 *
 * Mirrors `AppConfig::editor_preferences` in a Svelte writable so the
 * (PR3) in-app editor can react to toggle changes without re-fetching
 * from the backend on every keystroke. The store is `null` until the
 * first `loadEditorPrefs()` settles — components must guard for that
 * case (a loading skeleton, disabled controls, etc.) so we never paint
 * with stale defaults that disagree with what's persisted.
 *
 * Updates are optimistic: `updateEditorPrefs` patches the store first,
 * then persists. On save failure it reverts to the prior snapshot and
 * rethrows so the caller can re-sync its input state if needed.
 */

import { writable, get } from "svelte/store";

import {
  getEditorPreferences as apiGet,
  setEditorPreferences as apiSet,
} from "$lib/api/tauri";
import type { EditorPreferences } from "$lib/types";

/**
 * Reactive editor-preferences mirror. `null` until `loadEditorPrefs`
 * resolves — consumers must handle the loading state explicitly.
 */
export const editorPrefs = writable<EditorPreferences | null>(null);

/**
 * Hydrate the store from disk. Call once on app boot from
 * `+page.svelte` `onMount`. Failures are non-fatal — the store stays
 * `null` so consumers continue rendering their loading state until a
 * later successful load.
 */
export async function loadEditorPrefs(): Promise<void> {
  try {
    const value = await apiGet();
    editorPrefs.set(value);
  } catch {
    // IPC unavailable (tests, dev) — leave the store null; the next
    // successful boot will hydrate it.
  }
}

/**
 * Patch a subset of the preferences and persist. The store is updated
 * optimistically; on save failure it reverts and rethrows so the calling
 * input can show an error / re-sync its state.
 *
 * Throws synchronously when the store hasn't been hydrated yet — callers
 * are expected to disable controls until `editorPrefs` is non-null, so
 * this should never fire in normal flows.
 */
export async function updateEditorPrefs(
  patch: Partial<EditorPreferences>,
): Promise<void> {
  const previous = get(editorPrefs);
  if (!previous) {
    throw new Error("editorPrefs not loaded yet");
  }
  const next: EditorPreferences = { ...previous, ...patch };
  editorPrefs.set(next);
  try {
    await apiSet(next);
  } catch (err) {
    editorPrefs.set(previous);
    throw err;
  }
}
