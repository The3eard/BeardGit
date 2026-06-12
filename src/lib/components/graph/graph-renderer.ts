/**
 * Canvas-based git graph renderer.
 *
 * Draws commit nodes, lane segments, merge curves, ref badges, and text
 * columns onto a 2D canvas context. Designed for virtual-scroll rendering
 * with 300-row viewport chunks — only visible rows are drawn.
 *
 * Key functions:
 * - `renderGraph` — main draw call, paints everything onto the canvas
 * - `graphHitTest` — determines what the user clicked (node, segment, empty)
 * - `computeMetrics` — calculates pixel dimensions from lane/row counts
 */

import type { LayoutNode, LaneSegment, MergeCurve, GraphTheme, MrPr } from "../../types";
import { formatRelativeTimeUnix } from "../../utils/time";
import { hashString as _hashString } from "../../utils/ref-colors";
import { shortOid } from "../../utils/git";
import { recordRenderMetrics } from "./graph-perf";

export const ROW_HEIGHT = 28;
export const LANE_WIDTH = 22;
export const TEXT_PADDING = 14;
export const REF_BADGE_HEIGHT = 16;
export const REF_BADGE_PADDING = 6;

/* beardgit:allow-hex: the canvas API needs concrete color strings, so the
 * pre-theme fallback reads the CSS custom properties from `:root` (which
 * mirror the default theme) and only falls back to these literals when the
 * tokens are unavailable (unit tests / detached documents). The runtime
 * theme overwrites all of this via GitGraph.svelte's graphTheme store. */
function cssToken(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  return value || fallback;
}

