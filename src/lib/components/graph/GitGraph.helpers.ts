/**
 * Pure helpers extracted from `GitGraph.svelte` so they can be unit
 * tested without mounting the canvas component.
 */
import type { InitialSource } from "../branches/suggest-local-name";

/**
 * Wrap a commit OID into the `CreateBranchDialog`'s `InitialSource` shape.
 *
 * Used by the graph's "Create branch at {sha}..." context-menu action to
 * hand off to the shared dialog instead of calling `window.prompt`.
 */
export function buildCreateBranchSource(oid: string): InitialSource {
  return { kind: "commit", oid };
}

/**
 * Decide how the graph should react when the active repo's `RepoInfo`
 * changes. `repoInfo` gets a fresh object on BOTH a real repo switch and a
 * mutation-driven `refreshRepoInfo()`, so keying off object identity yanked
 * the scroll back to row 0 on every background change. Key off the repo
 * *path* instead:
 *
 * - `"none"`    → same repo (mutation refresh, lane-budget tweak) or no repo:
 *                 leave the viewport alone so the mutation reconcile path
 *                 keeps the user's scroll position.
 * - `"restore"` → switched to a repo whose viewport is already in its slice
 *                 (a revisited tab): keep the restored offset + selection from
 *                 the RepoState pointer swap.
 * - `"reset"`   → switched to a repo with no restored viewport (cold start, or
 *                 a genuinely new repo in this tab): load from the top.
 */
export function graphRepoChangeAction(
  prevPath: string | null,
  nextPath: string | null,
  hasViewport: boolean,
): "none" | "restore" | "reset" {
  if (nextPath === null || nextPath === prevPath) return "none";
  return hasViewport ? "restore" : "reset";
}
