/**
 * Windowed-list arithmetic, shared by every list that mounts one row per
 * item.
 *
 * This was inlined in `common/List.svelte`, which is a whole panel —
 * header, filter bar, refresh button, single selection. The lists that
 * actually grow without bound are not panels of that shape: the changes
 * list has checkboxes and shift-range selection, the workdir tree has
 * expandable directories. They could not consume the component, so they
 * rendered every row. Extracting the arithmetic is what lets them share
 * the behaviour without inheriting the layout.
 *
 * Deliberately pure: no runes, no DOM. Each consumer keeps its own
 * `$state` for `scrollTop` / `viewportHeight` and calls this from a
 * `$derived`, which also makes the edge cases testable without a browser.
 */

/** The slice of items to mount, plus the height the scrollbar should see. */
export interface VirtualWindow {
  /** First index to mount (inclusive). */
  start: number;
  /** Last index to mount (exclusive). */
  end: number;
  /** Height of the full list in px — the sizer keeps the scrollbar honest. */
  totalHeight: number;
}

export interface VirtualWindowInput {
  /** Number of items in the list *after* filtering. */
  count: number;
  /**
   * Pixel height of one row. Virtualization needs a uniform row height;
   * `undefined` opts out, which is how a consumer with variable-height
   * rows stays on the plain render path.
   */
  rowHeight: number | undefined;
  /** Current scroll offset of the scroll container. */
  scrollTop: number;
  /** Measured height of the scroll container's viewport. */
  viewportHeight: number;
  /**
   * Only virtualize above this many items. Below it the plain `{#each}` is
   * cheap and keeps layouts that depend on intrinsic row heights correct.
   */
  threshold: number;
}

/**
 * Rows mounted above and below the viewport, so a fast scroll doesn't
 * reveal blank space before the next frame.
 */
const OVERSCAN = 6;

/**
 * Fallback viewport height for the first paint, before anything has been
 * measured. Generous on purpose: mounting a few too many rows for one
 * frame is invisible, mounting too few shows a short list.
 */
const ASSUMED_VIEWPORT_HEIGHT = 600;

/**
 * Compute the window to mount, or `null` when the list should render every
 * row (no uniform row height, or fewer items than `threshold`).
 */
export function computeVirtualWindow(
  input: VirtualWindowInput,
): VirtualWindow | null {
  const { count, rowHeight, scrollTop, viewportHeight, threshold } = input;

  if (rowHeight === undefined || rowHeight <= 0) return null;
  if (count <= threshold) return null;

  const visible =
    Math.ceil((viewportHeight || ASSUMED_VIEWPORT_HEIGHT) / rowHeight) +
    OVERSCAN * 2;
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN);
  const end = Math.min(count, start + visible);

  return { start, end, totalHeight: count * rowHeight };
}

/**
 * Absolute-position style for the row at `index` within a sizer of
 * `rowHeight`. Kept here so consumers don't each re-derive the offset.
 */
export function virtualRowStyle(index: number, rowHeight: number): string {
  return `position: absolute; left: 0; right: 0; top: ${index * rowHeight}px; height: ${rowHeight}px`;
}
