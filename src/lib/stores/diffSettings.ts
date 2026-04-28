/**
 * Diff-display settings store.
 *
 * Wraps the persisted `AppConfig::diff_show_whitespace` flag in a
 * Svelte writable so any open `DiffEditor` re-renders immediately when
 * the user flips the toggle in Settings → General. The setting itself
 * round-trips through Tauri commands so the value is preserved across
 * app restarts.
 *
 * The default mirrors the backend default (`false`) so the very first
 * paint on a cold start — before `loadDiffShowWhitespace` returns —
 * matches what a fresh install will see.
 */

import { writable, get } from "svelte/store";

import {
  getDiffShowWhitespace as apiGet,
  setDiffShowWhitespace as apiSet,
} from "$lib/api/tauri";

/** Reactive flag consumed by `DiffEditor`. Default off. */
export const diffShowWhitespace = writable<boolean>(false);

/**
 * Hydrate the store from the persisted config. Called once at app
 * startup from `+layout.svelte`. Failures are non-fatal — the default
 * (`false`) stays in place.
 */
export async function loadDiffShowWhitespace(): Promise<void> {
  try {
    const value = await apiGet();
    diffShowWhitespace.set(value);
  } catch {
    // IPC unavailable (tests, dev) — keep the default.
  }
}

/**
 * Persist a new value, then update the store on success. Reverts the
 * store on failure so the UI stays in sync with what's actually on
 * disk.
 */
export async function updateDiffShowWhitespace(value: boolean): Promise<void> {
  const previous = get(diffShowWhitespace);
  diffShowWhitespace.set(value);
  try {
    await apiSet(value);
  } catch (err) {
    diffShowWhitespace.set(previous);
    throw err;
  }
}
