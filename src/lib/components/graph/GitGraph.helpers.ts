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
