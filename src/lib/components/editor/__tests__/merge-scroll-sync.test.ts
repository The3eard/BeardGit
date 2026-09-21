import { describe, it, expect } from "vitest";
import { centerChunkStartLines, mapScrollOffset, sideChunkStartLines } from "../merge-scroll-sync";
import type { MergeChunk } from "$lib/utils/three-way-diff";

function chunk(kind: MergeChunk["kind"], base: number, theirs: number, ours: number): MergeChunk {
  return {
    kind,
    baseRange: { start: 0, count: base },
    theirsRange: { start: 0, count: theirs },
    oursRange: { start: 0, count: ours },
  };
}

describe("mapScrollOffset", () => {
  // Source: a 1-line placeholder (20px) between two 100px blocks.
  // Destination: the same conflict is a 3200px block.
  const src = [0, 100, 120, 220];
  const dst = [0, 100, 3300, 3400];

  it("carries the fraction inside a chunk across", () => {
    expect(mapScrollOffset(src, dst, 100)).toBe(100);
    expect(mapScrollOffset(src, dst, 110)).toBe(1700);
    expect(mapScrollOffset(src, dst, 120)).toBe(3300);
    expect(mapScrollOffset(dst, src, 1700)).toBe(110);
  });

  it("keeps offsets in unchanged chunks identical", () => {
    expect(mapScrollOffset(src, dst, 50)).toBe(50);
    expect(mapScrollOffset(src, dst, 170)).toBe(3350);
  });

  it("extends linearly past both ends", () => {
    expect(mapScrollOffset(src, dst, -10)).toBe(-10);
    expect(mapScrollOffset(src, dst, 250)).toBe(3430);
  });

  it("lands on the chunk start when the source chunk has no height", () => {
    expect(mapScrollOffset([0, 100, 100, 200], [0, 100, 500, 600], 100)).toBe(500);
  });
});

describe("chunk start lines", () => {
  const chunks = [
    chunk("unchanged", 5, 5, 5),
    chunk("conflict", 2, 160, 80),
    chunk("theirs_only", 0, 3, 0),
    chunk("unchanged", 4, 4, 4),
  ];

  it("collapses a conflict to its single placeholder line in the result", () => {
    expect(centerChunkStartLines(chunks)).toEqual([0, 5, 6, 9, 13]);
  });

  it("uses each side's own line counts", () => {
    expect(sideChunkStartLines(chunks, "theirs")).toEqual([0, 5, 165, 168, 172]);
    expect(sideChunkStartLines(chunks, "ours")).toEqual([0, 5, 85, 85, 89]);
  });
});
