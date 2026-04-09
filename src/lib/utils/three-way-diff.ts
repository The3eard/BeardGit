/**
 * Three-way merge diff engine.
 *
 * Provides LCS-based line-level diffing, three-way chunk classification,
 * and auto-merged result building for the merge editor.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** A half-open range of lines within a document (0-based). */
export interface LineRange {
  start: number;
  count: number;
}

/** Classification of a chunk in a three-way merge result. */
export type ChunkKind = "unchanged" | "theirs_only" | "ours_only" | "conflict";

/** A single chunk produced by a three-way merge. */
export interface MergeChunk {
  kind: ChunkKind;
  baseRange: LineRange;
  theirsRange: LineRange;
  oursRange: LineRange;
}

/** A single edit produced by a two-way line diff. */
export interface DiffEdit {
  type: "equal" | "insert" | "delete";
  oldLines: string[];
  newLines: string[];
  oldStart: number;
  newStart: number;
}

// ---------------------------------------------------------------------------
// LCS + diffLines (unchanged from before)
// ---------------------------------------------------------------------------

export function lcsTable(a: string[], b: string[]): number[][] {
  const m = a.length;
  const n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (a[i - 1] === b[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }
  return dp;
}

type RawOp = { type: "equal" | "insert" | "delete"; oldIdx: number; newIdx: number; line: string };

/** Compute a line-level diff between two documents using LCS. */
export function diffLines(oldLines: string[], newLines: string[]): DiffEdit[] {
  if (oldLines.length === 0 && newLines.length === 0) return [];
  const dp = lcsTable(oldLines, newLines);
  const raw: RawOp[] = [];
  let i = oldLines.length;
  let j = newLines.length;
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && oldLines[i - 1] === newLines[j - 1]) {
      raw.push({ type: "equal", oldIdx: i - 1, newIdx: j - 1, line: oldLines[i - 1] });
      i--; j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      raw.push({ type: "insert", oldIdx: i, newIdx: j - 1, line: newLines[j - 1] });
      j--;
    } else {
      raw.push({ type: "delete", oldIdx: i - 1, newIdx: j, line: oldLines[i - 1] });
      i--;
    }
  }
  raw.reverse();
  const edits: DiffEdit[] = [];
  for (const op of raw) {
    const last = edits[edits.length - 1];
    if (last && last.type === op.type) {
      if (op.type === "equal" || op.type === "delete") last.oldLines.push(op.line);
      if (op.type === "equal" || op.type === "insert") last.newLines.push(op.line);
    } else {
      edits.push({
        type: op.type,
        oldLines: op.type === "equal" || op.type === "delete" ? [op.line] : [],
        newLines: op.type === "equal" || op.type === "insert" ? [op.line] : [],
        oldStart: op.oldIdx,
        newStart: op.newIdx,
      });
    }
  }
  return edits;
}

// ---------------------------------------------------------------------------
// Three-way diff — alignment-based approach
// ---------------------------------------------------------------------------

function splitLines(text: string): string[] {
  if (text === "") return [];
  return text.split("\n");
}

/**
 * Build aligned line arrays from a two-way diff.
 *
 * For each base line, produces the corresponding new-document line(s).
 * Returns an array of length baseLen where each entry is:
 *   null = this base line is unchanged (maps 1:1 to new doc)
 *   string[] = this base line was replaced by these new lines (can be empty = deleted)
 *
 * Also returns insertions that happen BEFORE base line i (pure inserts).
 */
function buildAlignment(
  edits: DiffEdit[],
  baseLen: number,
): { changes: (string[] | null)[]; insertsBefore: Map<number, string[]> } {
  const changes: (string[] | null)[] = new Array(baseLen).fill(null);
  const insertsBefore = new Map<number, string[]>();

  for (let ei = 0; ei < edits.length; ei++) {
    const edit = edits[ei];
    if (edit.type === "equal") continue;

    if (edit.type === "delete") {
      // Check if followed by an insert (= replacement)
      const next = edits[ei + 1];
      if (next && next.type === "insert") {
        // Mark all deleted base lines, attach replacement to the first
        for (let k = 0; k < edit.oldLines.length; k++) {
          changes[edit.oldStart + k] = k === 0 ? next.newLines : [];
        }
        ei++; // skip the insert
      } else {
        for (let k = 0; k < edit.oldLines.length; k++) {
          changes[edit.oldStart + k] = [];
        }
      }
    } else {
      // Pure insert at position edit.oldStart
      const existing = insertsBefore.get(edit.oldStart) ?? [];
      existing.push(...edit.newLines);
      insertsBefore.set(edit.oldStart, existing);
    }
  }

  return { changes, insertsBefore };
}

