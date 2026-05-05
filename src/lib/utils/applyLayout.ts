/**
 * Pure helper that composes the Navigation sidebar's render order from:
 *  1. the user's persisted order (may be empty, stale, or contain unknown ids),
 *  2. the canonical `DEFAULT_ORDER` (source of truth for "what items exist now"),
 *  3. the set of hidden ids.
 *
 * The `...DEFAULT_ORDER.filter(...)` tail merge is load-bearing: when a
 * future release ships a new nav item, existing user layouts pick it up
 * automatically at the end instead of silently dropping it.
 *
 * Kept pure (no Svelte store access, no DOM) so it can be unit-tested
 * and called from both the render path and the last-visible guardrail.
 */

/** Minimal shape of a Sidebar nav item that this helper cares about. */
export interface SidebarNavItem {
  id: string;
  label: string;
  icon: string;
}

/**
 * Canonical Navigation item order.
 *
 * **Keep in lockstep with `default_sidebar_nav_order()` in
 * `crates/storage/src/config.rs`.** When you add a new nav item, append
 * its id here and in the Rust default.
 */
export const DEFAULT_ORDER: readonly string[] = [
  "graph",
  "changes",
  "editor",
  "branches",
  "tags",
  "stashes",
  "worktrees",
  "reflog",
  "bisect",
  "submodules",
  "ai-config",
  "ai-sessions",
  "requests",
];

/**
 * Compose the render list from the registered items, saved order, and
 * hidden ids. Returns items in final display order with hidden ids
 * removed.
 *
 * Unknown ids in `order` (e.g. from a renamed nav item) are silently
 * skipped. New ids in `items` not mentioned in `order` are appended at
 * the end in `DEFAULT_ORDER` relative order.
 */
export function applyLayout(
  items: SidebarNavItem[],
  order: readonly string[],
  hidden: readonly string[],
): SidebarNavItem[] {
  const byId = new Map(items.map((i) => [i.id, i]));
  const seen = new Set<string>();
  const composed: string[] = [];

  for (const id of order) {
    if (!seen.has(id) && byId.has(id)) {
      composed.push(id);
      seen.add(id);
    }
  }
  for (const id of DEFAULT_ORDER) {
    if (!seen.has(id) && byId.has(id)) {
      composed.push(id);
      seen.add(id);
    }
  }
  // Anything in `items` but not in DEFAULT_ORDER (shouldn't happen in
  // practice) goes last, preserving registration order.
  for (const item of items) {
    if (!seen.has(item.id)) {
      composed.push(item.id);
      seen.add(item.id);
    }
  }

  const hiddenSet = new Set(hidden);
  return composed
    .map((id) => byId.get(id))
    .filter((x): x is SidebarNavItem => !!x)
    .filter((i) => !hiddenSet.has(i.id));
}
