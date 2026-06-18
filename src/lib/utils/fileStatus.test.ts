import { describe, it, expect } from "vitest";
import { normalizeFileStatus } from "./fileStatus";

describe("normalizeFileStatus", () => {
  it("maps the working-directory (staging) vocabulary", () => {
    expect(normalizeFileStatus("new")).toEqual({ kind: "added", letter: "A" });
    expect(normalizeFileStatus("modified")).toEqual({ kind: "modified", letter: "M" });
    expect(normalizeFileStatus("deleted")).toEqual({ kind: "deleted", letter: "D" });
    expect(normalizeFileStatus("renamed")).toEqual({ kind: "renamed", letter: "R" });
  });

  it("maps the diff vocabulary that used to fall through to '?'", () => {
    expect(normalizeFileStatus("added")).toEqual({ kind: "added", letter: "A" });
    expect(normalizeFileStatus("copied")).toEqual({ kind: "copied", letter: "C" });
    expect(normalizeFileStatus("untracked")).toEqual({ kind: "untracked", letter: "U" });
  });

  it("treats conflicts as a single conflicted kind", () => {
    expect(normalizeFileStatus("conflicted").kind).toBe("conflicted");
    expect(normalizeFileStatus("unmerged").kind).toBe("conflicted");
  });

  it("is case-insensitive", () => {
    expect(normalizeFileStatus("MODIFIED")).toEqual({ kind: "modified", letter: "M" });
  });

  it("returns the unknown kind for genuinely unrecognised values", () => {
    expect(normalizeFileStatus("wat")).toEqual({ kind: "unknown", letter: "?" });
    expect(normalizeFileStatus("")).toEqual({ kind: "unknown", letter: "?" });
  });
});
