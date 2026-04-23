/**
 * Smoke test — ReflogDetail's "Create branch" button opens
 * CreateBranchDialog instead of window.prompt.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { writable } from "svelte/store";

vi.mock("../../../api/tauri", () => ({
  getCommitDetail: vi.fn().mockResolvedValue({
    oid: "deadbeef",
    summary: "x",
    body: "",
    author: "a",
    email: "a@a",
    timestamp: 0,
    parents: [],
    refs: [],
  }),
  getCommitFiles: vi.fn().mockResolvedValue([]),
  checkoutDetached: vi.fn(),
  resetToCommit: vi.fn(),
  createBranchAt: vi.fn(),
  renameBranch: vi.fn(),
}));

vi.mock("../../../stores/reflog", () => ({ loadReflog: vi.fn() }));
vi.mock("../../../stores/branches", () => ({
  selectedBranchName: writable<string | null>(null),
}));
vi.mock("../../../stores/remotes", () => ({
  remoteNames: writable<string[]>([]),
  remotes: writable([]),
  refreshRemotes: vi.fn(),
}));
vi.mock("../../../api/runMutation", () => ({
  runMutation: vi.fn(async (opts: { invoke: () => Promise<unknown> }) => opts.invoke()),
}));
vi.mock("../../../stores/createBranchDialog", () => ({
  openCreateBranchDialog: vi.fn(),
}));

import ReflogDetail from "../ReflogDetail.svelte";
import * as createBranchDialogStore from "../../../stores/createBranchDialog";

afterEach(() => cleanup());

describe("ReflogDetail create-branch", () => {
  it("calls openCreateBranchDialog instead of window.prompt when Create branch is clicked", async () => {
    const promptSpy = vi.spyOn(window, "prompt").mockReturnValue(null);
    const { findByText } = render(ReflogDetail, {
      props: {
        entry: { oid: "deadbeef", message: "test", ref: "HEAD", timestamp: 0 },
      },
    });
    const btn = await findByText(/Create branch/i);
    await fireEvent.click(btn);
    expect(promptSpy).not.toHaveBeenCalled();
    expect(createBranchDialogStore.openCreateBranchDialog).toHaveBeenCalledWith({
      kind: "commit",
      oid: "deadbeef",
    });
    promptSpy.mockRestore();
  });
});
