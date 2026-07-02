/**
 * Factories for branch fixtures: BranchInfo plus a multi-branch helper
 * that returns a realistic mix (HEAD + other locals + remote-tracking).
 */

import type { BranchInfo } from "../../lib/types";

function oid(seed: number): string {
  return seed.toString(16).padStart(40, "0");
}

export function makeBranchInfo(
  overrides: Partial<BranchInfo> = {},
): BranchInfo {
  return {
    name: "feat/example",
    is_head: false,
    is_remote: false,
    oid: oid(1),
    upstream: "origin/feat/example",
    ahead: 0,
    behind: 0,
    upstream_gone: false,
    ...overrides,
  };
}

/**
 * Realistic multi-branch fixture: one HEAD local, two other locals
 * (one with ahead/behind divergence), one remote-tracking. Use as the
 * `getBranches` response when seeding the Branches view.
 */
export function makeBranchList(): BranchInfo[] {
  return [
    makeBranchInfo({
      name: "feat/example",
      is_head: true,
      oid: oid(1),
      upstream: "origin/feat/example",
      ahead: 2,
      behind: 0,
    }),
    makeBranchInfo({
      name: "main",
      oid: oid(2),
      upstream: "origin/main",
      ahead: 0,
      behind: 5,
    }),
    makeBranchInfo({
      name: "fix/legacy-cleanup",
      oid: oid(3),
      upstream: null,
      ahead: 0,
      behind: 0,
    }),
    makeBranchInfo({
      name: "origin/main",
      is_remote: true,
      oid: oid(4),
      upstream: null,
    }),
  ];
}
