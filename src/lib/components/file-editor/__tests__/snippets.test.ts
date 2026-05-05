/**
 * Snippet pack smoke tests — verifies the per-language packs return
 * `Completion` arrays for the supported languages and an empty list
 * for unmapped ones. We don't assert template content (that's the
 * CodeMirror snippet runtime's job) — only that the labels we
 * advertise in the docs / spec actually exist.
 */

import { describe, expect, it } from "vitest";
import { snippetsForLanguage } from "../snippets";

describe("snippetsForLanguage", () => {
  it("returns a non-empty pack for typescript with the `fn` snippet", () => {
    const pack = snippetsForLanguage("typescript");
    expect(pack.length).toBeGreaterThan(0);
    const labels = pack.map((c) => c.label);
    expect(labels).toContain("fn");
    expect(labels).toContain("class");
    expect(labels).toContain("interface");
  });

  it("returns a Rust pack with the canonical bread-and-butter snippets", () => {
    const labels = snippetsForLanguage("rust").map((c) => c.label);
    for (const expected of ["fn", "pub fn", "impl", "match", "let", "if let", "struct", "enum"]) {
      expect(labels).toContain(expected);
    }
  });

  it("returns a Python pack with `def` / `class` / `with`", () => {
    const labels = snippetsForLanguage("python").map((c) => c.label);
    expect(labels).toContain("def");
    expect(labels).toContain("class");
    expect(labels).toContain("with");
  });

  it("returns a Go pack with `func` / `defer`", () => {
    const labels = snippetsForLanguage("go").map((c) => c.label);
    expect(labels).toContain("func");
    expect(labels).toContain("defer");
  });

  it("returns an empty array for unmapped languages", () => {
    expect(snippetsForLanguage("haskell")).toEqual([]);
    expect(snippetsForLanguage("")).toEqual([]);
  });
});
