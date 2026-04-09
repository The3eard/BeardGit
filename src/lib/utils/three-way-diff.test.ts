import { describe, it, expect } from "vitest";
import { diffLines, threeWayDiff, buildMergedResult } from "./three-way-diff";

describe("diffLines", () => {
  it("identical documents produce a single equal edit", () => {
    const lines = ["a", "b", "c"];
    const result = diffLines(lines, lines);
    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("equal");
    expect(result[0].oldLines).toEqual(["a", "b", "c"]);
  });

  it("empty old and new produce no edits", () => {
    expect(diffLines([], [])).toEqual([]);
  });

  it("insertion at end", () => {
    const result = diffLines(["a", "b"], ["a", "b", "c"]);
    expect(result).toHaveLength(2);
    expect(result[0].type).toBe("equal");
    expect(result[1].type).toBe("insert");
    expect(result[1].newLines).toEqual(["c"]);
  });

  it("deletion at end", () => {
    const result = diffLines(["a", "b", "c"], ["a", "b"]);
    expect(result).toHaveLength(2);
    expect(result[0].type).toBe("equal");
    expect(result[1].type).toBe("delete");
    expect(result[1].oldLines).toEqual(["c"]);
  });

  it("replacement in middle", () => {
    const result = diffLines(["a", "b", "c"], ["a", "x", "c"]);
    const types = result.map((e) => e.type);
    expect(types).toContain("delete");
    expect(types).toContain("insert");
  });

  it("completely different content", () => {
    const result = diffLines(["a", "b"], ["x", "y"]);
    const hasDelete = result.some((e) => e.type === "delete");
    const hasInsert = result.some((e) => e.type === "insert");
    expect(hasDelete).toBe(true);
    expect(hasInsert).toBe(true);
  });
});

describe("threeWayDiff", () => {
  it("identical files produce a single unchanged chunk", () => {
    const text = "a\nb\nc";
    const result = threeWayDiff(text, text, text);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("unchanged");
  });

  it("theirs-only change", () => {
    const base = "a\nb\nc";
    const theirs = "a\nX\nc";
    const ours = "a\nb\nc";
    const result = threeWayDiff(base, theirs, ours);
    const kinds = result.map((c) => c.kind);
    expect(kinds).toContain("theirs_only");
    expect(kinds).not.toContain("conflict");
  });

  it("ours-only change", () => {
    const base = "a\nb\nc";
    const theirs = "a\nb\nc";
    const ours = "a\nY\nc";
    const result = threeWayDiff(base, theirs, ours);
    const kinds = result.map((c) => c.kind);
    expect(kinds).toContain("ours_only");
    expect(kinds).not.toContain("conflict");
  });

  it("both sides changing same line is a conflict", () => {
    const base = "a\nb\nc";
    const theirs = "a\nX\nc";
    const ours = "a\nY\nc";
    const result = threeWayDiff(base, theirs, ours);
    const kinds = result.map((c) => c.kind);
    expect(kinds).toContain("conflict");
  });

  it("non-overlapping changes are not conflicts", () => {
    const base = "a\nb\nc\nd";
    const theirs = "X\nb\nc\nd";
    const ours = "a\nb\nc\nY";
    const result = threeWayDiff(base, theirs, ours);
    const kinds = result.map((c) => c.kind);
    expect(kinds).toContain("theirs_only");
    expect(kinds).toContain("ours_only");
    expect(kinds).not.toContain("conflict");
  });

  it("empty base with insertions on both sides is a conflict", () => {
    const result = threeWayDiff("", "new theirs", "new ours");
    const kinds = result.map((c) => c.kind);
    expect(kinds).toContain("conflict");
  });
});

describe("buildMergedResult", () => {
  it("auto-applies non-conflicting changes", () => {
    const base = "a\nb\nc\nd";
    const theirs = "X\nb\nc\nd";
    const ours = "a\nb\nc\nY";
    const chunks = threeWayDiff(base, theirs, ours);
    const result = buildMergedResult(
      chunks,
      base.split("\n"),
      theirs.split("\n"),
      ours.split("\n"),
      (i) => `<<conflict ${i}>>`,
    );
    expect(result).toContain("X");
    expect(result).toContain("Y");
    expect(result).not.toContain("<<conflict");
  });

  it("conflicts produce placeholder lines", () => {
    const base = "a\nb\nc";
    const theirs = "a\nX\nc";
    const ours = "a\nY\nc";
    const chunks = threeWayDiff(base, theirs, ours);
    const result = buildMergedResult(
      chunks,
      base.split("\n"),
      theirs.split("\n"),
      ours.split("\n"),
      (i) => `<<conflict ${i}>>`,
    );
    expect(result).toContain("<<conflict 0>>");
  });
});
