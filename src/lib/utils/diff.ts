/**
 * diff.ts — Utilities for converting parsed diff structures back to raw content.
 *
 * Used when the backend provides pre-parsed `FileDiff` objects (e.g. stash diffs)
 * and the frontend needs raw old/new content strings for the CodeMirror DiffEditor.
 */

import type { FileDiff } from "../types";

/**
 * Reconstructs old and new file content from a `FileDiff` hunk list.
 *
 * Each `DiffLineInfo` has an `origin` field:
 *   - `' '` (space) — context line present in both old and new
 *   - `'-'` — line only in old content (removed)
 *   - `'+'` — line only in new content (added)
 *
 * Lines are reassembled in order to produce two complete content strings.
 */
export function fileDiffToContents(diff: FileDiff): { oldContent: string; newContent: string } {
  const oldLines: string[] = [];
  const newLines: string[] = [];

  for (const hunk of diff.hunks) {
    for (const line of hunk.lines) {
      if (line.origin === " ") {
        oldLines.push(line.content);
        newLines.push(line.content);
      } else if (line.origin === "-") {
        oldLines.push(line.content);
      } else if (line.origin === "+") {
        newLines.push(line.content);
      }
    }
  }

  return {
    oldContent: oldLines.join("\n"),
    newContent: newLines.join("\n"),
  };
}
