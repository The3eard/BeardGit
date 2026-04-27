/**
 * Unit tests for the shared `createBranchDialog` store — used by the
 * ⌘⇧B global shortcut to open the dialog from anywhere in the app.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  createBranchDialog,
  openCreateBranchDialog,
  closeCreateBranchDialog,
} from "../createBranchDialog";

beforeEach(() => closeCreateBranchDialog());

describe("createBranchDialog store", () => {
  it("defaults to closed", () => {
    expect(get(createBranchDialog).open).toBe(false);
  });

  it("opens with a HEAD source by default", () => {
    openCreateBranchDialog();
    const st = get(createBranchDialog);
    expect(st.open).toBe(true);
    expect(st.source).toEqual({ kind: "head" });
  });

  it("opens with the supplied source", () => {
    openCreateBranchDialog({ kind: "commit", oid: "abc" });
    expect(get(createBranchDialog).source).toEqual({ kind: "commit", oid: "abc" });
  });

  it("is a no-op when already open (prevents re-opening while another dialog is live)", () => {
    openCreateBranchDialog();
    openCreateBranchDialog({ kind: "commit", oid: "x" });
    // Still holds the first source — callers must close before re-opening.
    expect(get(createBranchDialog).source).toEqual({ kind: "head" });
  });

  it("closes", () => {
    openCreateBranchDialog();
    closeCreateBranchDialog();
    expect(get(createBranchDialog).open).toBe(false);
  });
});
