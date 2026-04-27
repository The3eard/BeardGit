/**
 * Unit tests for `RenameBranchDialog.svelte`.
 *
 * Covers:
 * - Rename is disabled when name is empty.
 * - Rename is disabled when new name equals the current name.
 * - On success, `selectedBranchName` is updated when the renamed branch
 *   was selected.
 * - `renameBranch` is called with the trimmed new name.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { tick } from "svelte";
import { get } from "svelte/store";

// vi.hoisted runs before any imports, before vi.mock factories.
// We create the writable here so the mock factory can close over it.
const { selectedBranchName } = vi.hoisted(() => {
  // Use require() style because ES imports are not available here.
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return { selectedBranchName: writable<string | null>("old-name") };
});

vi.mock("../../../api/tauri", () => ({
  renameBranch: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../../../api/runMutation", () => ({
  runMutation: vi.fn(async (opts: { invoke: () => Promise<unknown> }) => opts.invoke()),
}));

vi.mock("../../../stores/branches", () => ({
  selectedBranchName,
}));

import RenameBranchDialog from "../RenameBranchDialog.svelte";
import * as tauri from "../../../api/tauri";

afterEach(() => cleanup());
beforeEach(() => {
  (tauri.renameBranch as ReturnType<typeof vi.fn>).mockClear();
  selectedBranchName.set("old-name");
});

describe("RenameBranchDialog", () => {
  it("disables Rename when the input is empty", async () => {
    const { getByTestId } = render(RenameBranchDialog, {
      props: { open: true, currentName: "old-name", onClose: () => {} },
    });
    const input = getByTestId("rename-branch-input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "" } });
    const btn = getByTestId("rename-branch-submit") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it("disables Rename when the input equals the current name", () => {
    const { getByTestId } = render(RenameBranchDialog, {
      props: { open: true, currentName: "old-name", onClose: () => {} },
    });
    const btn = getByTestId("rename-branch-submit") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it("calls renameBranch with trimmed new name", async () => {
    const { getByTestId } = render(RenameBranchDialog, {
      props: { open: true, currentName: "old-name", onClose: () => {} },
    });
    const input = getByTestId("rename-branch-input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "  new-name  " } });
    await fireEvent.click(getByTestId("rename-branch-submit"));
    await tick();
    expect(tauri.renameBranch).toHaveBeenCalledWith("old-name", "new-name");
  });

  it("updates selectedBranchName when HEAD branch is renamed", async () => {
    const { getByTestId } = render(RenameBranchDialog, {
      props: { open: true, currentName: "old-name", onClose: () => {} },
    });
    const input = getByTestId("rename-branch-input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "new-name" } });
    await fireEvent.click(getByTestId("rename-branch-submit"));
    await tick();
    expect(get(selectedBranchName)).toBe("new-name");
  });
});
