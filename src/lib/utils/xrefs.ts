/**
 * Cross-reference parser for free-form text (commit messages, MR/PR bodies,
 * issue descriptions, release notes).
 *
 * Recognized patterns (in order of precedence at match position):
 *   - `https?://...`          → external URL
 *   - `v?\d+\.\d+\.\d+...`    → release tag (only if in `releaseTagCache`)
 *   - `#N`                    → MR/PR (if cached) or Issue (if cached)
 *
 * Anything unmatched stays as plain text. The output is a flat list of
 * `XrefSegment` objects that a renderer (e.g. the `<Xrefs>` Svelte
 * component) walks in order.
 *
 * Used by Phase 8.3 (issues) and by Phase 8.5 (releases) — the context
 * object is generic over all xref types so both consumers share the same
 * resolution logic.
 */

import type { MrPr, Issue } from "../types";

/** Context for resolving cross-references. */
export interface XrefContext {
  /** MR/PRs known locally, keyed by number. */
  mrPrCache: Map<number, MrPr>;
  /** Issues known locally, keyed by number. */
  issueCache: Map<number, Issue>;
  /** Release tags known locally. */
  releaseTagCache: Set<string>;
  /** Called when the user clicks a MR/PR cross-reference. */
  onOpenMrPr: (number: number) => void;
  /** Called when the user clicks an issue cross-reference. */
  onOpenIssue: (number: number) => void;
  /** Called when the user clicks a release tag cross-reference. */
  onOpenRelease: (tag: string) => void;
  /** Called when the user clicks an external URL. */
  onOpenExternal: (url: string) => void;
}

/** One output segment of [`parseXrefs`]. */
export type XrefSegment =
  | { type: "text"; value: string }
  | { type: "mr_pr"; number: number; display: string }
  | { type: "issue"; number: number; display: string }
  | { type: "release"; tag: string; display: string }
  | { type: "external"; url: string; display: string };

// URLs first (longest), then version tags, then #N.
// Non-capturing groups would break the numeric indexing below — keep them
// capturing. Alternation ordering defines precedence: a version-looking
// number inside a URL still falls under the URL branch because URLs match
// first at that position.
const PATTERN =
  /(https?:\/\/[^\s<>"')]+)|(v?\d+\.\d+\.\d+(?:[.\-+][A-Za-z0-9.]+)?)|#(\d+)/g;

/**
 * Split `text` into an ordered list of plain-text + cross-reference segments.
 *
 * The caller decides how each segment renders — this function is pure and
 * does not touch the DOM.
 */
export function parseXrefs(text: string, ctx: XrefContext): XrefSegment[] {
  if (!text) return [];
  const segments: XrefSegment[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  // Reset since the regex is module-scoped with `g` flag.
  PATTERN.lastIndex = 0;

  while ((match = PATTERN.exec(text)) !== null) {
    const start = match.index;
    const full = match[0];

    if (start > lastIndex) {
      segments.push({ type: "text", value: text.slice(lastIndex, start) });
    }

    if (match[1]) {
      segments.push({ type: "external", url: match[1], display: match[1] });
    } else if (match[2]) {
      const tag = match[2];
      const withV = tag.startsWith("v") ? tag : `v${tag}`;
      if (ctx.releaseTagCache.has(tag)) {
        segments.push({ type: "release", tag, display: tag });
      } else if (ctx.releaseTagCache.has(withV)) {
        segments.push({ type: "release", tag: withV, display: tag });
      } else {
        segments.push({ type: "text", value: full });
      }
    } else if (match[3]) {
      const num = Number(match[3]);
      if (ctx.mrPrCache.has(num)) {
        segments.push({ type: "mr_pr", number: num, display: `#${num}` });
      } else if (ctx.issueCache.has(num)) {
        segments.push({ type: "issue", number: num, display: `#${num}` });
      } else {
        segments.push({ type: "text", value: full });
      }
    }

    lastIndex = start + full.length;
  }

  if (lastIndex < text.length) {
    segments.push({ type: "text", value: text.slice(lastIndex) });
  }

  return segments;
}
