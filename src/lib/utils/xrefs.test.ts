import { describe, it, expect } from "vitest";
import { parseXrefs, type XrefContext } from "./xrefs";
import type { MrPr, Issue } from "../types";

function ctx(opts: {
  mrPrs?: MrPr[];
  issues?: Issue[];
  tags?: string[];
} = {}): XrefContext {
  return {
    mrPrCache: new Map((opts.mrPrs ?? []).map((m) => [m.number, m])),
    issueCache: new Map((opts.issues ?? []).map((i) => [i.number, i])),
    releaseTagCache: new Set(opts.tags ?? []),
    onOpenMrPr: () => {},
    onOpenIssue: () => {},
    onOpenRelease: () => {},
    onOpenExternal: () => {},
  };
}

function mkMr(n: number): MrPr {
  return { number: n } as MrPr;
}
function mkIssue(n: number): Issue {
  return { number: n } as Issue;
}

describe("parseXrefs", () => {
  it("returns plain text when no patterns", () => {
    const r = parseXrefs("hello world", ctx());
    expect(r).toEqual([{ type: "text", value: "hello world" }]);
  });

  it("returns empty for empty input", () => {
    expect(parseXrefs("", ctx())).toEqual([]);
  });

  it("parses #N as MR/PR when cached as MR", () => {
    const r = parseXrefs("see #42 for details", ctx({ mrPrs: [mkMr(42)] }));
    expect(r.find((s) => s.type === "mr_pr")).toMatchObject({ number: 42 });
  });

  it("parses #N as Issue when only issue is cached", () => {
    const r = parseXrefs("see #7 please", ctx({ issues: [mkIssue(7)] }));
    expect(r.find((s) => s.type === "issue")).toMatchObject({ number: 7 });
  });

  it("prefers MR/PR over issue when both cached", () => {
    const r = parseXrefs("#5", ctx({ mrPrs: [mkMr(5)], issues: [mkIssue(5)] }));
    expect(r.find((s) => s.type === "mr_pr" || s.type === "issue")?.type).toBe(
      "mr_pr",
    );
  });

  it("falls through to text when #N not in any cache", () => {
    const r = parseXrefs("nothing #999 here", ctx());
    expect(r.every((s) => s.type === "text")).toBe(true);
  });

  it("parses external URLs", () => {
    const r = parseXrefs("click https://example.com now", ctx());
    expect(r.find((s) => s.type === "external")).toMatchObject({
      url: "https://example.com",
    });
  });

  it("parses release tag when in tag cache (bare vX.Y.Z)", () => {
    const r = parseXrefs("see v1.2.3 release", ctx({ tags: ["v1.2.3"] }));
    expect(r.find((s) => s.type === "release")).toMatchObject({
      tag: "v1.2.3",
    });
  });

  it("matches X.Y.Z and prefixes v when the v-prefixed tag exists", () => {
    const r = parseXrefs("see 1.2.3", ctx({ tags: ["v1.2.3"] }));
    const seg = r.find((s) => s.type === "release");
    expect(seg).toMatchObject({ tag: "v1.2.3", display: "1.2.3" });
  });

  it("does not match version-like number when not in tag cache", () => {
    const r = parseXrefs("version 1.2.3 is unknown", ctx());
    expect(r.every((s) => s.type === "text")).toBe(true);
  });

  it("splits text around multiple matches", () => {
    const r = parseXrefs(
      "see #1 and #2",
      ctx({ mrPrs: [mkMr(1), mkMr(2)] }),
    );
    expect(r).toHaveLength(4); // "see ", #1, " and ", #2
    expect(r[1].type).toBe("mr_pr");
    expect(r[3].type).toBe("mr_pr");
  });
});
