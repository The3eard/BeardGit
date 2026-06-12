/**
 * Per-line syntax highlighting for the hand-rolled diff renderers
 * (StagingDiffEditor) — the views that can't host a full CodeMirror
 * instance because they own their line layout (hunk checkboxes,
 * stage/discard affordances).
 *
 * Reuses the same lazily-loaded `@codemirror/lang-*` packs as the real
 * editors (via `loadLanguageExtension`), but only borrows their lezer
 * parser: each line is parsed in isolation and emitted as HTML spans
 * with `@lezer/highlight`'s `classHighlighter` classes (`tok-keyword`,
 * `tok-string`, …). Colors come from the `--syntax-*` CSS custom
 * properties that `theme.ts::applyTheme()` keeps in sync with the
 * active theme's `[editor]` palette (see `lib/styles/syntax.css`).
 *
 * Parsing a line out of context is an approximation (multi-line
 * constructs like block comments won't carry across lines), which is
 * the accepted trade-off for keeping the staging renderer's DOM.
 */

import { classHighlighter, highlightCode } from "@lezer/highlight";
import type { Parser } from "@lezer/common";
import {
  getLanguageExtensionName,
  loadLanguageExtension,
} from "./language-support";

/** Per-language parser cache (`null` = pack has no usable lezer parser). */
const parserCache = new Map<string, Parser | null>();

/**
 * Resolve the lezer parser for a filename, or `null` when the language
 * is unknown / the pack doesn't expose one. Async because language
 * packs are dynamic imports; resolved parsers are cached per language.
 */
export async function loadLineParser(filename: string): Promise<Parser | null> {
  const lang = getLanguageExtensionName(filename);
  if (!lang) return null;
  const cached = parserCache.get(lang);
  if (cached !== undefined) return cached;
  const ext = await loadLanguageExtension(lang);
  const parser =
    (ext as { language?: { parser?: Parser } } | null)?.language?.parser ??
    null;
  parserCache.set(lang, parser);
  return parser;
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/**
 * Highlight a single line to an HTML string (all text HTML-escaped).
 * Falls back to the escaped plain line when there's no parser or the
 * parse throws.
 */
export function highlightLineHtml(parser: Parser | null, line: string): string {
  if (!parser || !line) return escapeHtml(line);
  try {
    const tree = parser.parse(line);
    let html = "";
    highlightCode(
      line,
      tree,
      classHighlighter,
      (text, classes) => {
        html += classes
          ? `<span class="${classes}">${escapeHtml(text)}</span>`
          : escapeHtml(text);
      },
      () => {
        html += "\n";
      },
    );
    return html;
  } catch {
    return escapeHtml(line);
  }
}
