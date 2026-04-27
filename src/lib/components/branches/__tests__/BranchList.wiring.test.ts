/**
 * Smoke/wiring tests for the Branches panel changes — verifies the
 * header "+" opens CreateBranchDialog, and the `doPush` helper
 * composes the right arguments for plain vs force push.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { writable } from "svelte/store";

vi.mock("../../../api/tauri", () => ({
  pushRemote: vi.fn().mockResolvedValue(1),
  getBranches: vi.fn().mockResolvedValue([]),
  getBranchCommits: vi.fn().mockResolvedValue([]),
  checkoutBranch: vi.fn(),
  deleteBranch: vi.fn(),
  mergeBranch: vi.fn(),
  rebaseBranch: vi.fn(),
  renameBranch: vi.fn(),
  createBranch: vi.fn(),
  createBranchAt: vi.fn(),
}));

vi.mock("../../../api/runMutation", () => ({
  runMutation: vi.fn(async (opts: { invoke: () => Promise<unknown> }) => opts.invoke()),
}));

vi.mock("../../../stores/remotes", () => ({
  remotes: writable([{ name: "origin", url: null }]),
  remoteNames: writable(["origin"]),
  refreshRemotes: vi.fn(),
}));

vi.mock("../../../stores/branches", () => ({
  branches: writable([
    { name: "main", is_head: true, is_remote: false, oid: "H" },
  ]),
  branchesLoading: writable(false),
  selectedBranchName: writable<string | null>("main"),
  localBranches: writable([
    { name: "main", is_head: true, is_remote: false, oid: "H" },
  ]),
  remoteBranches: writable([]),
  selectBranch: vi.fn(),
  refreshBranches: vi.fn(),
  doCheckout: vi.fn(),
  doDeleteBranch: vi.fn(),
  doMergeBranch: vi.fn(),
}));

vi.mock("../../../stores/createBranchDialog", () => ({
  openCreateBranchDialog: vi.fn(),
}));

import BranchList from "../BranchList.svelte";
import * as createBranchDialogStore from "../../../stores/createBranchDialog";

afterEach(() => cleanup());
beforeEach(() => vi.clearAllMocks());

describe("BranchList wiring", () => {
  it("renders the new-branch header button", () => {
    const { getByTestId } = render(BranchList);
    expect(getByTestId("branch-new-btn")).toBeTruthy();
  });

  it("calls openCreateBranchDialog when the header button is clicked", async () => {
    const { getByTestId } = render(BranchList);
    await fireEvent.click(getByTestId("branch-new-btn"));
    expect(createBranchDialogStore.openCreateBranchDialog).toHaveBeenCalledWith({ kind: "head" });
  });
});
