/**
 * Chunk-aligned scroll mapping for the merge editor.
 *
 * Each panel is described by the pixel offset at which every chunk starts,
 * plus one trailing entry for the end of the document. A scroll offset in
 * one panel is located inside its chunk and carried over to the same
 * fraction of the corresponding chunk in another panel, so a block that is
 * 160 lines on one side and one placeholder line on the other scrolls
 * through smoothly instead of jumping across.
 */

import type { MergeChunk } from "$lib/utils/three-way-diff";

/**
 * Map a scroll offset from one panel to another.
 *
 * @param srcTops  Chunk start offsets of the source panel, length chunks + 1.
 * @param dstTops  Same for the destination panel.
 * @param y        Scroll offset in the source panel.
 */
export function mapScrollOffset(srcTops: number[], dstTops: number[], y: number): number {
  const n = Math.min(srcTops.length, dstTops.length) - 1;
  if (n < 1) return y;
  if (y <= srcTops[0]) return dstTops[0] + (y - srcTops[0]);
  if (y >= srcTops[n]) return dstTops[n] + (y - srcTops[n]);

  let k = 0;
  while (k < n - 1 && srcTops[k + 1] <= y) k++;
  const srcSpan = srcTops[k + 1] - srcTops[k];
  const fraction = srcSpan > 0 ? (y - srcTops[k]) / srcSpan : 0;
  return dstTops[k] + fraction * (dstTops[k + 1] - dstTops[k]);
}

/**
 * 0-based start line of every chunk in the freshly built result document,
 * plus the total line count as a trailing entry. A conflict contributes its
 * single placeholder line.
 */
export function centerChunkStartLines(chunks: MergeChunk[]): number[] {
  const starts: number[] = [];
  let line = 0;
  for (const chunk of chunks) {
    starts.push(line);
    switch (chunk.kind) {
      case "unchanged": line += chunk.baseRange.count; break;
      case "theirs_only": line += chunk.theirsRange.count; break;
      case "ours_only": line += chunk.oursRange.count; break;
      case "conflict": line += 1; break;
    }
  }
  starts.push(line);
  return starts;
}

/** 0-based start line of every chunk on a side panel, plus the line total. */
export function sideChunkStartLines(chunks: MergeChunk[], side: "theirs" | "ours"): number[] {
  const starts: number[] = [];
  let line = 0;
  for (const chunk of chunks) {
    starts.push(line);
    line += side === "theirs" ? chunk.theirsRange.count : chunk.oursRange.count;
  }
  starts.push(line);
  return starts;
}
