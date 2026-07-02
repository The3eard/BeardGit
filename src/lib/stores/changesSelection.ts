/**
 * Checkbox selection for the working-tree file lists, split by list.
 *
 * The selection lives outside `ChangesList` so it PERSISTS across leaving
 * and re-entering the Changes view (the component unmounts, the store does
 * not). Since spec 08 it lives per-repo in `RepoState.changes` (see
 * `repo-state/ChangesSlice.ts`), so it also survives tab switches per-repo —
 * previously it was a module-global store that `clearChangesState` wiped on
 * every switch, dropping repo A's selection when you passed through repo B.
 * Paths that disappear from a list are pruned by `ChangesList` on refresh
 * rather than clearing the whole set.
 */

import { activeField, getActiveRepoState } from "./repo-state";

export const unstagedSelection = activeField<Set<string>>((rs) => rs.changes.unstagedSelection);
export const stagedSelection = activeField<Set<string>>((rs) => rs.changes.stagedSelection);

/** Reset both selections for the active repo. Called on project switch. */
export function clearChangesSelection(): void {
  getActiveRepoState().changes.clearSelection();
}
