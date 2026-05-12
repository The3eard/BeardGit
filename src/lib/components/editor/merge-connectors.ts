/**
 * SVG bezier connectors between merge editor panels.
 *
 * Draws curved paths connecting conflict regions on side panels to
 * conflict placeholder lines in the center panel. Automatically
 * switches to simplified thin lines when conflicts are dense.
 */

const SVG_NS = 'http://www.w3.org/2000/svg';

/** Minimum vertical gap (px) between two connectors before switching to simplified mode. */
const DENSE_THRESHOLD = 20;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Vertical range of a conflict region in a panel (in pixels). */
export interface RegionRect {
  top: number;
  bottom: number;
}

/** A pair of regions to connect: one on a side panel, one on the center. */
export interface ConnectorPair {
  side: RegionRect;
  center: RegionRect;
  resolved: boolean;
}

// ---------------------------------------------------------------------------
// renderConnectors
// ---------------------------------------------------------------------------

/**
 * Render connector paths into an SVG element.
 *
 * When conflicts are sparse (> 20px apart), draws filled bezier curves.
 * When dense, switches to thin straight connector lines to stay readable.
 *
 * @param svg       - The SVG element to render into.
 * @param pairs     - Connector pairs to draw.
 * @param width     - Width of the SVG element (px).
 * @param direction - "left": side on left, center on right.
 *                    "right": center on left, side on right.
 */
export function renderConnectors(
  svg: SVGSVGElement,
  pairs: ConnectorPair[],
  width: number,
  direction: 'left' | 'right',
): void {
  while (svg.firstChild) svg.removeChild(svg.firstChild);

  // Filter out resolved pairs and pairs with invalid coordinates
  // (coordsAtPos returns null for off-screen lines → {top:0, bottom:0})
  const active = pairs.filter(p =>
    !p.resolved &&
    !(p.side.top === 0 && p.side.bottom === 0) &&
    !(p.center.top === 0 && p.center.bottom === 0)
  );
  if (active.length === 0) return;

  // Detect density: check minimum gap between consecutive connectors
  const dense = isDense(active);

  if (dense) {
    renderSimplified(svg, active, width, direction);
  } else {
    renderBezier(svg, active, width, direction);
  }
}

/** Check if connectors are too close together for bezier curves. */
function isDense(pairs: ConnectorPair[]): boolean {
  if (pairs.length <= 1) return false;

  // Sort by side top position
  const sorted = [...pairs].sort((a, b) => a.side.top - b.side.top);
  for (let i = 1; i < sorted.length; i++) {
    const gap = sorted[i].side.top - sorted[i - 1].side.bottom;
    if (gap < DENSE_THRESHOLD) return true;
  }

  // Also check center positions
  const sortedCenter = [...pairs].sort((a, b) => a.center.top - b.center.top);
  for (let i = 1; i < sortedCenter.length; i++) {
    const gap = sortedCenter[i].center.top - sortedCenter[i - 1].center.bottom;
    if (gap < DENSE_THRESHOLD) return true;
  }

  return false;
}

/** Render full bezier curves with filled areas (sparse mode). */
function renderBezier(
  svg: SVGSVGElement,
  pairs: ConnectorPair[],
  width: number,
  direction: 'left' | 'right',
): void {
  const cp = width * 0.5;

  for (const pair of pairs) {
    const sideX = direction === 'left' ? 0 : width;
    const centerX = direction === 'left' ? width : 0;
    const cp1X = direction === 'left' ? cp : width - cp;
    const cp2X = direction === 'left' ? width - cp : cp;

    const { top: sideTop, bottom: sideBot } = pair.side;
    const { top: centerTop, bottom: centerBot } = pair.center;

    // Filled area
    const d = [
      `M ${sideX} ${sideTop}`,
      `C ${cp1X} ${sideTop}, ${cp2X} ${centerTop}, ${centerX} ${centerTop}`,
      `L ${centerX} ${centerBot}`,
      `C ${cp2X} ${centerBot}, ${cp1X} ${sideBot}, ${sideX} ${sideBot}`,
      'Z',
    ].join(' ');

    const fillPath = document.createElementNS(SVG_NS, 'path');
    fillPath.setAttribute('d', d);
    fillPath.style.fill = 'var(--accent-primary)';
    fillPath.style.fillOpacity = '0.06';
    fillPath.style.stroke = 'none';
    svg.appendChild(fillPath);

    // Top edge stroke
    const topCurve = document.createElementNS(SVG_NS, 'path');
    topCurve.setAttribute('d',
      `M ${sideX} ${sideTop} C ${cp1X} ${sideTop}, ${cp2X} ${centerTop}, ${centerX} ${centerTop}`);
    topCurve.style.fill = 'none';
    topCurve.style.stroke = 'var(--accent-primary)';
    topCurve.style.strokeOpacity = '0.3';
    topCurve.style.strokeWidth = '1';
    svg.appendChild(topCurve);

    // Bottom edge stroke
    const botCurve = document.createElementNS(SVG_NS, 'path');
    botCurve.setAttribute('d',
      `M ${sideX} ${sideBot} C ${cp1X} ${sideBot}, ${cp2X} ${centerBot}, ${centerX} ${centerBot}`);
    botCurve.style.fill = 'none';
    botCurve.style.stroke = 'var(--accent-primary)';
    botCurve.style.strokeOpacity = '0.3';
    botCurve.style.strokeWidth = '1';
    svg.appendChild(botCurve);
  }
}

