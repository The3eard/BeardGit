import { describe, expect, it } from "vitest";
import {
  joinRepoPath,
  normalizeDir,
  validateDir,
  validateLeaf,
} from "../path-validation";

describe("normalizeDir", () => {
  it("returns empty for blank input", () => {
    expect(normalizeDir("")).toBe("");
    expect(normalizeDir("   ")).toBe("");
    expect(normalizeDir("/")).toBe("");
  });
  it("converts backslashes and collapses separators", () => {
    expect(normalizeDir("src\\utils")).toBe("src/utils");
    expect(normalizeDir("src//utils/")).toBe("src/utils");
    expect(normalizeDir("/src/utils/")).toBe("src/utils");
  });
  it("trims segments", () => {
    expect(normalizeDir(" src / utils ")).toBe("src/utils");
  });
});

describe("validateLeaf", () => {
  it("accepts a plain name", () => {
    expect(validateLeaf("index.ts")).toBeNull();
  });
  it("rejects empty", () => {
    expect(validateLeaf("")).toBe("empty");
    expect(validateLeaf("   ")).toBe("empty");
  });
  it("rejects traversal", () => {
    expect(validateLeaf("..")).toBe("traversal");
    expect(validateLeaf("../x")).toBe("traversal");
  });
  it("rejects an absolute leaf", () => {
    expect(validateLeaf("/etc")).toBe("absolute");
  });
  it("rejects separators and reserved chars", () => {
    expect(validateLeaf("a/b")).toBe("invalid-chars");
    expect(validateLeaf("a\\b")).toBe("invalid-chars");
    expect(validateLeaf("a:b")).toBe("invalid-chars");
    expect(validateLeaf("a?b")).toBe("invalid-chars");
  });
});

describe("validateDir", () => {
  it("accepts the repo root (empty)", () => {
    expect(validateDir("")).toBeNull();
    expect(validateDir("   ")).toBeNull();
  });
  it("accepts nested directories", () => {
    expect(validateDir("src/lib/utils")).toBeNull();
    expect(validateDir("src\\lib")).toBeNull();
    expect(validateDir("src/lib/")).toBeNull();
  });
  it("rejects absolute paths", () => {
    expect(validateDir("/etc")).toBe("absolute");
    expect(validateDir("\\etc")).toBe("absolute");
  });
  it("rejects traversal segments", () => {
    expect(validateDir("../secrets")).toBe("traversal");
    expect(validateDir("src/../etc")).toBe("traversal");
    expect(validateDir("..")).toBe("traversal");
  });
  it("rejects windows drive / reserved chars", () => {
    expect(validateDir("C:/foo")).toBe("invalid-chars");
    expect(validateDir("a/b?c")).toBe("invalid-chars");
  });
});

describe("joinRepoPath", () => {
  it("joins parent and leaf", () => {
    expect(joinRepoPath("src/utils", "index.ts")).toBe("src/utils/index.ts");
  });
  it("returns the bare leaf at repo root", () => {
    expect(joinRepoPath("", "README.md")).toBe("README.md");
    expect(joinRepoPath("   ", "README.md")).toBe("README.md");
  });
  it("normalizes the parent before joining", () => {
    expect(joinRepoPath("src\\utils/", "x.ts")).toBe("src/utils/x.ts");
  });
});
