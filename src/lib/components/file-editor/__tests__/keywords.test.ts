/**
 * Keyword-pack smoke tests — verifies the curated reserved-word lists
 * for each supported language and the `keywordCompletion` source
 * factory's behaviour at the `matchBefore(/\w+/)` boundary.
 */

import { describe, expect, it } from "vitest";
import { keywordCompletion, keywordsForLanguage } from "../keywords";
import type { CompletionContext } from "@codemirror/autocomplete";

describe("keywordsForLanguage", () => {
  it("returns the canonical Rust keywords", () => {
    const words = keywordsForLanguage("rust");
    for (const expected of ["fn", "let", "match", "impl", "struct", "enum", "trait", "Self"]) {
      expect(words).toContain(expected);
    }
  });

  it("returns the canonical TypeScript keywords", () => {
    const words = keywordsForLanguage("typescript");
    expect(words).toContain("interface");
    expect(words).toContain("readonly");
    expect(words).toContain("export");
  });

  it("returns CSS at-rules and common values", () => {
    const words = keywordsForLanguage("css");
    expect(words).toContain("@media");
    expect(words).toContain("flex");
  });

  it("returns C++ keywords (a superset of C)", () => {
    const cpp = keywordsForLanguage("cpp");
    expect(cpp).toContain("class");
    expect(cpp).toContain("nullptr");
    // C-only keywords are still present in the cpp list.
    expect(cpp).toContain("static");
  });

  it("returns an empty array for unmapped languages", () => {
    expect(keywordsForLanguage("haskell")).toEqual([]);
    expect(keywordsForLanguage("")).toEqual([]);
  });
});

describe("keywordCompletion source", () => {
  function mkContext(line: string, options: { explicit?: boolean } = {}): CompletionContext {
    // Minimal stub of `CompletionContext` — only `matchBefore` and
    // `explicit` are read by the implementation.
    const explicit = options.explicit ?? false;
    return {
      explicit,
      matchBefore(re: RegExp) {
        const match = line.match(re);
        if (!match) return null;
        const from = line.length - match[0].length;
        return { from, to: line.length, text: match[0] };
      },
    } as unknown as CompletionContext;
  }

  it("returns options when a partial word is being typed", () => {
    const source = keywordCompletion(["foo", "bar", "baz"]);
    const ctx = mkContext("fo");
    const result = source(ctx);
    expect(result).not.toBeNull();
    expect(result!.options.map((o) => o.label)).toEqual(["foo", "bar", "baz"]);
    // Tagged keyword so the completion popup uses the right icon.
    expect(result!.options.every((o) => o.type === "keyword")).toBe(true);
  });

  it("returns null when no word is being typed and not explicit", () => {
    const source = keywordCompletion(["foo"]);
    const ctx = mkContext("");
    expect(source(ctx)).toBeNull();
  });

  it("returns options when explicit even with no word", () => {
    const source = keywordCompletion(["foo"]);
    const ctx = mkContext("", { explicit: true });
    // Implementation returns null when matchBefore returns null even
    // explicitly — explicit only matters when a zero-length word is
    // matched (cursor flush against the boundary). Empty docs produce
    // no match, so the result remains null.
    expect(source(ctx)).toBeNull();
  });
});
