/**
 * Shared type used by the per-category `settingsIndex` exports that
 * feed the global search bar in `SettingsPage.svelte`.
 *
 * Every setting the shell should be able to jump to gets a
 * descriptor: the search query matches against `label` +
 * `description`, and clicking a match sets the active category to
 * `category` and (optionally) scrolls to an element marked with
 * `data-setting-anchor="<anchor>"`.
 */

import type { CategoryId } from "$lib/stores/settingsRoute";

/** Descriptor for one row in the global settings search index. */
export interface SettingDescriptor {
  /** Stable unique id — used as a Svelte `{#each}` key. */
  id: string;
  /** Human-readable label shown in the search dropdown. */
  label: string;
  /** Longer description also matched against the search query. */
  description: string;
  /** Which category the setting lives in. */
  category: CategoryId;
  /** Optional anchor string that targets a `data-setting-anchor` element. */
  anchor?: string;
}
