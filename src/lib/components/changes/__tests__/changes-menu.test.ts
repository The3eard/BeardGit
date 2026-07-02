import { describe, it, expect } from "vitest";
import { isBatchSelection, batchActionIds } from "../changes-menu";

describe("isBatchSelection", () => {
  it("is false when fewer than two files are checked", () => {
    expect(isBatchSelection(new Set(), "a.txt")).toBe(false);
    expect(isBatchSelection(new Set(["a.txt"]), "a.txt")).toBe(false);
  });

  it("is false when the cursor file is not part of the selection", () => {
    expect(isBatchSelection(new Set(["a.txt", "b.txt"]), "c.txt")).toBe(false);
  });

  it("is true with ≥2 checked and the cursor file among them", () => {
    expect(isBatchSelection(new Set(["a.txt", "b.txt"]), "a.txt")).toBe(true);
    expect(isBatchSelection(new Set(["a.txt", "b.txt", "c.txt"]), "c.txt")).toBe(true);
  });
});

describe("batchActionIds", () => {
  it("offers stage + discard on the unstaged list", () => {
    expect(batchActionIds(false)).toEqual(["stage", "discard", "stash", "copyPaths"]);
  });

  it("offers unstage (no stage/discard) on the staged list", () => {
    const ids = batchActionIds(true);
    expect(ids).toEqual(["unstage", "stash", "copyPaths"]);
    expect(ids).not.toContain("stage");
    expect(ids).not.toContain("discard");
  });

  it("both lists can stash and copy paths", () => {
    expect(batchActionIds(false)).toEqual(expect.arrayContaining(["stash", "copyPaths"]));
    expect(batchActionIds(true)).toEqual(expect.arrayContaining(["stash", "copyPaths"]));
  });
});
