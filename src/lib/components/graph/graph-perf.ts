/**
 * Graph rendering performance metrics collector.
 *
 * Reads `performance.measure()` entries produced by the instrumented
 * `renderGraph()` function and exposes them for the dev overlay.
 * Near-zero overhead in production — `performance.mark/measure` are
 * native browser primitives that cost ~0.001ms each.
 */

export interface RenderMetrics {
  /** Total render time in milliseconds. */
  totalMs: number;
  /** Time spent drawing lane segments. */
  lanesMs: number;
  /** Time spent drawing merge curves. */
  mergesMs: number;
  /** Time spent drawing commit nodes. */
  nodesMs: number;
  /** Time spent drawing ref/MR badges. */
  badgesMs: number;
  /** Time spent drawing text columns (summary, author, date, SHA). */
  textMs: number;
  /** Timestamp when this measurement was taken. */
  timestamp: number;
}

let lastMetrics: RenderMetrics | null = null;
const frameTimes: number[] = [];
const MAX_FRAME_HISTORY = 10;

/** Extract duration from a named performance measure, defaulting to 0. */
function getMeasureDuration(name: string): number {
  const entries = performance.getEntriesByName(name);
  return entries.length > 0 ? entries[entries.length - 1].duration : 0;
}

/**
 * Collect render metrics from performance entries.
 *
 * Call this at the end of each `renderGraph()` invocation, after all
 * `performance.measure()` calls have been made.
 */
export function recordRenderMetrics(): void {
  const metrics: RenderMetrics = {
    totalMs: getMeasureDuration('render:total'),
    lanesMs: getMeasureDuration('render:lanes'),
    mergesMs: getMeasureDuration('render:merges'),
    nodesMs: getMeasureDuration('render:nodes'),
    badgesMs: getMeasureDuration('render:badges'),
    textMs: getMeasureDuration('render:text'),
    timestamp: performance.now(),
  };

  lastMetrics = metrics;

  // Track frame timestamps for FPS calculation
  frameTimes.push(metrics.timestamp);
  if (frameTimes.length > MAX_FRAME_HISTORY) {
    frameTimes.shift();
  }

  // Clean up performance entries to prevent memory growth
  performance.clearMarks();
  performance.clearMeasures();
}

/** Return the most recent render metrics, or null if none recorded. */
export function getLastMetrics(): RenderMetrics | null {
  return lastMetrics;
}

/**
 * Compute rolling FPS from the last N frame timestamps.
 *
 * Returns 0 if fewer than 2 frames have been recorded.
 */
export function getRollingFps(): number {
  if (frameTimes.length < 2) return 0;
  const oldest = frameTimes[0];
  const newest = frameTimes[frameTimes.length - 1];
  const elapsed = newest - oldest;
  if (elapsed <= 0) return 0;
  return ((frameTimes.length - 1) / elapsed) * 1000;
}

/** Reset all collected metrics. Exposed for testing. */
export function resetPerfMetrics(): void {
  lastMetrics = null;
  frameTimes.length = 0;
}
