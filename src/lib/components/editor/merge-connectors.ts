/**
 * SVG bezier connectors between merge editor panels.
 *
 * Draws curved bands connecting a conflict region on a side panel to the
 * region of the result panel it feeds. Coordinates may lie outside the
 * visible gap; the SVG clips them, so a block that is half scrolled away
 * still connects to what remains on screen. Switches to thin lines when
 * conflicts are dense.
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
  /** The conflict the user is working on; drawn stronger. */
  active?: boolean;
}

// ---------------------------------------------------------------------------
// renderConnectors
// ---------------------------------------------------------------------------

/**
 * Render connector paths into an SVG element.
 *
 * @param svg       - The SVG element to render into.
 * @param pairs     - Connector pairs to draw; resolved and off-screen ones are skipped.
 * @param width     - Width of the SVG element (px).
 * @param height    - Height of the SVG element (px); pairs entirely outside are skipped.
 * @param direction - "left": side on left, center on right.
 *                    "right": center on left, side on right.
 */
export function renderConnectors(
  svg: SVGSVGElement,
  pairs: ConnectorPair[],
  width: number,
  height: number,
  direction: 'left' | 'right',
): void {
  while (svg.firstChild) svg.removeChild(svg.firstChild);

  const active = pairs.filter((p) => {
    if (p.resolved) return false;
    const top = Math.min(p.side.top, p.center.top);
    const bottom = Math.max(p.side.bottom, p.center.bottom);
    return bottom >= 0 && top <= height;
  });
  if (active.length === 0) return;

  if (isDense(active)) {
    renderSimplified(svg, active, width, direction);
  } else {
    renderBezier(svg, active, width, direction);
  }
}

/** Check if connectors are too close together for bezier curves. */
function isDense(pairs: ConnectorPair[]): boolean {
  if (pairs.length <= 1) return false;

  const sorted = [...pairs].sort((a, b) => a.side.top - b.side.top);
  for (let i = 1; i < sorted.length; i++) {
    const gap = sorted[i].side.top - sorted[i - 1].side.bottom;
    if (gap < DENSE_THRESHOLD) return true;
  }

  const sortedCenter = [...pairs].sort((a, b) => a.center.top - b.center.top);
  for (let i = 1; i < sortedCenter.length; i++) {
    const gap = sortedCenter[i].center.top - sortedCenter[i - 1].center.bottom;
    if (gap < DENSE_THRESHOLD) return true;
  }

  return false;
}

function strokeOpacity(pair: ConnectorPair, base: number): string {
  return String(pair.active ? Math.min(1, base * 2) : base);
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
    fillPath.style.fillOpacity = pair.active ? '0.14' : '0.06';
    fillPath.style.stroke = 'none';
    svg.appendChild(fillPath);

    for (const [a, b] of [[sideTop, centerTop], [sideBot, centerBot]]) {
      const curve = document.createElementNS(SVG_NS, 'path');
      curve.setAttribute('d', `M ${sideX} ${a} C ${cp1X} ${a}, ${cp2X} ${b}, ${centerX} ${b}`);
      curve.style.fill = 'none';
      curve.style.stroke = 'var(--accent-primary)';
      curve.style.strokeOpacity = strokeOpacity(pair, 0.3);
      curve.style.strokeWidth = pair.active ? '1.5' : '1';
      svg.appendChild(curve);
    }
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

    const line = document.createElementNS(SVG_NS, 'line');
    line.setAttribute('x1', String(sideX));
    line.setAttribute('y1', String(sideMid));
    line.setAttribute('x2', String(centerX));
    line.setAttribute('y2', String(centerMid));
    line.style.stroke = 'var(--accent-primary)';
    line.style.strokeOpacity = strokeOpacity(pair, 0.35);
    line.style.strokeWidth = pair.active ? '2' : '1.5';
    svg.appendChild(line);

    const tickLen = 3;

    const sideTick = document.createElementNS(SVG_NS, 'line');
    sideTick.setAttribute('x1', String(sideX));
    sideTick.setAttribute('y1', String(pair.side.top));
    sideTick.setAttribute('x2', String(sideX));
    sideTick.setAttribute('y2', String(pair.side.bottom));
    sideTick.style.stroke = 'var(--accent-primary)';
    sideTick.style.strokeOpacity = strokeOpacity(pair, 0.25);
    sideTick.style.strokeWidth = '2';
    svg.appendChild(sideTick);

    const centerTick = document.createElementNS(SVG_NS, 'line');
    centerTick.setAttribute('x1', String(centerX));
    centerTick.setAttribute('y1', String(centerMid - tickLen));
    centerTick.setAttribute('x2', String(centerX));
    centerTick.setAttribute('y2', String(centerMid + tickLen));
    centerTick.style.stroke = 'var(--accent-primary)';
    centerTick.style.strokeOpacity = strokeOpacity(pair, 0.25);
    centerTick.style.strokeWidth = '2';
    svg.appendChild(centerTick);
  }
}
