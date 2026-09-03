import { describe, it, expect } from "vitest";
import { ROW_HEIGHT, LANE_WIDTH, curveBend, defaultGraphTheme, renderGraph } from "./graph-renderer";

/**
 * A recording 2D context: every method is a no-op that logs its name and the
 * `strokeStyle` / `globalAlpha` in force, `measureText` returns a width so
 * the text code paths run. Enough to assert what each stroke was drawn with.
 */
function recordingContext() {
  const strokes: { style: string; alpha: number; kind: string }[] = [];
  const state: Record<string, unknown> = { strokeStyle: "", fillStyle: "", globalAlpha: 1 };
  let lastPath = "";
  const ctx = new Proxy(state, {
    get(target, prop: string) {
      if (prop in target) return target[prop];
      if (prop === "measureText") return () => ({ width: 10 });
      if (prop === "bezierCurveTo") return () => { lastPath = "curve"; };
      if (prop === "lineTo") return () => { if (lastPath !== "curve") lastPath = "line"; };
      if (prop === "arc") return () => { lastPath = "arc"; };
      if (prop === "beginPath") return () => { lastPath = ""; };
      if (prop === "stroke") {
        return () => {
          strokes.push({
            style: String(target.strokeStyle),
            alpha: Number(target.globalAlpha),
            kind: lastPath,
          });
        };
      }
      return () => undefined;
    },
    set(target, prop: string, value) {
      target[prop] = value;
      return true;
    },
  }) as unknown as CanvasRenderingContext2D;
  return { ctx, strokes };
}

const node = (oid: string, lane: number, row: number, segment_group: number, is_merge = false) => ({
  oid, lane, row, segment_group, is_merge, is_root: false,
  refs: [] as string[], summary: oid, author: "a", email: "a@x", timestamp: 0,
});
const seg = (lane: number, start_row: number, end_row: number, group_id: number) => ({
  lane, start_row, end_row, color_index: lane, recycled: false, sync_state: "Unknown" as const, group_id,
});
const curve = (from_lane: number, from_row: number, to_lane: number, to_row: number, group_id: number) => ({
  from_lane, from_row, to_lane, to_row, color_index: from_lane, group_id,
});

describe("renderGraph merge curves", () => {
  // m (lane 0, row 0) merges b2 (lane 1, row 2); lane 1 opens at row 0 for it.
  // b2's parent is base (lane 0, row 3): lane 1 closes at row 2.
  const nodes = [node("m", 0, 0, 0, true), node("b1", 0, 1, 0), node("b2", 1, 2, 1), node("base", 0, 3, 0)];
  const segments = [seg(0, 0, 3, 0), seg(1, 0, 2, 1)];
  const curves = [curve(0, 0, 1, 2, 0), curve(1, 2, 0, 3, 1)];
  const theme = defaultGraphTheme();

  it("draws the curve that opens a lane in that lane's colour, not the child's", () => {
    const { ctx, strokes } = recordingContext();
    renderGraph(ctx, nodes, 0, 600, 200, 2, null, [], segments, curves, theme);
    const curveStrokes = strokes.filter((s) => s.kind === "curve");
    expect(curveStrokes).toHaveLength(2);
    // Curve 0 hands off to lane 1's segment → lane 1's colour. Curve 1 is a
    // branch rejoining the mainline → its own (lane 1) colour.
    expect(curveStrokes[0].style).toBe(theme.laneColors[1]);
    expect(curveStrokes[1].style).toBe(theme.laneColors[1]);
  });

  it("dims the lane-opening curve with the lane's group, not the child's", () => {
    const { ctx, strokes } = recordingContext();
    // Select group 0 (the mainline): lane 1's line, including its opening
    // bend, must be dimmed together.
    renderGraph(ctx, nodes, 0, 600, 200, 2, null, [], segments, curves, theme, null, [], 0);
    const curveStrokes = strokes.filter((s) => s.kind === "curve");
    expect(curveStrokes[0].alpha).toBeCloseTo(0.85 * theme.dimOpacity);
  });
});

describe("curveBend", () => {
  // The segment clip and the curve drawing both call this; the two used to
  // compute the arrival point differently, leaving a stub on the parent's
  // lane above where the curve actually landed.
  it("turns within one row for a one-lane hop", () => {
    expect(curveBend(0, 0, LANE_WIDTH, ROW_HEIGHT * 5)).toBe(ROW_HEIGHT);
  });
  it("eases over more rows for a wide hop, capped by the vertical span", () => {
    const wide = curveBend(0, 0, LANE_WIDTH * 6, ROW_HEIGHT * 10);
    expect(wide).toBeGreaterThan(ROW_HEIGHT);
    expect(wide).toBeLessThanOrEqual(ROW_HEIGHT * 10);
    // Adjacent rows: never more than the span allows (one row).
    expect(curveBend(0, 0, LANE_WIDTH * 6, ROW_HEIGHT)).toBe(ROW_HEIGHT);
  });
});

describe("graph-renderer bisect theme defaults", () => {
  it("has bisect color fields in default theme", () => {
    const theme = defaultGraphTheme();
    expect(theme.bisectGoodColor).toContain("63, 185, 80");
    expect(theme.bisectBadColor).toContain("248, 81, 73");
    expect(theme.bisectSkipColor).toContain("139, 148, 158");
    expect(theme.bisectCurrentColor).toContain("227, 179, 65");
  });
});
