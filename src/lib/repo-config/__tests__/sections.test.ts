import { describe, it, expect } from "vitest";
import { sectionsForProvider, SECTION_IDS, type SectionId } from "../sections";

describe("sectionsForProvider", () => {
  it("returns the MVP five-section list for github", () => {
    const ids = sectionsForProvider("github").map((s) => s.id);
    expect(ids).toEqual([
      "general",
      "visibility",
      "features",
      "protection",
      "labels",
    ] satisfies SectionId[]);
  });

  it("returns the MVP five-section list for gitlab", () => {
    const ids = sectionsForProvider("gitlab").map((s) => s.id);
    expect(ids).toEqual([
      "general",
      "visibility",
      "features",
      "protection",
      "labels",
    ] satisfies SectionId[]);
  });

  it("every id is a canonical SECTION_IDS member", () => {
    for (const kind of ["github", "gitlab"] as const) {
      for (const s of sectionsForProvider(kind)) {
        expect(SECTION_IDS).toContain(s.id);
      }
    }
  });

  it("attaches a label and an icon glyph to every entry", () => {
    for (const s of sectionsForProvider("github")) {
      expect(s.label).toBeTypeOf("string");
      expect(s.label.length).toBeGreaterThan(0);
      expect(s.icon).toBeTypeOf("string");
      expect(s.icon.length).toBeGreaterThan(0);
    }
  });
});
