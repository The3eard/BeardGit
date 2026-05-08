/**
 * Diff-display settings store.
 *
 * Wraps the persisted `AppConfig` diff flags in Svelte writables so any
 * open diff view re-renders immediately when the user flips a toggle in
 * Settings → General. Each setting round-trips through Tauri commands
 * so the value is preserved across app restarts.
 *
 * Defaults mirror the backend defaults so the very first paint on a
 * cold start — before the `load…` calls return — matches what a fresh
 * install will see.
 */

import { writable, get } from "svelte/store";

import {
  getDiffShowWhitespace as apiGetShowWhitespace,
  setDiffShowWhitespace as apiSetShowWhitespace,
  getDiffLineWrapping as apiGetLineWrapping,
  setDiffLineWrapping as apiSetLineWrapping,
} from "$lib/api/tauri";

/** Reactive flag consumed by `DiffEditor`. Default off. */
export const diffShowWhitespace = writable<boolean>(false);

/**
 * Reactive flag consumed by every diff view (commit, PR/MR, stash, tag,
 * and the staging panel in Changes). When ON, long lines soft-wrap so
 * the whole content stays visible. When OFF, lines render with
 * `white-space: pre` and the surrounding container exposes a horizontal
 * scrollbar. Default ON. Independent from the file-editor's own
 * `line_wrapping` preference.
 */
export const diffLineWrapping = writable<boolean>(true);

/**
 * Hydrate the store from the persisted config. Called once at app
 * startup from `+layout.svelte`. Failures are non-fatal — the default
 * (`false`) stays in place.
 */
export async function loadDiffShowWhitespace(): Promise<void> {
  try {
    const value = await apiGetShowWhitespace();
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
    await apiSetShowWhitespace(value);
  } catch (err) {
    diffShowWhitespace.set(previous);
    throw err;
  }
}

/**
 * Hydrate the diff-line-wrapping store from the persisted config.
 * Called once at app startup. Failures are non-fatal — the default
 * (`true`) stays in place.
 */
export async function loadDiffLineWrapping(): Promise<void> {
  try {
    const value = await apiGetLineWrapping();
    diffLineWrapping.set(value);
  } catch {
    // IPC unavailable (tests, dev) — keep the default.
  }
}

/**
 * Persist a new line-wrapping value, then update the store on success.
 * Reverts on failure to stay in sync with disk.
 */
export async function updateDiffLineWrapping(value: boolean): Promise<void> {
  const previous = get(diffLineWrapping);
  diffLineWrapping.set(value);
  try {
    await apiSetLineWrapping(value);
  } catch (err) {
    diffLineWrapping.set(previous);
    throw err;
  }
}
