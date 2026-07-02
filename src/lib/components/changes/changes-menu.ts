/**
 * Pure gating logic for the Changes context menu's batch ("… Selected (N)")
 * section. Kept free of Svelte/DOM so the rules can be unit-tested directly;
 * `ChangesList.svelte` maps the returned action ids to labels + handlers.
 */

/** Batch actions, identified by kind, in the order they appear in the menu. */
export type BatchActionId = "stage" | "unstage" | "discard" | "stash" | "copyPaths";

/**
 * Whether the right-click should surface the batch section: at least two files
 * are checked AND the right-clicked file is one of them. Mirrors the existing
 * "Stash Selected" intent — a lone selection or a right-click on a file outside
 * the selection keeps just the single-file actions.
 */
export function isBatchSelection(selected: Set<string>, filePath: string): boolean {
  return selected.size >= 2 && selected.has(filePath);
}

/**
 * Batch actions offered for a list. The unstaged list can stage/discard its
 * selection; the staged list can unstage it; both can stash and copy paths.
 */
export function batchActionIds(isStaged: boolean): BatchActionId[] {
  return isStaged
    ? ["unstage", "stash", "copyPaths"]
    : ["stage", "discard", "stash", "copyPaths"];
}
