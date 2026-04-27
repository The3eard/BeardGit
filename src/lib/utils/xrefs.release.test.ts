import { describe, it, expect } from "vitest";
import { parseXrefs, type XrefContext } from "./xrefs";

function ctx(tags: string[]): XrefContext {
  return {
    mrPrCache: new Map(),
    issueCache: new Map(),
    releaseTagCache: new Set(tags),
    onOpenMrPr: () => {},
    onOpenIssue: () => {},
    onOpenRelease: () => {},
    onOpenExternal: () => {},
  };
}

describe("parseXrefs release tags (Phase 8.5)", () => {
  it("emits release segment for known tag with v prefix", () => {
    const segments = parseXrefs(
      "see v1.2.3 for details",
      ctx(["v1.2.3", "v0.1.8"]),
    );
    expect(
      segments.some((s) => s.type === "release" && s.tag === "v1.2.3"),
    ).toBe(true);
  });

  it("falls through for unknown version", () => {
    const segments = parseXrefs(
      "see v9.9.9 for details",
      ctx(["v1.2.3", "v0.1.8"]),
    );
    expect(
      segments.some((s) => s.type === "release" && s.tag === "v9.9.9"),
    ).toBe(false);
  });

  it("maps bare X.Y.Z to v-prefixed release tag when only v-prefixed is cached", () => {
    const segments = parseXrefs("see 1.2.3", ctx(["v1.2.3"]));
    const release = segments.find((s) => s.type === "release");
    expect(release).toMatchObject({ tag: "v1.2.3", display: "1.2.3" });
  });

  it("does not misfire on page-size-like numbers", () => {
    // "100px" lacks .N.N pattern so shouldn't match
    const segments = parseXrefs("width is 100 pixels", ctx(["v1.2.3"]));
    expect(segments.every((s) => s.type === "text")).toBe(true);
  });

  it("multiple release tags in the same text are all resolved", () => {
    const segments = parseXrefs(
      "compare v1.0.0 against v1.2.3",
      ctx(["v1.0.0", "v1.2.3"]),
    );
    const releases = segments.filter((s) => s.type === "release");
    expect(releases).toHaveLength(2);
  });
});
