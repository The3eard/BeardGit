/**
 * Pure selection/grouping logic for the branch-cleanup dialog (spec 11).
 *
 * Kept free of Svelte/DOM so the pre-check rules and force handling can be
 * unit-tested directly. The dialog component wires these into reactive state.
 */

import type { BranchCleanupCandidate, BranchCleanupList } from "../../types";

/**
 * Whether a candidate requires a force (`git branch -D`) delete: its upstream
 * is gone AND it is not fully merged into the target, so the safe `-d` would
 * refuse it. Merged branches (gone or not) delete safely with `-d`.
 */
export function needsForce(c: BranchCleanupCandidate): boolean {
  return c.upstream_gone && !c.merged;
}

/**
 * Initial checkbox selection: gone branches pre-checked, merged branches
 * unchecked (squash-merge workflows make "merged" incomplete, so we don't
 * oversell it — the user opts in).
 */
export function initialSelection(list: BranchCleanupList): Set<string> {
  return new Set(list.gone.map((c) => c.name));
}

/** All candidates (gone + merged) as one flat list. */
export function allCandidates(list: BranchCleanupList): BranchCleanupCandidate[] {
  return [...list.gone, ...list.merged];
}

/**
 * Names among `selected` that require a force delete — the subset passed as
 * `force` to `deleteBranches`. Merged/gone-merged selections are excluded
 * (they delete safely).
 */
export function selectedForceNames(
  list: BranchCleanupList,
  selected: Set<string>,
): string[] {
  return allCandidates(list)
    .filter((c) => selected.has(c.name) && needsForce(c))
    .map((c) => c.name);
}

/**
 * Whether the delete action may proceed:
 * - at least one branch is selected, and
 * - if any selected branch needs force, the force acknowledgment is checked.
 */
export function canDelete(
  list: BranchCleanupList,
  selected: Set<string>,
  forceAck: boolean,
): boolean {
  if (selected.size === 0) return false;
  const forceNeeded = selectedForceNames(list, selected).length > 0;
  return !forceNeeded || forceAck;
}
