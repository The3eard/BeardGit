import { describe, it, expect } from "vitest";
import { ROW_HEIGHT, LANE_WIDTH, curveBend, defaultGraphTheme, refColor, renderGraph } from "./graph-renderer";

/**
 * A recording 2D context: every method is a no-op that logs its name and the
 * `strokeStyle` / `globalAlpha` in force, `measureText` returns a width so
 * the text code paths run. Enough to assert what each stroke was drawn with.
 */
function recordingContext() {
  const strokes: { style: string; alpha: number; kind: string; x: number; y1: number; y2: number }[] = [];
  const state: Record<string, unknown> = { strokeStyle: "", fillStyle: "", globalAlpha: 1 };
  let lastPath = "";
  let x = 0, y1 = 0, y2 = 0;
  const ctx = new Proxy(state, {
    get(target, prop: string) {
      if (prop in target) return target[prop];
      if (prop === "measureText") return () => ({ width: 10 });
      if (prop === "moveTo") return (mx: number, my: number) => { x = mx; y1 = my; y2 = my; };
      if (prop === "bezierCurveTo") return () => { lastPath = "curve"; };
      if (prop === "lineTo") return (_lx: number, ly: number) => { y2 = ly; if (lastPath !== "curve") lastPath = "line"; };
      if (prop === "arc") return () => { lastPath = "arc"; };
      if (prop === "beginPath") return () => { lastPath = ""; };
      if (prop === "stroke") {
        return () => {
          strokes.push({
            style: String(target.strokeStyle),
            alpha: Number(target.globalAlpha),
            kind: lastPath,
            x, y1, y2,
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
const curve = (from_lane: number, from_row: number, to_lane: number, to_row: number, group_id: number, opens_lane = false) => ({
  from_lane, from_row, to_lane, to_row, color_index: from_lane, group_id, opens_lane,
});

describe("renderGraph merge curves", () => {
  // m (lane 0, row 0) merges b2 (lane 1, row 2); lane 1 opens at row 0 for it.
  // b2's parent is base (lane 0, row 3): lane 1 closes at row 2.
  const nodes = [node("m", 0, 0, 0, true), node("b1", 0, 1, 0), node("b2", 1, 2, 1), node("base", 0, 3, 0)];
  const segments = [seg(0, 0, 3, 0), seg(1, 0, 2, 1)];
  const curves = [curve(0, 0, 1, 2, 0, true), curve(1, 2, 0, 3, 1)];
  const theme = defaultGraphTheme();

  it("keeps a first-parent edge into a lane opened for another parent in the child's colour", () => {
    // m (lane 0, row 0) merges p2 (lane 1, row 1) and reaches p1 (lane 1,
    // row 3) as first parent. Only the p2 edge opens lane 1; the p1 edge
    // must bend at the bottom in lane 0's colour, or m's line ends nowhere.
    const twoParents = [node("m", 0, 0, 0, true), node("p2", 1, 1, 1), node("y", 0, 2, 0), node("p1", 1, 3, 1)];
    const segs = [seg(0, 0, 2, 0), seg(1, 0, 3, 1)];
    const cs = [curve(0, 0, 1, 1, 0, true), curve(0, 0, 1, 3, 0)];
    const { ctx, strokes } = recordingContext();
    renderGraph(ctx, twoParents, 0, 600, 200, 2, null, [], segs, cs, theme);
    const curveStrokes = strokes.filter((s) => s.kind === "curve");
    expect(curveStrokes.map((s) => s.style)).toEqual([theme.laneColors[1], theme.laneColors[0]]);
  });

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

describe("renderGraph segment ends under a departing curve", () => {
  // Tip t on lane 2 (row 1) whose parent u sits on lane 0 (row 3). Lane 2's
  // segment closes at row 2; the curve (2,1)→(0,3) spans two lanes, so its
  // bend starts above row 2's centre. The straight run must stop there.
  const nodes = [node("h", 0, 0, 0), node("t", 2, 1, 1), node("f", 0, 2, 0), node("u", 0, 3, 0)];
  const segments = [seg(0, 0, 3, 0), seg(2, 1, 2, 1)];
  const curves = [curve(2, 1, 0, 3, 1)];

  it("clips the segment where the curve begins to bend, so no spike pokes out", () => {
    const { ctx, strokes } = recordingContext();
    renderGraph(ctx, nodes, 0, 600, 200, 3, null, [], segments, curves, defaultGraphTheme());
    const laneX2 = LANE_WIDTH + 2 * LANE_WIDTH;
    const lane2 = strokes.find((s) => s.kind === "line" && s.x === laneX2);
    expect(lane2).toBeDefined();
    const rowCentre = (r: number) => r * ROW_HEIGHT + ROW_HEIGHT / 2;
    const bend = curveBend(laneX2, rowCentre(1), LANE_WIDTH, rowCentre(3));
    expect(bend).toBeGreaterThan(ROW_HEIGHT); // the case that used to spike
    expect(lane2!.y2).toBeCloseTo(rowCentre(3) - bend);
    expect(lane2!.y2).toBeLessThan(rowCentre(2));
  });
});

describe("refColor", () => {
  it("colours a badge by ref kind from the theme, never by name hash", () => {
    const theme = defaultGraphTheme();
    expect(refColor("refs/heads/feat/x", theme)).toBe(theme.refBadge.branch);
    expect(refColor("refs/remotes/origin/feat/x", theme)).toBe(theme.refBadge.remote);
    expect(refColor("refs/tags/v1.0", theme)).toBe(theme.refBadge.tag);
    expect(refColor("HEAD", theme)).toBe(theme.refBadge.head);
    // Two branches → same colour: the kind is the signal, not the name.
    expect(refColor("refs/heads/a", theme)).toBe(refColor("refs/heads/b", theme));
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
