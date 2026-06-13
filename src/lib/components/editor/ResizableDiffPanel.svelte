<script lang="ts">
  /**
   * ResizableDiffPanel — the bottom diff panel shell shared by every view
   * that shows a file diff under its main content (graph commit diff,
   * branch/reflog file diff, PR/MR file diff).
   *
   * Owns a single drag handle: the full-width bar on top → vertical
   * resize (height), capped at 4/5 of the window so the panel can grow
   * large while keeping the view above it usable. The panel's outer
   * edges always stick to the surrounding columns/window — horizontal
   * balance inside a diff is the DiffEditor's own centre split handle.
   *
   * Height is held in the `diffPanelSize` store so it persists across
   * view switches. The diff content is provided by the caller as
   * `children`.
   */
  import { onMount, onDestroy, type Snippet } from "svelte";
  import { diffPanelHeight, resetDiffPanelHeight } from "$lib/stores/diffPanelSize";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Centre the panel content — used for the standalone loading spinner. */
    loading?: boolean;
    children: Snippet;
  }
  let { loading = false, children }: Props = $props();

  let rowEl: HTMLDivElement;
  let isDraggingHeight = $state(false);

  const MIN_HEIGHT = 150;
  /** Keep at least this many px of the view above the panel visible. */
  const VIEW_MIN_REMAINDER = 80;

  /** Upper bound for the height: 4/5 of the window, but never so tall the
   *  view above (graph/list) drops below `VIEW_MIN_REMAINDER`. */
  function maxHeight(): number {
    const winCap = window.innerHeight * 0.8;
    const container = rowEl?.parentElement;
    if (!container) return winCap;
    return Math.min(winCap, container.clientHeight - VIEW_MIN_REMAINDER);
  }

  // Clamp into [min(MIN, hi), hi]. On a very short window/container the upper
  // bound can fall below MIN; the lower bound is capped at `hi` so the result
  // never exceeds the cap (a naive max(MIN, min(hi, h)) would push it back up).
  function clampHeight(h: number): number {
    const hi = maxHeight();
    return Math.max(Math.min(MIN_HEIGHT, hi), Math.min(hi, h));
  }
  // Active drag teardown, so a mid-drag unmount (the panel is conditionally
  // rendered) doesn't leak window mousemove/mouseup listeners.
  let activeDragCleanup: (() => void) | null = null;

  function startHeightResize(e: MouseEvent) {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = $diffPanelHeight;
    isDraggingHeight = true;
    const onMove = (ev: MouseEvent) => {
      const delta = startY - ev.clientY; // dragging up grows the panel
      diffPanelHeight.set(clampHeight(startHeight + delta));
    };
    const stop = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", stop);
      isDraggingHeight = false;
      activeDragCleanup = null;
    };
    activeDragCleanup = stop;
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", stop);
  }

  // Correct a persisted oversize value on mount and whenever the window
  // shrinks — clamping otherwise only ran inside the drag/keyboard handlers,
  // so a panel sized large in a maximized window kept its size after a resize.
  onMount(() => {
    const reclamp = () => {
      diffPanelHeight.set(clampHeight($diffPanelHeight));
    };
    reclamp();
    window.addEventListener("resize", reclamp);
    return () => window.removeEventListener("resize", reclamp);
  });

  // Safety net: if the panel unmounts mid-drag, tear the window listeners down.
  onDestroy(() => activeDragCleanup?.());

  function handleHeightKeys(e: KeyboardEvent) {
    if (e.key === "ArrowUp") {
      e.preventDefault();
      diffPanelHeight.set(clampHeight($diffPanelHeight + 20));
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      diffPanelHeight.set(clampHeight($diffPanelHeight - 20));
    } else if (e.key === "Home") {
      e.preventDefault();
      resetDiffPanelHeight();
    }
  }

</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="diff-resize-handle"
  class:is-dragging={isDraggingHeight}
  role="separator"
  aria-orientation="horizontal"
  aria-label={m.resize_diff_panel()}
  tabindex="0"
  onmousedown={startHeightResize}
  ondblclick={resetDiffPanelHeight}
  onkeydown={handleHeightKeys}
></div>

<div class="diff-row" bind:this={rowEl} style="height: {$diffPanelHeight}px">
  <div class="diff-panel" class:diff-panel-loading={loading}>
    {@render children()}
  </div>
</div>

<style>
  .diff-resize-handle {
    height: 4px;
    cursor: row-resize;
    background: transparent;
    transition: background 0.15s;
    flex-shrink: 0;
    border-top: 1px solid var(--border);
  }
  .diff-resize-handle:hover {
    background: var(--overlay-accent-blue);
  }
  .diff-resize-handle.is-dragging {
    background: var(--accent-primary);
  }

  .diff-row {
    display: flex;
    flex-shrink: 0;
    overflow: hidden;
  }

  .diff-panel {
    flex: 1 1 0;
    min-width: 0;
    height: 100%;
    overflow: hidden;
  }

  .diff-panel-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    border-top: 1px solid var(--border);
  }
</style>
