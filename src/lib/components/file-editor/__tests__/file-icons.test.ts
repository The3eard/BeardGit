/**
 * Sanity tests for the `fileGlyphFor` Nerd Font icon helper.
 *
 * We only check that a few well-known files map to non-empty glyphs and
 * that the generic fallback fires for unknown extensions — the exact
 * codepoint per file is documented in `file-icons.ts` itself.
 */

import { describe, expect, it } from "vitest";
import { fileGlyphFor } from "../file-icons";

describe("fileGlyphFor", () => {
  it("returns a glyph for known basenames", () => {
    expect(fileGlyphFor("package.json")).toBeTruthy();
    expect(fileGlyphFor("Dockerfile")).toBeTruthy();
    expect(fileGlyphFor("Cargo.toml")).toBeTruthy();
    expect(fileGlyphFor("README.md")).toBeTruthy();
  });

  it("falls back to the extension lookup for unknown basenames", () => {
    // `something.ts` isn't in BASENAME_MAP — it's resolved via the .ts
    // extension. Glyph must be non-empty.
    expect(fileGlyphFor("something.ts")).toBeTruthy();
    expect(fileGlyphFor("a.rs")).toBeTruthy();
  });

  it("returns the generic glyph for unknown extensions", () => {
    const generic = fileGlyphFor("file.unknownext");
    const alsoGeneric = fileGlyphFor("noextension");
    expect(generic).toBeTruthy();
    expect(alsoGeneric).toBeTruthy();
    // Both unknown shapes resolve to the same fallback glyph.
    expect(generic).toEqual(alsoGeneric);
  });

  it("treats trailing-dot names as having no extension", () => {
    expect(fileGlyphFor("trailing.")).toBeTruthy();
  });
});
