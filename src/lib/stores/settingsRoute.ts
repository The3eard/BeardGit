/**
 * Settings route helper — syncs the active Settings category (and
 * optional inner anchor) to the URL hash, so external links like
 * `#ai` or `#appearance` open the right category without extra glue.
 *
 * The store is the single source of truth for which category is
 * currently active. Writes from either direction (`setCategory()` or
 * the user editing the hash manually / the browser forward/back
 * button) keep the URL and the store in lock-step.
 *
 * Bridges with `pendingSettingsSection`: when the statusbar (or any
 * other writer) sets `pendingSettingsSection` to a known slug we
 * mirror it here so the shell shows the right category on mount. The
 * bridge is one-way and defensive — unknown slugs fall back to
 * `DEFAULT_CATEGORY` rather than propagate broken state.
 *
 * Why a hash and not a query string?
 *   - Svelte's page store + our existing URL handling treat the hash
 *     as a pure client-side signal, which matches Settings (no
 *     server round-trip).
 *   - Matches the convention in the spec snippet
 *     (`?view=settings&cat=appearance#theme`) where the `#theme`
 *     portion is the inner anchor — a shell-only concern.
 */

import { writable, get } from "svelte/store";
import { pendingSettingsSection } from "./navigation";

/** Canonical category slugs used by the shell + category components. */
export const CATEGORY_IDS = [
  "general",
  "appearance",
  "editor",
  "git",
  "ai",
  "integrations",
  "advanced",
] as const;

/** Union of every canonical category slug. */
export type CategoryId = (typeof CATEGORY_IDS)[number];

/** Default category for unknown / missing / empty hashes. */
export const DEFAULT_CATEGORY: CategoryId = "general";

/**
 * Map of legacy section ids (used by `pendingSettingsSection`) to the
 * new category slug they should deep-link to. External writers keep
 * using the old ids; the bridge translates them so we don't ripple
 * rename churn into the statusbar + friends.
 */
const LEGACY_SECTION_MAP: Record<string, CategoryId> = {
  // Connection was split — OAuth / token / CLI auth all live under
  // Integrations now.
  connection: "integrations",
  // "Updates" was its own tab; the new IA folds it into Advanced.
  updates: "advanced",
  // Git config graduates to its own top-level category.
  "git-config": "git",
  // The rest keep their slug.
  ai: "ai",
  integrations: "integrations",
  general: "general",
  appearance: "appearance",
  editor: "editor",
  advanced: "advanced",
};

/** Route state carried in the store + URL. */
export interface SettingsRoute {
  category: CategoryId;
  /** Optional anchor inside the category (for search deep-linking). */
  anchor?: string;
}

/** Internal — coerce an arbitrary string to a valid CategoryId. */
function normaliseCategory(raw: string | null | undefined): CategoryId {
  if (!raw) return DEFAULT_CATEGORY;
  const lowered = raw.toLowerCase();
  if ((CATEGORY_IDS as readonly string[]).includes(lowered)) {
    return lowered as CategoryId;
  }
  // Accept legacy section ids written by statusbar slots.
  const mapped = LEGACY_SECTION_MAP[lowered];
  if (mapped) return mapped;
  return DEFAULT_CATEGORY;
}

/** Parse a hash like `#ai` or `#appearance.theme` into a route. */
function parseHash(hash: string): SettingsRoute {
  const stripped = hash.startsWith("#") ? hash.slice(1) : hash;
  if (!stripped) return { category: DEFAULT_CATEGORY };
  const [rawCat, ...rest] = stripped.split(".");
  const anchor = rest.join(".") || undefined;
  return {
    category: normaliseCategory(rawCat),
    anchor,
  };
}

/** Serialise a route back to the hash form we read. */
function formatHash(route: SettingsRoute): string {
  return route.anchor
    ? `#${route.category}.${route.anchor}`
    : `#${route.category}`;
}

/**
 * The route store. Writers should prefer {@link setCategory} so the
 * URL stays in sync — direct `settingsRoute.set(...)` calls are
 * supported but won't update `location.hash` on their own.
 */
export const settingsRoute = writable<SettingsRoute>({
  category: DEFAULT_CATEGORY,
});

/**
 * Imperatively move the store + URL hash to a new category. Clears
 * any previously-set anchor unless one is supplied.
 */
export function setCategory(id: string, anchor?: string): void {
  const next: SettingsRoute = { category: normaliseCategory(id), anchor };
  settingsRoute.set(next);
  if (typeof window !== "undefined") {
    const serialised = formatHash(next);
    // Avoid firing a hashchange loop when the hash is already correct.
    if (window.location.hash !== serialised) {
      window.location.hash = serialised;
    }
  }
}

/**
 * Re-seed the store from `location.hash`. Call once on page load; the
 * `hashchange` listener set up by {@link initSettingsRouteSync} keeps
 * the store and URL in lock-step after that.
 */
export function seedFromLocation(): void {
  if (typeof window === "undefined") return;
  const parsed = parseHash(window.location.hash);
  settingsRoute.set(parsed);
}

/**
 * Install the browser-side sync between the hash and the store. The
 * returned function tears the listener down — wire it into an
 * `onMount` + its cleanup function.
 */
export function initSettingsRouteSync(): () => void {
  if (typeof window === "undefined") return () => {};
  const handler = () => {
    const parsed = parseHash(window.location.hash);
    const current = get(settingsRoute);
    if (
      parsed.category !== current.category ||
      parsed.anchor !== current.anchor
    ) {
      settingsRoute.set(parsed);
    }
  };
  window.addEventListener("hashchange", handler);
  seedFromLocation();
  return () => window.removeEventListener("hashchange", handler);
}

/**
 * Bridge from the existing `pendingSettingsSection` store. Writers
 * (statusbar slots) set that store to a legacy section id before
 * flipping the sidebar to Settings; we mirror the value into
 * `settingsRoute` and clear the pending store so a later manual
 * navigation doesn't replay the deep link.
 *
 * Returns the unsubscribe function so the caller can tear the
 * subscription down on unmount.
 */
export function bindPendingSectionBridge(): () => void {
  return pendingSettingsSection.subscribe((pending) => {
    if (!pending) return;
    const mapped = normaliseCategory(pending);
    setCategory(mapped);
    pendingSettingsSection.set(null);
  });
}
