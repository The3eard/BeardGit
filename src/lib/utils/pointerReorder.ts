/**
 * Mouse-driven list reordering.
 *
 * Replaces HTML5 drag & drop for in-app row reordering: with Tauri's
 * `dragDropEnabled` on (required for the welcome-screen folder drop),
 * wry intercepts native drag sessions and HTML5 `dragover`/`drop`
 * never reach the webview on Windows — and WebKit builds vary on
 * macOS. Plain mousemove/mouseup never enter the native drag
 * machinery, so this works on every platform.
 *
 * Call from a row's `onmousedown`. The drag lives on `window`
 * listeners, so it survives the pointer leaving the list; a release
 * outside any row simply cancels. A plain click (press + release on
 * the same row) calls `onEnd` without `onDrop`.
 */

export interface PointerReorderOptions {
  /** The initiating mousedown event. */
  event: MouseEvent;
  /** Index of the row being dragged. */
  index: number;
  /** Element whose descendants matching `rowSelector` are the rows. */
  container: HTMLElement;
  /** Selector for the reorderable rows, in display order. */
  rowSelector: string;
  /** Hover feedback while dragging — hovered row index, null when off-list. */
  onDragOver: (index: number | null) => void;
  /** Release over a row other than the origin. `to` is the hovered index. */
  onDrop: (from: number, to: number) => void;
  /** Always called once the gesture ends (clear dragging visuals). */
  onEnd: () => void;
}

/** Row index whose vertical band contains `clientY`, or null. */
function hitTest(
  container: HTMLElement,
  rowSelector: string,
  clientY: number,
): number | null {
  const rows = container.querySelectorAll(rowSelector);
  for (let i = 0; i < rows.length; i++) {
    const rect = rows[i].getBoundingClientRect();
    if (clientY >= rect.top && clientY <= rect.bottom) return i;
  }
  return null;
}

export function startPointerReorder(opts: PointerReorderOptions): void {
  const { event, index, container, rowSelector, onDragOver, onDrop, onEnd } =
    opts;
  // Only primary-button drags; right-click must keep its context menu.
  if (event.button !== 0) return;
  // Suppress text-selection/native-drag side effects of the press.
  event.preventDefault();

  function onMove(e: MouseEvent) {
    onDragOver(hitTest(container, rowSelector, e.clientY));
  }

  function onUp(e: MouseEvent) {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
    const to = hitTest(container, rowSelector, e.clientY);
    if (to !== null && to !== index) onDrop(index, to);
    onEnd();
  }

  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}
