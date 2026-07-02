/**
 * Unit tests for the branch-cleanup dialog's pure selection logic (spec 11):
 * pre-check rules, force detection, and the delete gate.
 */

import { describe, it, expect } from "vitest";
import {
  needsForce,
  initialSelection,
  selectedForceNames,
  canDelete,
} from "../branch-cleanup";
import type { BranchCleanupCandidate, BranchCleanupList } from "../../../types";

function candidate(
  name: string,
  overrides: Partial<BranchCleanupCandidate> = {},
): BranchCleanupCandidate {
  return {
    name,
    tip_oid: name.padEnd(40, "0"),
    last_commit_time: 1000,
    ahead: 0,
    upstream_gone: false,
    merged: false,
    ...overrides,
  };
}

/**
 * A list with:
 * - `gone-unmerged` — gone + not merged → needs force
 * - `gone-merged`   — gone + merged     → safe delete
 * - `merged-only`   — merged, not gone  → safe delete
 */
function makeList(): BranchCleanupList {
  return {
    target: "main",
    gone: [
      candidate("gone-unmerged", { upstream_gone: true, merged: false, ahead: 3 }),
      candidate("gone-merged", { upstream_gone: true, merged: true, ahead: 0 }),
    ],
    merged: [candidate("merged-only", { upstream_gone: false, merged: true, ahead: 0 })],
  };
}

describe("branch-cleanup selection logic", () => {
  it("needsForce is true only for gone-and-unmerged branches", () => {
    expect(needsForce(candidate("x", { upstream_gone: true, merged: false }))).toBe(true);
    expect(needsForce(candidate("x", { upstream_gone: true, merged: true }))).toBe(false);
    expect(needsForce(candidate("x", { upstream_gone: false, merged: true }))).toBe(false);
    expect(needsForce(candidate("x", { upstream_gone: false, merged: false }))).toBe(false);
  });

  it("pre-checks gone branches and leaves merged ones unchecked", () => {
    const sel = initialSelection(makeList());
    expect(sel.has("gone-unmerged")).toBe(true);
    expect(sel.has("gone-merged")).toBe(true);
    expect(sel.has("merged-only")).toBe(false);
    expect(sel.size).toBe(2);
  });

  it("selectedForceNames returns only selected branches that need force", () => {
    const list = makeList();
    // Everything selected → only the gone-unmerged one needs force.
    const all = new Set(["gone-unmerged", "gone-merged", "merged-only"]);
    expect(selectedForceNames(list, all)).toEqual(["gone-unmerged"]);
    // Deselect the force-needing one → nothing needs force.
    expect(selectedForceNames(list, new Set(["gone-merged", "merged-only"]))).toEqual([]);
  });

  it("canDelete gates on the force acknowledgment when force is needed", () => {
    const list = makeList();
    const withForce = new Set(["gone-unmerged"]);
    expect(canDelete(list, withForce, false)).toBe(false); // needs ack
    expect(canDelete(list, withForce, true)).toBe(true);

    // A selection with no force-needing branch deletes without the ack.
    expect(canDelete(list, new Set(["merged-only"]), false)).toBe(true);
    expect(canDelete(list, new Set(["gone-merged"]), false)).toBe(true);

    // Nothing selected → never deletable.
    expect(canDelete(list, new Set(), true)).toBe(false);
  });
});
