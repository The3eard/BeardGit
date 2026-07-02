/**
 * Unit tests for the branch-cleanup store action (spec 11): `doDeleteBranches`
 * passes the force subset through, returns the batch result, and clears the
 * selection when the selected branch was among those deleted.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { mockInvokeResponse, clearInvokeMocks, invokeMock } from "../../test/setup";
import { __resetRepoStateForTests } from "./repo-state";
import { doDeleteBranches, selectedBranchName } from "./branches";

describe("doDeleteBranches", () => {
  beforeEach(() => {
    clearInvokeMocks();
    __resetRepoStateForTests();
  });

  it("forwards names + force and returns the batch result", async () => {
    mockInvokeResponse("delete_branches", { deleted: ["a", "b"], failed: [] });

    const result = await doDeleteBranches(["a", "b"], ["b"]);

    expect(result).toEqual({ deleted: ["a", "b"], failed: [] });
    const call = invokeMock.mock.calls.find((c) => c[0] === "delete_branches");
    expect(call?.[1]).toEqual({ names: ["a", "b"], force: ["b"] });
  });

  it("surfaces partial failures in the result", async () => {
    mockInvokeResponse("delete_branches", {
      deleted: ["a"],
      failed: [{ name: "main", reason: "cannot delete branch checked out" }],
    });

    const result = await doDeleteBranches(["a", "main"], []);
    expect(result.deleted).toEqual(["a"]);
    expect(result.failed).toHaveLength(1);
    expect(result.failed[0].name).toBe("main");
  });

  it("clears the current selection when the selected branch is deleted", async () => {
    selectedBranchName.set("a");
    mockInvokeResponse("delete_branches", { deleted: ["a"], failed: [] });

    await doDeleteBranches(["a"], ["a"]);
    expect(get(selectedBranchName)).toBeNull();
  });

  it("keeps the selection when a different branch is deleted", async () => {
    selectedBranchName.set("keep");
    mockInvokeResponse("delete_branches", { deleted: ["a"], failed: [] });

    await doDeleteBranches(["a"], []);
    expect(get(selectedBranchName)).toBe("keep");
  });
});
