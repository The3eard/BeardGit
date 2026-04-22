/**
 * Unit tests for {@link renderMarkdown} — the GFM renderer + sanitiser that
 * powers the body/comment/description surfaces in `ReleaseDetail`,
 * `IssueDetail`, and `MrPrDetail`.
 *
 * Coverage focuses on the GFM features that `snarkdown` (the previous
 * renderer) either dropped or garbled — fenced code blocks, GFM tables,
 * task lists — plus a handful of XSS regressions against the sanitiser.
 */
import { describe, expect, it } from "vitest";
import { renderMarkdown } from "../markdown";

describe("renderMarkdown — GFM", () => {
  it("renders fenced code blocks as <pre><code> with HTML-escaped content", () => {
    const html = renderMarkdown("```\n<script>a</script>\n```");
    expect(html).toMatch(/<pre><code[^>]*>/i);
    expect(html).toContain("&lt;script&gt;a&lt;/script&gt;");
    // Literal <script> tag must not appear inside the output.
    expect(html).not.toMatch(/<script/i);
  });

  it("renders GFM tables with thead/tbody and the expected cells", () => {
    const html = renderMarkdown("| a | b |\n| - | - |\n| 1 | 2 |\n");
    expect(html).toMatch(/<table>[\s\S]*<thead>[\s\S]*<\/thead>[\s\S]*<tbody>[\s\S]*<\/tbody>[\s\S]*<\/table>/);
    expect(html).toMatch(/<th>\s*a\s*<\/th>/);
    expect(html).toMatch(/<td>\s*1\s*<\/td>/);
  });

  it("renders task lists with <input type=\"checkbox\"> elements preserved", () => {
    const html = renderMarkdown("- [ ] todo\n- [x] done\n");
    const matches = html.match(/<input[^>]*type=["']?checkbox["']?[^>]*>/gi) ?? [];
    expect(matches.length).toBe(2);
    // One of the two should be pre-checked.
    expect(html).toMatch(/<input[^>]*checked[^>]*>/i);
  });

  it("rewrites <a> links with target=\"_blank\" + rel=\"noopener noreferrer\"", () => {
    const html = renderMarkdown("[beardgit](https://example.com)");
    expect(html).toMatch(
      /<a\s+target="_blank"\s+rel="noopener noreferrer"\s+href="https:\/\/example\.com"[^>]*>beardgit<\/a>/i,
    );
  });

  it("renders ~~strikethrough~~ as <del>", () => {
    const html = renderMarkdown("~~gone~~");
    expect(html).toMatch(/<del>gone<\/del>/);
  });

  it("renders inline `code` spans as <code>", () => {
    const html = renderMarkdown("tap `foo()`");
    expect(html).toMatch(/<code>foo\(\)<\/code>/);
  });
});

describe("renderMarkdown — sanitiser / XSS", () => {
  it("strips raw <script> tags", () => {
    const html = renderMarkdown("<script>alert(1)</script>");
    expect(html).not.toMatch(/<script/i);
    expect(html).not.toContain("alert(1)</script>");
  });

  it("neutralises javascript: URLs on links", () => {
    // marked itself does not escape `javascript:` URL schemes, so the
    // sanitiser must strip the href. The text should survive.
    const html = renderMarkdown("[x](javascript:alert(1))");
    expect(html).not.toMatch(/href\s*=\s*["']?javascript:/i);
    expect(html).toContain("x");
  });

  it("strips event-handler attributes on allowed tags", () => {
    const html = renderMarkdown('<a href="x" onclick="evil()">y</a>');
    expect(html).not.toMatch(/onclick/i);
    expect(html).toContain("y");
  });

  it("strips disallowed inline tags (e.g. <iframe>) while keeping inner text", () => {
    const html = renderMarkdown("<iframe src=\"x\">hi</iframe>");
    expect(html).not.toMatch(/<iframe/i);
    expect(html).not.toMatch(/<\/iframe>/i);
    expect(html).toContain("hi");
  });

  it("strips non-checkbox <input> elements", () => {
    const html = renderMarkdown('<input type="text" name="pwned">');
    expect(html).not.toMatch(/<input/i);
  });

  it("keeps <input type=\"checkbox\"> when authored directly", () => {
    const html = renderMarkdown('<input type="checkbox" disabled>');
    expect(html).toMatch(/<input[^>]*type=["']?checkbox["']?[^>]*>/i);
  });
});

describe("renderMarkdown — guards", () => {
  it("returns an empty string for empty input", () => {
    expect(renderMarkdown("")).toBe("");
  });

  it("returns an empty string for falsy input (runtime guard)", () => {
    // TypeScript prevents this at compile time but Xrefs/runtime callers
    // may still pass null/undefined through `as string`.
    expect(renderMarkdown(undefined as unknown as string)).toBe("");
    expect(renderMarkdown(null as unknown as string)).toBe("");
  });
});