/** Render simplified thin lines connecting midpoints (dense mode). */
function renderSimplified(
  svg: SVGSVGElement,
  pairs: ConnectorPair[],
  width: number,
  direction: 'left' | 'right',
): void {
  const sideX = direction === 'left' ? 0 : width;
  const centerX = direction === 'left' ? width : 0;

  for (const pair of pairs) {
    const sideMid = (pair.side.top + pair.side.bottom) / 2;
    const centerMid = (pair.center.top + pair.center.bottom) / 2;

    // Single thin line connecting midpoints
    const line = document.createElementNS(SVG_NS, 'line');
    line.setAttribute('x1', String(sideX));
    line.setAttribute('y1', String(sideMid));
    line.setAttribute('x2', String(centerX));
    line.setAttribute('y2', String(centerMid));
    line.style.stroke = 'var(--accent-primary)';
    line.style.strokeOpacity = '0.35';
    line.style.strokeWidth = '1.5';
    svg.appendChild(line);

    // Small tick marks at the endpoints to show the range
    const tickLen = 3;

    // Side tick (top to bottom of region)
    const sideTick = document.createElementNS(SVG_NS, 'line');
    sideTick.setAttribute('x1', String(sideX));
    sideTick.setAttribute('y1', String(pair.side.top));
    sideTick.setAttribute('x2', String(sideX));
    sideTick.setAttribute('y2', String(pair.side.bottom));
    sideTick.style.stroke = 'var(--accent-primary)';
    sideTick.style.strokeOpacity = '0.25';
    sideTick.style.strokeWidth = '2';
    svg.appendChild(sideTick);

    // Center tick
    const centerTick = document.createElementNS(SVG_NS, 'line');
    centerTick.setAttribute('x1', String(centerX));
    centerTick.setAttribute('y1', String(centerMid - tickLen));
    centerTick.setAttribute('x2', String(centerX));
    centerTick.setAttribute('y2', String(centerMid + tickLen));
    centerTick.style.stroke = 'var(--accent-primary)';
    centerTick.style.strokeOpacity = '0.25';
    centerTick.style.strokeWidth = '2';
    svg.appendChild(centerTick);
  }
}

// ---------------------------------------------------------------------------
// getLineRect
// ---------------------------------------------------------------------------

/**
 * Get the pixel Y range of a line range using actual DOM coordinates.
 *
 * @param view       - CodeMirror EditorView.
 * @param fromLine   - 0-based start line index.
 * @param lineCount  - Number of lines in the range.
 * @param refTop     - The top Y of the reference container (connector gap).
 * @returns RegionRect with top/bottom relative to refTop.
 */
export function getLineRect(
  view: {
    coordsAtPos: (pos: number) => { top: number; bottom: number } | null;
    state: { doc: { line: (n: number) => { from: number; to: number }; lines: number } };
  },
  fromLine: number,
  lineCount: number,
  refTop: number,
): RegionRect {
  const totalLines = view.state.doc.lines;

  const firstLine = Math.max(1, Math.min(fromLine + 1, totalLines));
  const lastLine = Math.max(1, Math.min(fromLine + lineCount, totalLines));

  const firstPos = view.state.doc.line(firstLine).from;
  const lastPos = view.state.doc.line(lastLine).from;

  const topCoord = view.coordsAtPos(firstPos);
  const botCoord = view.coordsAtPos(lastPos);

  if (!topCoord || !botCoord) {
    return { top: 0, bottom: 0 };
  }

  return {
    top: topCoord.top - refTop,
    bottom: botCoord.bottom - refTop,
  };
}
