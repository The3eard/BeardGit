/**
 * Conflict placeholders for the merge editor's result document.
 *
 * An unresolved conflict occupies one line of the result document, the
 * placeholder. The per-side decision (accept / discard) is encoded in that
 * line's text rather than kept in component state, so a single CodeMirror
 * transaction carries both the inserted lines and the new state, and undo
 * restores both. Everything the editor knows about a conflict's progress is
 * recoverable by scanning the document.
 *
 * Format:  `◆ CONFLICT <index>` while both sides are pending,
 *          `◆ CONFLICT <index> [theirs:<decision> ours:<decision>]` otherwise.
 */

import type { ChangeSpec, Text } from "@codemirror/state";

export const CONFLICT_PREFIX = "◆ CONFLICT ";

export type ConflictSide = "theirs" | "ours";
export type SideDecision = "pending" | "accepted" | "discarded";

export interface ConflictMarker {
  index: number;
  theirs: SideDecision;
  ours: SideDecision;
}

const DECISIONS: readonly SideDecision[] = ["pending", "accepted", "discarded"];

export function pendingMarker(index: number): ConflictMarker {
  return { index, theirs: "pending", ours: "pending" };
}

export function formatPlaceholder(marker: ConflictMarker): string {
  const head = `${CONFLICT_PREFIX}${marker.index}`;
  if (marker.theirs === "pending" && marker.ours === "pending") return head;
  return `${head} [theirs:${marker.theirs} ours:${marker.ours}]`;
}

/** Parse a placeholder line; `null` when the line is ordinary content. */
export function parsePlaceholder(text: string): ConflictMarker | null {
  if (!text.startsWith(CONFLICT_PREFIX)) return null;
  const m = /^(\d+)(?: \[theirs:(\w+) ours:(\w+)\])?$/.exec(
    text.slice(CONFLICT_PREFIX.length).trim(),
  );
  if (!m) return null;
  const theirs = (m[2] ?? "pending") as SideDecision;
  const ours = (m[3] ?? "pending") as SideDecision;
  if (!DECISIONS.includes(theirs) || !DECISIONS.includes(ours)) return null;
  return { index: parseInt(m[1], 10), theirs, ours };
}

export interface PlaceholderHit {
  marker: ConflictMarker;
  /** 1-based line number in the document. */
  lineNumber: number;
}

/** Every placeholder line in the document, in document order. */
export function findPlaceholders(doc: Text): PlaceholderHit[] {
  const hits: PlaceholderHit[] = [];
  for (let n = 1; n <= doc.lines; n++) {
    const marker = parsePlaceholder(doc.line(n).text);
    if (marker) hits.push({ marker, lineNumber: n });
  }
  return hits;
}

/** True while any conflict placeholder remains in the content. */
export function hasPlaceholders(content: string): boolean {
  return content.includes(CONFLICT_PREFIX);
}

/**
 * Changes that record a decision for one side of a conflict.
 *
 * Accepted lines are inserted above the placeholder, so accepting both
 * sides yields theirs followed by ours in click order. The placeholder is
 * rewritten with the new state while the other side is still pending and
 * removed, newline included, once both sides are decided. Returns `null`
 * when the placeholder is gone or that side is already decided.
 */
export function decideSide(
  doc: Text,
  index: number,
  side: ConflictSide,
  decision: "accepted" | "discarded",
  sideLines: string[],
): ChangeSpec[] | null {
  const hit = findPlaceholders(doc).find((h) => h.marker.index === index);
  if (!hit || hit.marker[side] !== "pending") return null;

  const line = doc.line(hit.lineNumber);
  const next: ConflictMarker = { ...hit.marker, [side]: decision };
  const changes: ChangeSpec[] = [];

  if (decision === "accepted" && sideLines.length > 0) {
    changes.push({ from: line.from, insert: sideLines.join("\n") + "\n" });
  }

  const otherSide: ConflictSide = side === "theirs" ? "ours" : "theirs";
  if (next[otherSide] === "pending") {
    changes.push({ from: line.from, to: line.to, insert: formatPlaceholder(next) });
  } else if (line.to < doc.length) {
    changes.push({ from: line.from, to: line.to + 1 });
  } else if (line.from > 0) {
    changes.push({ from: line.from - 1, to: line.to });
  } else {
    changes.push({ from: line.from, to: line.to });
  }
  return changes;
}
