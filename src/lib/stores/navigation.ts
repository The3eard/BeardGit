/**
 * Navigation store — tracks the currently-active sidebar view.
 *
 * Extracted out of `+page.svelte` so that cross-cutting concerns (e.g. the
 * `<Xrefs>` component) can programmatically switch views without lifting
 * local state everywhere.
 *
 * The identifiers match the `id` field on Sidebar nav items
 * (`"graph"`, `"changes"`, `"merge-requests"`, `"issues"`, …).
 */

import { writable, get } from "svelte/store";
import { hasActiveProvider } from "./provider";

/** Currently active sidebar view identifier. */
export const activeViewStore = writable<string>("graph");

/**
 * Views that are only meaningful when a forge provider (GitHub / GitLab)
 * is connected. When the provider disconnects while the user is on one
 * of these views, `installProviderDisconnectReroute()` flips the active
 * view back to `graph` so they don't stare at a blank panel.
 */
export const PROVIDER_VIEWS: readonly string[] = [
  "pipelines",
  "issues",
  "merge-requests",
  "releases",
  "repo-config",
];

/**
 * Subscribe to `hasActiveProvider`; when it goes falsy and the current
 * view is provider-scoped, reroute to `graph`. Returns an unsubscribe
 * function so callers (app-shell `onMount`) can tear down on destroy.
 */
export function installProviderDisconnectReroute(): () => void {
  return hasActiveProvider.subscribe((active) => {
    if (!active && PROVIDER_VIEWS.includes(get(activeViewStore))) {
      activeViewStore.set("graph");
    }
  });
}

/**
 * Deep-link target for the Settings view.
 *
 * Writers (e.g. the statusbar's AI / Forge / Version slots) set this to a
 * section id (`"ai" | "connection" | "appearance" | "git-config" |
 * "updates"`) _before_ flipping `activeViewStore` to `"settings"`.
 * `SettingsPage.svelte` subscribes and mirrors the value into its local
 * active-section state, then clears it by writing `null` back so the next
 * manual navigation into Settings opens the default "Connection" tab
 * instead of re-deep-linking to whatever the previous click targeted.
 *
 * `null` means "no override — use the current active section (or the
 * default)".
 */
export const pendingSettingsSection = writable<string | null>(null);