/** `rgba()` string from a `#RRGGBB` token value at the given alpha. */
function tokenAlpha(name: string, fallback: string, alpha: number): string {
  const hex = cssToken(name, fallback);
  if (!hex.startsWith("#") || hex.length < 7) return hex;
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * Pre-theme canvas fallback, built from the `:root` design tokens so the
 * first rendered frame matches the default theme's statics instead of
 * flashing a hardcoded palette until `applyTheme()` lands.
 */
export function defaultGraphTheme(): GraphTheme {
  const blue = cssToken("--accent-blue", "#58a6ff");
  const green = cssToken("--accent-green", "#3fb950");
  const orange = cssToken("--accent-orange", "#f0883e");
  const purple = cssToken("--accent-purple", "#bc8cff");
  const red = cssToken("--accent-red", "#f85149");
  const secondary = cssToken("--accent-secondary", "#bc8cff");
  return {
    background: cssToken("--bg-primary", "#0d1117"),
    currentLine: cssToken("--bg-secondary", "#161b22"),
    selection: cssToken("--selection", "#1c2333"),
    foreground: cssToken("--text-primary", "#c9d1d9"),
    comment: cssToken("--text-secondary", "#8b949e"),
    red,
    orange,
    yellow: orange,
    green,
    cyan: blue,
    purple,
    pink: secondary,
    laneColors: [
      cssToken("--graph-color-0", "#58a6ff"),
      cssToken("--graph-color-1", "#3fb950"),
      cssToken("--graph-color-2", "#f0883e"),
      cssToken("--graph-color-3", "#bb80ff"),
      cssToken("--graph-color-4", "#f778ba"),
      cssToken("--graph-color-5", "#79c0ff"),
    ],
    headLaneTint: tokenAlpha("--accent-primary", "#58a6ff", 0.04),
    dimOpacity: 0.3,
    selectionHighlight: tokenAlpha("--accent-blue", "#58a6ff", 0.08),
    nodeRadius: 5,
    mergeRadius: 6,
    refBadge: {
      branch: blue,
      remote: purple,
      tag: orange,
      head: secondary,
    },
    textPrimary: cssToken("--text-primary", "#c9d1d9"),
    textSecondary: cssToken("--text-secondary", "#8b949e"),
    textSha: orange,
    bisectGoodColor: tokenAlpha("--accent-green", "#3fb950", 0.15),
    bisectBadColor: tokenAlpha("--accent-red", "#f85149", 0.15),
    bisectSkipColor: tokenAlpha("--text-secondary", "#8b949e", 0.15),
    bisectCurrentColor: tokenAlpha("--accent-orange", "#e3b341", 0.15),
  };
}

// ── Canvas fonts ────────────────────────────────────────────────────────
// Deliberate typography: identifiers (SHA, dates, branch/tag badges) use
// the app's mono stack (Fira Code first — same as --font-mono) so the
// graph reads like a git tool; prose (commit summaries, authors) stays
// on the system sans stack.
const CANVAS_FONT_SANS = "-apple-system, BlinkMacSystemFont, sans-serif";
const CANVAS_FONT_MONO = "'Fira Code', 'SF Mono', 'Consolas', monospace";

// ── Column configuration ────────────────────────────────────────────────

export interface GraphColumn {
  id: string;
  label: string;
  width: number;
  visible: boolean;
}

export const DEFAULT_COLUMNS: GraphColumn[] = [
  { id: "author", label: "Author", width: 130, visible: true },
  { id: "date",   label: "Date",   width: 100, visible: true },
  { id: "email",  label: "Email",  width: 160, visible: false },
  { id: "sha",    label: "SHA",    width: 65,  visible: false },
];

// ── Helpers ─────────────────────────────────────────────────────────────

export function laneColor(lane: number, theme: GraphTheme): string {
  return theme.laneColors[lane % theme.laneColors.length];
}

export interface GraphMetrics {
  rowHeight: number;
  graphWidth: number;
  textStartX: number;
  totalHeight: number;
}

export function computeMetrics(laneCount: number, nodeCount: number): GraphMetrics {
  const graphWidth = Math.max((laneCount + 1) * LANE_WIDTH, 48);
  const textStartX = graphWidth + TEXT_PADDING;
  const totalHeight = nodeCount * ROW_HEIGHT;
  return { rowHeight: ROW_HEIGHT, graphWidth, textStartX, totalHeight };
}

function laneX(lane: number): number {
  return LANE_WIDTH + lane * LANE_WIDTH;
}

function rowY(row: number, offset: number): number {
  return (row - offset) * ROW_HEIGHT + ROW_HEIGHT / 2;
}


function withAlpha(color: string, alpha: number): string {
  // Only #RRGGBB literals can be split into channels. A user theme could
  // supply rgb()/hsl()/#RGB — parseInt would then yield NaN and produce an
  // invalid rgba() that the canvas ignores (the highlight/separator vanishes).
  // Fall back to the colour as-is so it still renders (just without alpha).
  if (!/^#[0-9a-f]{6}$/i.test(color)) return color;
  const r = parseInt(color.slice(1, 3), 16);
  const g = parseInt(color.slice(3, 5), 16);
  const b = parseInt(color.slice(5, 7), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

// ── MR/PR badge drawing ────────────────────────────────────────────────

/**
 * Draw a MR/PR badge pill on the graph canvas.
 *
 * Returns the total width consumed so subsequent badges can be offset.
 */
function drawMrPrBadge(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  number: number,
  isGitHub: boolean,
): number {
  const text = isGitHub ? `PR #${number}` : `MR !${number}`;
  const padding = 4;
  ctx.font = `10px ${CANVAS_FONT_MONO}`;
  const width = ctx.measureText(text).width + padding * 2;
  const height = 14;

  // Draw pill background — purple tint
  ctx.fillStyle = "rgba(137, 87, 229, 0.2)"; /* beardgit:allow-hex: canvas ctx.fillStyle requires concrete color */
  ctx.beginPath();
  roundRect(ctx, x, y - height / 2, width, height, 3);
  ctx.fill();

  // Draw border
  ctx.strokeStyle = "rgba(163, 113, 247, 0.4)"; /* beardgit:allow-hex: canvas ctx.strokeStyle requires concrete color */
  ctx.lineWidth = 1;
  ctx.stroke();

  // Draw text
  ctx.fillStyle = "#a371f7"; /* beardgit:allow-hex: canvas ctx.fillStyle requires concrete color */
  ctx.textBaseline = "middle";
  ctx.textAlign = "left";
  ctx.fillText(text, x + padding, y);

  return width + 4; // total width consumed including gap
}

// ── Main render ─────────────────────────────────────────────────────────

/**
 * Main graph render function — paints the entire visible viewport.
 *
 * Draws in order: HEAD lane tint, lane segments (with sync-state line styles),
 * merge curves, commit nodes, ref badges, MR/PR badges, and text columns
 * (summary, author, date, SHA). Supports group-based dimming for branch focus.
 */
export function renderGraph(
  ctx: CanvasRenderingContext2D,
  nodes: LayoutNode[],
  offset: number,
  canvasWidth: number,
  canvasHeight: number,
  laneCount: number,
  selectedOid: string | null,
  columns: GraphColumn[] = DEFAULT_COLUMNS,
  laneSegments: LaneSegment[] = [],
  mergeCurves: MergeCurve[] = [],
  theme: GraphTheme = defaultGraphTheme(),
  headLane: number | null = null,
  userEmails: string[] = [],
  selectedGroup: number | null = null,
  hoveredGroup: number | null = null,
  hoveredRow: number | null = null,
  mrPrByBranch: Map<string, MrPr> = new Map(),
  isGitHubProvider: boolean = false,
  bisectGood: Set<string> = new Set(),
  bisectBad: Set<string> = new Set(),
  bisectSkip: Set<string> = new Set(),
  bisectCurrent: string | null = null,
): void {
  ctx.clearRect(0, 0, canvasWidth, canvasHeight);
  performance.mark('render-start');

  // Opacity helpers for group-focus dimming
  // Returns a multiplier: 1.0 when no group is selected or group matches, dimOpacity otherwise
  function groupAlpha(groupId: number): number {
    return selectedGroup === null ? 1.0 : (groupId === selectedGroup ? 1.0 : theme.dimOpacity);
  }
  function setGroupOpacity(groupId: number) {
    ctx.globalAlpha = groupAlpha(groupId);
  }
  function resetOpacity() {
    ctx.globalAlpha = 1.0;
  }

  // HEAD lane background tint
  if (headLane !== null) {
    ctx.globalAlpha = selectedGroup === null ? 1.0 : 1.0; // HEAD tint always visible
    ctx.fillStyle = theme.headLaneTint;
    ctx.fillRect(
      laneX(headLane) - LANE_WIDTH / 2,
      0,
      LANE_WIDTH,
      canvasHeight
    );
    resetOpacity();
  }

  // Hovered group is handled during lane segment drawing — thicker + brighter line

  const metrics = computeMetrics(laneCount, nodes.length);

  // Calculate right-side columns total width
  const visibleCols = columns.filter((c) => c.visible);
  const rightColumnsWidth = visibleCols.reduce((sum, c) => sum + c.width, 0) + visibleCols.length * 12;

  // Draw hover highlight
  if (hoveredRow !== null) {
    const hoveredNode = nodes.find((n) => n.row === hoveredRow);
    if (hoveredNode && hoveredNode.oid !== selectedOid) {
      const y = rowY(hoveredRow, offset);
      ctx.fillStyle = withAlpha(theme.foreground, 0.04);
      ctx.fillRect(0, y - ROW_HEIGHT / 2, canvasWidth, ROW_HEIGHT);
    }
  }

  // Draw selection highlight
  if (selectedOid) {
    const selectedNode = nodes.find((n) => n.oid === selectedOid);
    if (selectedNode) {
      const y = rowY(selectedNode.row, offset);
      ctx.fillStyle = theme.selectionHighlight;
      ctx.fillRect(0, y - ROW_HEIGHT / 2, canvasWidth, ROW_HEIGHT);
    }
  }

  // ── Draw bisect overlays (row tints + current ring indicator) ──
  for (const node of nodes) {
    const y = rowY(node.row, offset);
    if (bisectGood.has(node.oid)) {
      ctx.fillStyle = theme.bisectGoodColor;
      ctx.fillRect(0, y - ROW_HEIGHT / 2, canvasWidth, ROW_HEIGHT);
    } else if (bisectBad.has(node.oid)) {
      ctx.fillStyle = theme.bisectBadColor;
      ctx.fillRect(0, y - ROW_HEIGHT / 2, canvasWidth, ROW_HEIGHT);
    } else if (bisectSkip.has(node.oid)) {
      ctx.fillStyle = theme.bisectSkipColor;
      ctx.fillRect(0, y - ROW_HEIGHT / 2, canvasWidth, ROW_HEIGHT);
    }
    if (node.oid === bisectCurrent) {
      ctx.fillStyle = theme.bisectCurrentColor;
      ctx.fillRect(0, y - ROW_HEIGHT / 2, canvasWidth, ROW_HEIGHT);
    }
  }

  // ── Build lookup sets for arrow suppression ──
  // Positions with visible commit nodes
  const nodePositions = new Set<string>();
  for (const node of nodes) {
    nodePositions.add(`${node.lane},${node.row}`);
  }
  // Positions connected by merge curves (no arrow needed there)
  // curveDepartsTo: a curve starts at from_row targeting to_lane → segment start is connected
  // curveArrivesAt: a curve arrives at (to_lane, to_row) → segment end is connected
  const curveDepartsTo = new Set<string>();
  const curveArrivesAt = new Set<string>();
  for (const curve of mergeCurves) {
    curveDepartsTo.add(`${curve.to_lane},${curve.from_row}`);
    curveArrivesAt.add(`${curve.to_lane},${curve.to_row}`);
  }

  // ── Draw lane segments (continuous vertical lines) ──
  performance.mark('lanes-start');
  const ARROW_SIZE = 4;
  for (const seg of laneSegments) {
    setGroupOpacity(seg.group_id);
    // Set line style based on hover, HEAD lane, and sync state
    const isHeadLane = headLane !== null && seg.lane === headLane;
    const isHovered = hoveredGroup !== null && seg.group_id === hoveredGroup && selectedGroup === null;
    if (isHovered) {
      // Hovered lane: thicker + brighter line (the line itself glows)
      ctx.lineWidth = 3.5;
      ctx.setLineDash(seg.sync_state === "RemoteOnly" ? [4, 3] : seg.sync_state === "LocalOnly" ? [6, 3] : []);
    } else if (isHeadLane) {
      ctx.lineWidth = 3;
      ctx.setLineDash(seg.sync_state === "RemoteOnly" ? [4, 3] : seg.sync_state === "LocalOnly" ? [6, 3] : []);
    } else {
      switch (seg.sync_state) {
        case "LocalOnly":
          ctx.lineWidth = 2;
          ctx.setLineDash([6, 3]);
          break;
        case "RemoteOnly":
          ctx.lineWidth = 1.2;
          ctx.setLineDash([4, 3]);
          break;
        default: // "Synced" | "Unknown"
          ctx.lineWidth = 2;
          ctx.setLineDash([]);
          break;
      }
    }
    const x = laneX(seg.lane);
    const rawY1 = rowY(seg.start_row, offset);
    const rawY2 = rowY(seg.end_row, offset);
    const y1 = Math.max(rawY1, -ROW_HEIGHT);
    const y2 = Math.min(rawY2, canvasHeight + ROW_HEIGHT);

    if (y1 > canvasHeight + ROW_HEIGHT || y2 < -ROW_HEIGHT) continue;

    const hasNodeAtStart = nodePositions.has(`${seg.lane},${seg.start_row}`);
    const hasNodeAtEnd = nodePositions.has(`${seg.lane},${seg.end_row}`);
    const hasCurveAtStart = curveDepartsTo.has(`${seg.lane},${seg.start_row}`);
    const hasCurveAtEnd = curveArrivesAt.has(`${seg.lane},${seg.end_row}`);

    // If a merge curve targets this lane from the segment's start row,
    // clip the segment to start where the S-curve arrives at this lane.
    // This prevents orphaned line pieces above the curve's arrival point.
    let drawY1 = y1;
    if (hasCurveAtStart) {
      const curve = mergeCurves.find(
        (c) => c.to_lane === seg.lane && c.from_row === seg.start_row
      );
      if (curve) {
        const cy1 = rowY(curve.from_row, offset);
        const cy2 = rowY(curve.to_row, offset);
        const dist = Math.abs(cy2 - cy1);
        const ch = Math.min(ROW_HEIGHT * 2.5, dist * 0.6);
        const arrivalY = cy1 + ROW_HEIGHT * 0.3 + ch;
        drawY1 = Math.max(drawY1, arrivalY);
      }
    }

    // Skip segment if the clipped range is empty
    if (drawY1 >= y2) continue;

    const color = laneColor(seg.color_index, theme);
    const la = groupAlpha(seg.group_id);
    ctx.strokeStyle = color;
    ctx.globalAlpha = 0.85 * la;
    ctx.beginPath();
    ctx.moveTo(x, drawY1);
    ctx.lineTo(x, y2);
    ctx.stroke();

    // Draw ▲ arrow at top if no node AND no curve connection at start
    if (!hasNodeAtStart && !hasCurveAtStart && rawY1 >= 0 && rawY1 < canvasHeight) {
      ctx.fillStyle = color;
      ctx.globalAlpha = 0.6 * la;
      ctx.beginPath();
      ctx.moveTo(x, rawY1 - ARROW_SIZE * 1.2);
      ctx.lineTo(x - ARROW_SIZE, rawY1 + ARROW_SIZE * 0.5);
      ctx.lineTo(x + ARROW_SIZE, rawY1 + ARROW_SIZE * 0.5);
      ctx.closePath();
      ctx.fill();
    }
    // Or arrow at viewport edge if segment extends above
    if (rawY1 < 0) {
      ctx.fillStyle = color;
      ctx.globalAlpha = 0.5 * la;
      ctx.beginPath();
      ctx.moveTo(x, 2);
      ctx.lineTo(x - ARROW_SIZE, 2 + ARROW_SIZE * 1.5);
      ctx.lineTo(x + ARROW_SIZE, 2 + ARROW_SIZE * 1.5);
      ctx.closePath();
      ctx.fill();
    }

    // Draw ▼ arrow at bottom if recycled (lane was reclaimed — branch continues
    // further down in a different lane) or if no node/curve connection at end.
    if (seg.recycled && rawY2 >= 0 && rawY2 < canvasHeight) {
      // Recycled indicator: slightly larger, more opaque arrow
      ctx.fillStyle = color;
      ctx.globalAlpha = 0.8 * la;
      ctx.beginPath();
      ctx.moveTo(x, rawY2 + ARROW_SIZE * 1.5);
      ctx.lineTo(x - ARROW_SIZE * 1.2, rawY2 - ARROW_SIZE * 0.3);
      ctx.lineTo(x + ARROW_SIZE * 1.2, rawY2 - ARROW_SIZE * 0.3);
      ctx.closePath();
      ctx.fill();
    } else if (!hasNodeAtEnd && !hasCurveAtEnd && rawY2 >= 0 && rawY2 < canvasHeight) {
      ctx.fillStyle = color;
      ctx.globalAlpha = 0.6 * la;
      ctx.beginPath();
      ctx.moveTo(x, rawY2 + ARROW_SIZE * 1.2);
      ctx.lineTo(x - ARROW_SIZE, rawY2 - ARROW_SIZE * 0.5);
      ctx.lineTo(x + ARROW_SIZE, rawY2 - ARROW_SIZE * 0.5);
      ctx.closePath();
      ctx.fill();
    }
    // Or arrow at viewport edge if segment extends below
    if (rawY2 > canvasHeight) {
      ctx.fillStyle = color;
      ctx.globalAlpha = 0.5 * la;
      ctx.beginPath();
      ctx.moveTo(x, canvasHeight - 2);
      ctx.lineTo(x - ARROW_SIZE, canvasHeight - 2 - ARROW_SIZE * 1.5);
      ctx.lineTo(x + ARROW_SIZE, canvasHeight - 2 - ARROW_SIZE * 1.5);
      ctx.closePath();
      ctx.fill();
    }
    // Reset line dash and opacity after each segment
    ctx.setLineDash([]);
    resetOpacity();
  }
  ctx.globalAlpha = 1.0;
  ctx.lineWidth = 2;
  ctx.setLineDash([]);
  performance.mark('lanes-end');
  performance.measure('render:lanes', 'lanes-start', 'lanes-end');

  // ── Draw merge curves (cross-lane S-curves) ──
  performance.mark('merges-start');
  ctx.lineWidth = 2;
  for (const curve of mergeCurves) {
    const x1 = laneX(curve.from_lane);
    const y1 = rowY(curve.from_row, offset);
    const x2 = laneX(curve.to_lane);
    const y2 = rowY(curve.to_row, offset);

    const minY = Math.min(y1, y2);
    const maxY = Math.max(y1, y2);
    if (maxY < -ROW_HEIGHT || minY > canvasHeight + ROW_HEIGHT) continue;

    const curveVisible = selectedGroup === null || curve.group_id === selectedGroup;
    const curveAlpha = curveVisible ? 1.0 : theme.dimOpacity;
    ctx.strokeStyle = laneColor(curve.color_index, theme);
    ctx.globalAlpha = 0.85 * curveAlpha;
    ctx.beginPath();

    const clampedY1 = Math.max(y1, -ROW_HEIGHT * 2);
    const clampedY2 = Math.min(y2, canvasHeight + ROW_HEIGHT * 2);

    const distance = Math.abs(clampedY2 - clampedY1);
    const curveHeight = Math.min(ROW_HEIGHT * 2.5, distance * 0.6);
    const curveStartY = clampedY1 + ROW_HEIGHT * 0.3;
    const curveEndY = curveStartY + curveHeight;

    ctx.moveTo(x1, clampedY1);
    ctx.lineTo(x1, curveStartY);
    ctx.bezierCurveTo(
      x1, curveStartY + curveHeight * 0.5,
      x2, curveEndY - curveHeight * 0.5,
      x2, curveEndY
    );
    if (clampedY2 > curveEndY) {
      ctx.lineTo(x2, clampedY2);
    }

    ctx.stroke();
    resetOpacity();
  }
  performance.mark('merges-end');
  performance.measure('render:merges', 'merges-start', 'merges-end');

  // Draw nodes — with background halo to clear lane lines behind them
  performance.mark('nodes-start');
  for (const node of nodes) {
    const x = laneX(node.lane);
    const y = rowY(node.row, offset);
    const color = laneColor(node.lane, theme);
    const isSelected = node.oid === selectedOid;

    // Background halo — clears lane lines behind the node for visibility
    ctx.beginPath();
    ctx.arc(x, y, (node.is_merge ? theme.mergeRadius : theme.nodeRadius) + 2, 0, Math.PI * 2);
    ctx.fillStyle = theme.background;
    ctx.fill();

    setGroupOpacity(node.segment_group);
    ctx.beginPath();
    if (node.is_merge) {
      // Merge node: hollow circle with thick border
      ctx.arc(x, y, theme.mergeRadius, 0, Math.PI * 2);
      ctx.strokeStyle = color;
      ctx.lineWidth = 2.5;
      ctx.fillStyle = isSelected ? color : theme.background;
      ctx.fill();
      ctx.stroke();
    } else {
      // Regular node: solid filled circle
      const radius = isSelected ? theme.nodeRadius + 1.5 : theme.nodeRadius;
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();
    }

    // Bisect current commit — draw a yellow ring around the node
    if (node.oid === bisectCurrent) {
      ctx.beginPath();
      const ringRadius = (node.is_merge ? theme.mergeRadius : theme.nodeRadius) + 3;
      ctx.arc(x, y, ringRadius, 0, Math.PI * 2);
      ctx.strokeStyle = theme.bisectCurrentColor.replace(/[\d.]+\)$/, "0.8)");
      ctx.lineWidth = 2.5;
      ctx.stroke();
      ctx.lineWidth = 2;
    }
    ctx.lineWidth = 2;
    resetOpacity();
  }
  performance.mark('nodes-end');
  performance.measure('render:nodes', 'nodes-start', 'nodes-end');

  // ── Column layout calculation ──
  performance.mark('badges-start');
  // Right columns are fixed-width, drawn right-to-left.
  // The "message" area (refs + summary) gets whatever space remains.
  const textX = metrics.textStartX;
  const COL_GAP = 12;

  // Build column positions right-to-left
  const colPositions: { col: GraphColumn; startX: number; endX: number }[] = [];
  {
    let x = canvasWidth;
    for (let i = visibleCols.length - 1; i >= 0; i--) {
      const col = visibleCols[i];
      const endX = x;
      const startX = x - col.width;
      colPositions.unshift({ col, startX, endX });
      x = startX - COL_GAP;
    }
  }

  // Message area: from graph end to the first right column
  const messageEndX = colPositions.length > 0
    ? colPositions[0].startX - COL_GAP
    : canvasWidth - 8;

  // Draw thin vertical separators between message area and each column pair
  if (colPositions.length > 0) {
    ctx.strokeStyle = withAlpha(theme.comment, 0.15);
    ctx.lineWidth = 1;

    // Separator between message area and first column
    ctx.beginPath();
    ctx.moveTo(messageEndX + COL_GAP / 2, 0);
    ctx.lineTo(messageEndX + COL_GAP / 2, canvasHeight);
    ctx.stroke();

    // Separators between each column pair
    for (let i = 0; i < colPositions.length - 1; i++) {
      const sepX = colPositions[i].endX + COL_GAP / 2;
      ctx.beginPath();
      ctx.moveTo(sepX, 0);
      ctx.lineTo(sepX, canvasHeight);
      ctx.stroke();
    }
  }

  // ── Draw rows ──
  for (const node of nodes) {
    setGroupOpacity(node.segment_group);
    const y = rowY(node.row, offset);
    const isSelected = node.oid === selectedOid;
    const rowTop = y - ROW_HEIGHT / 2;
    let currentX = textX;

    // ── Ref badges (clipped to message area) ──
    if (node.refs.length > 0) {
      ctx.save();
      ctx.beginPath();
      ctx.rect(textX, rowTop, messageEndX - textX, ROW_HEIGHT);
      ctx.clip();

      ctx.font = `11px ${CANVAS_FONT_MONO}`;
      ctx.textBaseline = "middle";

      for (const ref of node.refs) {
        const label = formatRef(ref);
        const badgeColor = refColor(ref, theme);
        const textWidth = ctx.measureText(label).width;
        const badgeWidth = textWidth + REF_BADGE_PADDING * 2;

        // Stop drawing badges if they'd overflow
        if (currentX + badgeWidth > messageEndX - 40) break;

        ctx.fillStyle = badgeColor + "22";
        ctx.strokeStyle = badgeColor + "66";
        ctx.lineWidth = 1;
        roundRect(ctx, currentX, y - REF_BADGE_HEIGHT / 2, badgeWidth, REF_BADGE_HEIGHT, 3);
        ctx.fill();
        ctx.stroke();

        ctx.fillStyle = badgeColor;
        ctx.fillText(label, currentX + REF_BADGE_PADDING, y);
        currentX += badgeWidth + 4;
      }

      ctx.restore();
    }

    // ── MR/PR badges (after ref badges, before summary) ──
    if (mrPrByBranch.size > 0 && node.refs.length > 0) {
      for (const ref of node.refs) {
        const branchName = formatRef(ref);
        const mrPr = mrPrByBranch.get(branchName);
        if (mrPr && currentX + 60 < messageEndX) {
          ctx.save();
          ctx.beginPath();
          ctx.rect(textX, rowTop, messageEndX - textX, ROW_HEIGHT);
          ctx.clip();
          const badgeW = drawMrPrBadge(ctx, currentX, y, mrPr.number, isGitHubProvider);
          currentX += badgeW;
          ctx.restore();
          break; // only one MR/PR badge per commit
        }
      }
    }

    // ── Commit summary (clipped to remaining message area) ──
    const summaryX = currentX;
    const maxSummaryWidth = messageEndX - summaryX;

    if (node.summary && maxSummaryWidth > 20) {
      ctx.save();
      ctx.beginPath();
      ctx.rect(summaryX, rowTop, maxSummaryWidth, ROW_HEIGHT);
      ctx.clip();

      const isMyCommit = userEmails.length > 0 &&
        (userEmails.includes(node.email.toLowerCase()) || userEmails.includes(node.author.toLowerCase()));
      ctx.font = isMyCommit
        ? `bold 13px ${CANVAS_FONT_SANS}`
        : `13px ${CANVAS_FONT_SANS}`;
      ctx.fillStyle = isSelected ? "#ffffff" : theme.textPrimary; /* beardgit:allow-hex: canvas requires concrete color; selected row inverts text */
      ctx.textBaseline = "middle";
      ctx.textAlign = "left";
      const summary = truncateText(ctx, node.summary, maxSummaryWidth - 4);
      ctx.fillText(summary, summaryX, y);

      ctx.restore();
    }

    // ── Right columns (each clipped to its own bounds) ──
    for (const { col, startX, endX } of colPositions) {
      ctx.save();
      ctx.beginPath();
      ctx.rect(startX, rowTop, endX - startX, ROW_HEIGHT);
      ctx.clip();

      let text = "";
      let style = isSelected ? theme.textPrimary : theme.textSecondary;
      let font = `12px ${CANVAS_FONT_SANS}`;

      switch (col.id) {
        case "sha":
          text = shortOid(node.oid);
          font = `12px ${CANVAS_FONT_MONO}`;
          style = isSelected ? theme.textPrimary : theme.textSha;
          break;
        case "author":
          text = node.author || "";
          break;
        case "date":
          text = node.timestamp ? formatRelativeTimeUnix(node.timestamp) : "";
          font = `11px ${CANVAS_FONT_MONO}`;
          break;
        case "email":
          text = node.email || "";
          break;
      }

      ctx.font = font;
      ctx.fillStyle = style;
      ctx.textBaseline = "middle";
      ctx.textAlign = "right";
      // Truncate text to fit column width
      const truncated = truncateText(ctx, text, col.width - 4);
      ctx.fillText(truncated, endX - 4, y);

      ctx.restore();
    }
    resetOpacity();
  }
  performance.mark('badges-end');
  performance.measure('render:badges', 'badges-start', 'badges-end');
  // Text is currently drawn inside the badges loop; when the loop is split
  // this becomes a standalone measure. Until then, keep the alias but name
  // it accurately so the HUD doesn't claim two separate timings.
  performance.measure('render:badges-and-text', 'badges-start', 'badges-end');

  performance.mark('render-end');
  performance.measure('render:total', 'render-start', 'render-end');
  recordRenderMetrics();
}

// ── Utility functions ───────────────────────────────────────────────────

function truncateText(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string {
  if (ctx.measureText(text).width <= maxWidth) return text;
  let lo = 0;
  let hi = text.length;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (ctx.measureText(text.slice(0, mid) + "\u2026").width <= maxWidth) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }
  return lo > 0 ? text.slice(0, lo) + "\u2026" : "\u2026";
}

function formatRef(ref: string): string {
  if (ref.startsWith("refs/heads/")) return ref.replace("refs/heads/", "");
  if (ref.startsWith("refs/remotes/")) return ref.replace("refs/remotes/", "");
  if (ref.startsWith("refs/tags/")) return ref.replace("refs/tags/", "");
  if (ref === "HEAD") return "HEAD";
  return ref;
}

/** Delegates to the shared ref-colors utility for consistent hashing. */
const hashString = _hashString;

function refColor(ref: string, theme: GraphTheme): string {
  if (ref === "HEAD") return theme.refBadge.head;
  const label = formatRef(ref);
  const colors = theme.laneColors;
  return colors[hashString(label) % colors.length];
}

function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number, y: number, width: number, height: number, radius: number
): void {
  ctx.beginPath();
  ctx.moveTo(x + radius, y);
  ctx.lineTo(x + width - radius, y);
  ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
  ctx.lineTo(x + width, y + height - radius);
  ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  ctx.lineTo(x + radius, y + height);
  ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
  ctx.lineTo(x, y + radius);
  ctx.quadraticCurveTo(x, y, x + radius, y);
  ctx.closePath();
}

export function hitTest(
  y: number, offset: number, nodeCount: number
): number | null {
  const row = Math.floor(y / ROW_HEIGHT) + offset;
  if (row < offset || row >= offset + nodeCount) return null;
  return row;
}

// ── Enhanced hit testing with lane/segment awareness ────────────────────

export interface GraphHitResult {
  type: "node" | "segment" | "empty";
  row?: number;
  groupId?: number;
}

/**
 * Determines what the user clicked: a commit node, a lane segment, or empty space.
 * Node clicks take priority over segment clicks when the click is near a node's lane.
 * Segment hits use `group_id` so recycled lanes highlight only the correct branch.
 */
export function graphHitTest(
  x: number,
  y: number,
  offset: number,
  nodes: LayoutNode[],
  laneCount: number,
  laneSegments: LaneSegment[] = [],
): GraphHitResult {
  const row = Math.floor(y / ROW_HEIGHT) + offset;
  const node = nodes.find((n) => n.row === row);

  // Check if click is near a node's lane column
  if (node) {
    const nx = laneX(node.lane);
    if (Math.abs(x - nx) <= LANE_WIDTH / 2) {
      return { type: "node", row };
    }
  }

  // Check if click is on a lane segment — find the actual segment at this position
  for (const seg of laneSegments) {
    const lx = laneX(seg.lane);
    if (Math.abs(x - lx) <= LANE_WIDTH / 2 && row >= seg.start_row && row <= seg.end_row) {
      return { type: "segment", groupId: seg.group_id };
    }
  }

  // Click on text area — treat as node click if a node exists at this row
  if (node) {
    return { type: "node", row };
  }

  return { type: "empty" };
}

/**
 * Check if a mouse X position is near a column separator for resize.
 * Returns the index into the visible columns array of the column whose
 * left edge is being dragged, or -1 if not near any separator.
 */
export function getResizeTarget(
  mouseX: number,
  columns: GraphColumn[],
  canvasWidth: number,
): number {
  const RESIZE_ZONE = 4;
  const COL_GAP = 12;
  const visibleCols = columns.filter(c => c.visible);

  // Build column positions right-to-left (same as renderGraph)
  let x = canvasWidth;
  const positions: { startX: number; colIndex: number }[] = [];
  for (let i = visibleCols.length - 1; i >= 0; i--) {
    const startX = x - visibleCols[i].width;
    positions.unshift({ startX, colIndex: i });
    x = startX - COL_GAP;
  }

  // Check if mouse is near the separator line (midpoint of gap before column)
  for (const pos of positions) {
    const sepX = pos.startX - COL_GAP / 2;
    if (Math.abs(mouseX - sepX) <= RESIZE_ZONE) {
      return pos.colIndex;
    }
  }

  return -1;
}
