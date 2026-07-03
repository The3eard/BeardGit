/**
 * Store-level assertion for the typed error envelope on the branch/checkout
 * commands migrated in this pass: a checkout that would overwrite local
 * changes, and a safe branch delete refused because the branch is unmerged,
 * both surface their structured `code` through the same `runMutation` façade
 * the call sites use — so the frontend can branch on it (and render an
 * actionable label) rather than parsing free text.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import { checkoutBranch, deleteBranch, renameBranch } from "$lib/api/tauri";
import { runMutation } from "$lib/api/runMutation";
import { getErrorCode, errorCodeMessage } from "$lib/api/errors";

beforeEach(() => mocks.invoke.mockReset());

describe("checkout dirty-tree failure surfaces its code", () => {
  it("rejects with an IpcError whose would_lose_changes code getErrorCode extracts", async () => {
    mocks.invoke.mockRejectedValueOnce({
      code: "would_lose_changes",
      message: "1 conflict prevents checkout",
    });

    let caught: unknown;
    await runMutation({
      kind: "checkout",
      invoke: () => checkoutBranch("feature"),
      failureToastPrefix: "Checkout failed",
    }).catch((e) => {
      caught = e;
    });

    expect(getErrorCode(caught)).toBe("would_lose_changes");
    expect(errorCodeMessage(getErrorCode(caught)!)).toBe(
      "Checkout would overwrite uncommitted changes — commit or stash first",
    );
    expect(mocks.invoke).toHaveBeenCalledWith("checkout_branch", {
      name: "feature",
    });
  });
});

describe("safe branch delete of an unmerged branch surfaces its code", () => {
  it("rejects with an IpcError whose not_fully_merged code getErrorCode extracts", async () => {
    mocks.invoke.mockRejectedValueOnce({
      code: "not_fully_merged",
      message: "error: The branch 'x' is not fully merged.",
    });

    let caught: unknown;
    await runMutation({
      kind: "delete_branch",
      invoke: () => deleteBranch("x", false),
      failureToastPrefix: "Delete failed",
    }).catch((e) => {
      caught = e;
    });

    expect(getErrorCode(caught)).toBe("not_fully_merged");
    expect(mocks.invoke).toHaveBeenCalledWith("delete_branch", {
      name: "x",
      force: false,
    });
  });
});

describe("renaming onto an existing branch name surfaces its code", () => {
  it("rejects with an IpcError whose branch_exists code getErrorCode extracts", async () => {
    mocks.invoke.mockRejectedValueOnce({
      code: "branch_exists",
      message: "fatal: a branch named 'feat/b' already exists",
    });

    let caught: unknown;
    await runMutation({
      kind: "rename_branch",
      invoke: () => renameBranch("feat/a", "feat/b"),
      failureToastPrefix: "Rename failed",
    }).catch((e) => {
      caught = e;
    });

    expect(getErrorCode(caught)).toBe("branch_exists");
    expect(errorCodeMessage(getErrorCode(caught)!)).toBe(
      "A branch with that name already exists — choose a different name",
    );
    expect(mocks.invoke).toHaveBeenCalledWith("rename_branch", {
      oldName: "feat/a",
      newName: "feat/b",
    });
  });
});