/**
 * Perform a three-way diff/merge classification.
 *
 * Uses per-base-line alignment to correctly detect overlapping changes.
 */
export function threeWayDiff(base: string, theirs: string, ours: string): MergeChunk[] {
  const baseLines = splitLines(base);
  const theirsLines = splitLines(theirs);
  const oursLines = splitLines(ours);

  if (baseLines.length === 0) {
    if (theirsLines.length === 0 && oursLines.length === 0) return [];
    if (theirsLines.length > 0 && oursLines.length > 0) {
      return [{ kind: "conflict", baseRange: { start: 0, count: 0 }, theirsRange: { start: 0, count: theirsLines.length }, oursRange: { start: 0, count: oursLines.length } }];
    }
    if (theirsLines.length > 0) {
      return [{ kind: "theirs_only", baseRange: { start: 0, count: 0 }, theirsRange: { start: 0, count: theirsLines.length }, oursRange: { start: 0, count: 0 } }];
    }
    return [{ kind: "ours_only", baseRange: { start: 0, count: 0 }, theirsRange: { start: 0, count: 0 }, oursRange: { start: 0, count: oursLines.length } }];
  }

  const theirEdits = diffLines(baseLines, theirsLines);
  const ourEdits = diffLines(baseLines, oursLines);
  const tAlign = buildAlignment(theirEdits, baseLines.length);
  const oAlign = buildAlignment(ourEdits, baseLines.length);

  // Classify each base line
  type LineClass = "unchanged" | "theirs" | "ours" | "both";
  const classification: LineClass[] = new Array(baseLines.length);
  for (let i = 0; i < baseLines.length; i++) {
    const tChanged = tAlign.changes[i] !== null;
    const oChanged = oAlign.changes[i] !== null;
    if (tChanged && oChanged) classification[i] = "both";
    else if (tChanged) classification[i] = "theirs";
    else if (oChanged) classification[i] = "ours";
    else classification[i] = "unchanged";
  }

  // Expand: if a "theirs" line is adjacent to a "both" or "ours" group that
  // overlaps the same edit, merge them into "both" (conflict).
  // Simple approach: find contiguous runs of non-unchanged and check if both
  // sides are represented.
  let i = 0;
  while (i < baseLines.length) {
    if (classification[i] === "unchanged") { i++; continue; }
    // Start of a changed region
    const start = i;
    let hasTheirs = false;
    let hasOurs = false;
    while (i < baseLines.length && classification[i] !== "unchanged") {
      if (classification[i] === "theirs" || classification[i] === "both") hasTheirs = true;
      if (classification[i] === "ours" || classification[i] === "both") hasOurs = true;
      i++;
    }
    // If both sides touched this contiguous region, it's a conflict
    if (hasTheirs && hasOurs) {
      for (let j = start; j < i; j++) classification[j] = "both";
    }
  }

  // Build chunks by grouping consecutive same-class lines
  const chunks: MergeChunk[] = [];
  let bi = 0;
  let ti = 0;
  let oi = 0;

  /** Emit insert chunks at position `pos`. */
  function emitInserts(pos: number) {
    const tIns = tAlign.insertsBefore.get(pos);
    const oIns = oAlign.insertsBefore.get(pos);
    if (tIns && oIns) {
      chunks.push({ kind: "conflict", baseRange: { start: pos, count: 0 }, theirsRange: { start: ti, count: tIns.length }, oursRange: { start: oi, count: oIns.length } });
      ti += tIns.length;
      oi += oIns.length;
    } else if (tIns) {
      chunks.push({ kind: "theirs_only", baseRange: { start: pos, count: 0 }, theirsRange: { start: ti, count: tIns.length }, oursRange: { start: oi, count: 0 } });
      ti += tIns.length;
    } else if (oIns) {
      chunks.push({ kind: "ours_only", baseRange: { start: pos, count: 0 }, theirsRange: { start: ti, count: 0 }, oursRange: { start: oi, count: oIns.length } });
      oi += oIns.length;
    }
  }

  emitInserts(0);

  while (bi < baseLines.length) {
    const cls = classification[bi];

    if (cls === "unchanged") {
      const start = bi;
      const tStart = ti;
      const oStart = oi;
      while (bi < baseLines.length && classification[bi] === "unchanged") {
        bi++; ti++; oi++;
        emitInserts(bi);
      }
      chunks.push({ kind: "unchanged", baseRange: { start, count: bi - start }, theirsRange: { start: tStart, count: ti - tStart }, oursRange: { start: oStart, count: oi - oStart } });
    } else if (cls === "theirs") {
      const start = bi;
      const tStart = ti;
      const oStart = oi;
      while (bi < baseLines.length && classification[bi] === "theirs") {
        const rep = tAlign.changes[bi];
        if (rep !== null && bi === start) {
          ti += rep.length; // replacement lines
        } else if (rep !== null && rep.length === 0) {
          // deleted, no new lines
        }
        oi++; // ours unchanged for this base line
        bi++;
        emitInserts(bi);
      }
      // Recalculate theirs count properly
      let theirsCount = 0;
      for (let j = start; j < bi; j++) {
        const rep = tAlign.changes[j];
        if (rep !== null) theirsCount += rep.length;
        else theirsCount++; // unchanged on this side
      }
      // Reset ti to correct position
      ti = tStart + theirsCount;
      chunks.push({ kind: "theirs_only", baseRange: { start, count: bi - start }, theirsRange: { start: tStart, count: theirsCount }, oursRange: { start: oStart, count: bi - start } });
    } else if (cls === "ours") {
      const start = bi;
      const tStart = ti;
      const oStart = oi;
      while (bi < baseLines.length && classification[bi] === "ours") {
        ti++; // theirs unchanged
        bi++;
        emitInserts(bi);
      }
      let oursCount = 0;
      for (let j = start; j < bi; j++) {
        const rep = oAlign.changes[j];
        if (rep !== null) oursCount += rep.length;
        else oursCount++;
      }
      oi = oStart + oursCount;
      chunks.push({ kind: "ours_only", baseRange: { start, count: bi - start }, theirsRange: { start: tStart, count: bi - start }, oursRange: { start: oStart, count: oursCount } });
    } else {
      // Conflict ("both")
      const start = bi;
      const tStart = ti;
      const oStart = oi;
      while (bi < baseLines.length && classification[bi] === "both") {
        bi++;
      }
      // Count replacement lines for each side
      let theirsCount = 0;
      for (let j = start; j < bi; j++) {
        const rep = tAlign.changes[j];
        if (rep !== null) theirsCount += rep.length;
        else theirsCount++;
      }
      let oursCount = 0;
      for (let j = start; j < bi; j++) {
        const rep = oAlign.changes[j];
        if (rep !== null) oursCount += rep.length;
        else oursCount++;
      }
      ti = tStart + theirsCount;
      oi = oStart + oursCount;
      chunks.push({ kind: "conflict", baseRange: { start, count: bi - start }, theirsRange: { start: tStart, count: theirsCount }, oursRange: { start: oStart, count: oursCount } });
      emitInserts(bi);
    }
  }

  // Trailing inserts after last base line
  emitInserts(baseLines.length);

  // Merge adjacent chunks of the same kind
  const merged: MergeChunk[] = [];
  for (const chunk of chunks) {
    const last = merged[merged.length - 1];
    if (last && last.kind === chunk.kind) {
      last.baseRange.count += chunk.baseRange.count;
      last.theirsRange.count += chunk.theirsRange.count;
      last.oursRange.count += chunk.oursRange.count;
    } else {
      merged.push({ kind: chunk.kind, baseRange: { ...chunk.baseRange }, theirsRange: { ...chunk.theirsRange }, oursRange: { ...chunk.oursRange } });
    }
  }

  return merged;
}

// ---------------------------------------------------------------------------
// buildMergedResult
// ---------------------------------------------------------------------------

/** Build the merged output from classified chunks. */
export function buildMergedResult(
  chunks: MergeChunk[],
  baseLines: string[],
  theirsLines: string[],
  oursLines: string[],
  conflictPlaceholder: (index: number) => string,
): string {
  const output: string[] = [];
  let conflictIndex = 0;
  for (const chunk of chunks) {
    switch (chunk.kind) {
      case "unchanged":
        output.push(...baseLines.slice(chunk.baseRange.start, chunk.baseRange.start + chunk.baseRange.count));
        break;
      case "theirs_only":
        output.push(...theirsLines.slice(chunk.theirsRange.start, chunk.theirsRange.start + chunk.theirsRange.count));
        break;
      case "ours_only":
        output.push(...oursLines.slice(chunk.oursRange.start, chunk.oursRange.start + chunk.oursRange.count));
        break;
      case "conflict":
        output.push(conflictPlaceholder(conflictIndex++));
        break;
    }
  }
  return output.join("\n");
}
