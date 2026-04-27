/**
 * Hash-route sync for the Repo settings view.
 *
 * Mirrors `settingsRoute.ts`. The hash shape is
 * `#repo-config/<section-id>`. Unknown ids fall back to
 * `DEFAULT_SECTION`. External entry points (future search, future
 * status-bar links) push a `SectionId` into
 * `pendingRepoConfigSection`; `RepoConfigPage` consumes it once on
 * mount and nulls it.
 */

import { writable, get } from "svelte/store";
import { SECTION_IDS, toSectionId, type SectionId } from "$lib/repo-config/sections";

export const DEFAULT_SECTION: SectionId = "general";

export interface RepoConfigRoute {
  section: SectionId;
}

export const repoConfigRoute = writable<RepoConfigRoute>({
  section: DEFAULT_SECTION,
});

/** One-shot deep-link target consumed by `RepoConfigPage` on mount. */
export const pendingRepoConfigSection = writable<SectionId | null>(null);

const HASH_PREFIX = "#repo-config/";

function normalise(raw: string | null | undefined): SectionId {
  return toSectionId(raw ?? null) ?? DEFAULT_SECTION;
}

function parseHash(hash: string): RepoConfigRoute | null {
  if (!hash.startsWith(HASH_PREFIX)) return null;
  const raw = hash.slice(HASH_PREFIX.length);
  return { section: normalise(raw) };
}

function formatHash(route: RepoConfigRoute): string {
  return `${HASH_PREFIX}${route.section}`;
}

/** Imperatively set the active section + URL hash. */
export function setSection(id: string): void {
  const next: RepoConfigRoute = { section: normalise(id) };
  repoConfigRoute.set(next);
  if (typeof window !== "undefined") {
    const serialised = formatHash(next);
    if (window.location.hash !== serialised) {
      window.location.hash = serialised;
    }
  }
}

/** Re-seed the store from `location.hash` if the hash targets this view. */
export function seedFromLocation(): void {
  if (typeof window === "undefined") return;
  const parsed = parseHash(window.location.hash);
  if (parsed) repoConfigRoute.set(parsed);
}

/**
 * Install a `hashchange` listener that syncs the store from the URL.
 * Returns a teardown callback — wire it into an `onMount` return.
 */
export function initRepoConfigRouteSync(): () => void {
  if (typeof window === "undefined") return () => {};
  const handler = () => {
    const parsed = parseHash(window.location.hash);
    if (!parsed) return;
    const current = get(repoConfigRoute);
    if (parsed.section !== current.section) {
      repoConfigRoute.set(parsed);
    }
  };
  window.addEventListener("hashchange", handler);
  seedFromLocation();
  return () => window.removeEventListener("hashchange", handler);
}

/** Re-export the canonical id list for consumers that need it. */
export { SECTION_IDS };
export type { SectionId };
