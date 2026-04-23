/**
 * Unit tests for `CreateBranchDialog.svelte`.
 *
 * The dialog is the single "create branch" entry point. Tests cover:
 * - Submit disabled when name is empty or whitespace.
 * - Name trim on submit.
 * - HEAD source dispatches `createBranch`; branch-tip source dispatches
 *   `createBranchAt(name, oid)`; commit source dispatches
 *   `createBranchAt(name, oid)`.
 * - Checkout checkbox chains a `checkoutBranch` call.
 * - Prefill runs when `open` flips true and source is a remote ref.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { tick } from "svelte";
import { writable } from "svelte/store";

vi.mock("../../../api/tauri", () => ({
  createBranch: vi.fn().mockResolvedValue(undefined),
  createBranchAt: vi.fn().mockResolvedValue(undefined),
  checkoutBranch: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../../../api/runMutation", () => ({
  runMutation: vi.fn(async (opts: { invoke: () => Promise<unknown> }) => {
    return await opts.invoke();
  }),
}));

vi.mock("../../../stores/remotes", () => ({
  remoteNames: writable<string[]>(["origin"]),
}));

vi.mock("../../../stores/branches", () => ({
  branches: writable([
    { name: "main", is_head: true, is_remote: false, oid: "HEADOID" },
    { name: "feature/x", is_head: false, is_remote: false, oid: "FEATOID" },
    { name: "origin/feature/x", is_head: false, is_remote: true, oid: "REMOID" },
  ]),
  localBranches: writable([
    { name: "main", is_head: true, is_remote: false, oid: "HEADOID" },
    { name: "feature/x", is_head: false, is_remote: false, oid: "FEATOID" },
  ]),
  remoteBranches: writable([
    { name: "origin/feature/x", is_head: false, is_remote: true, oid: "REMOID" },
  ]),
}));

import CreateBranchDialog from "../CreateBranchDialog.svelte";
import * as tauri from "../../../api/tauri";

afterEach(() => cleanup());
beforeEach(() => {
  (tauri.createBranch as ReturnType<typeof vi.fn>).mockClear();
  (tauri.createBranchAt as ReturnType<typeof vi.fn>).mockClear();
  (tauri.checkoutBranch as ReturnType<typeof vi.fn>).mockClear();
});

describe("CreateBranchDialog", () => {
  it("disables Create when name is empty", () => {
    const { getByTestId } = render(CreateBranchDialog, {
      props: {
        open: true,
        initialSource: { kind: "head" },
        onClose: () => {},
      },
    });
    const btn = getByTestId("create-branch-submit") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it("trims the name on submit", async () => {
    const { getByTestId } = render(CreateBranchDialog, {
      props: {
        open: true,
        initialSource: { kind: "head" },
        onClose: () => {},
      },
    });
    const nameInput = getByTestId("create-branch-name") as HTMLInputElement;
    await fireEvent.input(nameInput, { target: { value: "  feature/x  " } });
    const checkout = getByTestId("create-branch-checkout") as HTMLInputElement;
    if (checkout.checked) await fireEvent.click(checkout);
    const btn = getByTestId("create-branch-submit") as HTMLButtonElement;
    await fireEvent.click(btn);
    await tick();
    expect(tauri.createBranch).toHaveBeenCalledWith("feature/x");
  });

  it("dispatches createBranchAt for a branch-tip source", async () => {
    const { getByTestId } = render(CreateBranchDialog, {
      props: {
        open: true,
        initialSource: { kind: "ref", name: "feature/x", oid: "FEATOID" },
        onClose: () => {},
      },
    });
    const nameInput = getByTestId("create-branch-name") as HTMLInputElement;
    await fireEvent.input(nameInput, { target: { value: "new-branch" } });
    const checkout = getByTestId("create-branch-checkout") as HTMLInputElement;
    if (checkout.checked) await fireEvent.click(checkout);
    await fireEvent.click(getByTestId("create-branch-submit"));
    await tick();
    expect(tauri.createBranchAt).toHaveBeenCalledWith("new-branch", "FEATOID");
  });

  it("dispatches createBranchAt for a commit source", async () => {
    const { getByTestId } = render(CreateBranchDialog, {
      props: {
        open: true,
        initialSource: { kind: "commit", oid: "DEADBEEF" },
        onClose: () => {},
      },
    });
    const nameInput = getByTestId("create-branch-name") as HTMLInputElement;
    await fireEvent.input(nameInput, { target: { value: "from-commit" } });
    const checkout = getByTestId("create-branch-checkout") as HTMLInputElement;
    if (checkout.checked) await fireEvent.click(checkout);
    await fireEvent.click(getByTestId("create-branch-submit"));
    await tick();
    expect(tauri.createBranchAt).toHaveBeenCalledWith("from-commit", "DEADBEEF");
  });

  it("chains checkoutBranch when the checkbox is checked", async () => {
    const { getByTestId } = render(CreateBranchDialog, {
      props: {
        open: true,
        initialSource: { kind: "head" },
        onClose: () => {},
      },
    });
    const nameInput = getByTestId("create-branch-name") as HTMLInputElement;
    await fireEvent.input(nameInput, { target: { value: "nb" } });
    // Checkbox defaults to true — leave it on.
    await fireEvent.click(getByTestId("create-branch-submit"));
    await tick();
    expect(tauri.createBranch).toHaveBeenCalledWith("nb");
    expect(tauri.checkoutBranch).toHaveBeenCalledWith("nb");
  });

  it("prefills the name when opened against a remote ref", async () => {
    const { getByTestId } = render(CreateBranchDialog, {
      props: {
        open: true,
        initialSource: { kind: "ref", name: "origin/feature/x", oid: "REMOID" },
        onClose: () => {},
      },
    });
    await tick();
    const nameInput = getByTestId("create-branch-name") as HTMLInputElement;
    expect(nameInput.value).toBe("feature/x");
  });
});
