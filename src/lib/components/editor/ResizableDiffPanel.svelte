<script lang="ts">
  /**
   * ResizableDiffPanel — the bottom diff panel shell shared by every view
   * that shows a file diff under its main content (graph commit diff,
   * branch/reflog file diff, PR/MR file diff).
   *
   * Owns two drag handles:
   *  - a full-width bar on top → vertical resize (height), capped at 4/5 of
   *    the window so the panel can grow large while keeping the view above
   *    it usable;
   *  - a thin bar on the panel's right edge → horizontal resize (width).
   *    It lives outside the panel as a flex sibling, so it never overlaps
   *    the diff's own vertical scrollbar.
   *
   * Size is held in the `diffPanelSize` store so it persists across view
   * switches. The diff content is provided by the caller as `children`.
   */
  import { onMount, onDestroy, type Snippet } from "svelte";
  import {
    diffPanelHeight,
    diffPanelWidth,
    resetDiffPanelHeight,
    resetDiffPanelWidth,
  } from "$lib/stores/diffPanelSize";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Centre the panel content — used for the standalone loading spinner. */
    loading?: boolean;
    children: Snippet;
  }
  let { loading = false, children }: Props = $props();

  let rowEl: HTMLDivElement;
  let panelEl: HTMLDivElement;
  let isDraggingHeight = $state(false);
  let isDraggingWidth = $state(false);

  const MIN_HEIGHT = 150;
  const MIN_WIDTH = 320;
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

  /** Upper bound for the width: the full row width. */
  function maxWidth(): number {
    return rowEl?.clientWidth ?? window.innerWidth;
  }

  // Clamp into [min(MIN, hi), hi]. On a very short window/container the upper
  // bound can fall below MIN; the lower bound is capped at `hi` so the result
  // never exceeds the cap (a naive max(MIN, min(hi, h)) would push it back up).
  function clampHeight(h: number): number {
    const hi = maxHeight();
    return Math.max(Math.min(MIN_HEIGHT, hi), Math.min(hi, h));
  }
  function clampWidth(w: number): number {
    const hi = maxWidth();
    return Math.max(Math.min(MIN_WIDTH, hi), Math.min(hi, w));
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

  function startWidthResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = panelEl?.offsetWidth ?? maxWidth();
    isDraggingWidth = true;
    const onMove = (ev: MouseEvent) => {
      const delta = ev.clientX - startX; // dragging right grows the panel
      diffPanelWidth.set(clampWidth(startWidth + delta));
    };
    const stop = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", stop);
      isDraggingWidth = false;
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
      diffPanelWidth.update((w) => (w == null ? w : clampWidth(w)));
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

  function handleWidthKeys(e: KeyboardEvent) {
    const current = panelEl?.offsetWidth ?? maxWidth();
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      diffPanelWidth.set(clampWidth(current - 20));
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      diffPanelWidth.set(clampWidth(current + 20));
    } else if (e.key === "Home") {
      e.preventDefault();
      resetDiffPanelWidth();
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
  <div
    class="diff-panel"
    class:diff-panel-loading={loading}
    bind:this={panelEl}
    style={$diffPanelWidth != null ? `flex: 0 0 ${$diffPanelWidth}px` : ""}
  >
    {@render children()}
  </div>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="diff-width-handle"
    class:is-dragging={isDraggingWidth}
    role="separator"
    aria-orientation="vertical"
    aria-label={m.resize_diff_panel()}
    tabindex="0"
    onmousedown={startWidthResize}
    ondblclick={resetDiffPanelWidth}
    onkeydown={handleWidthKeys}
  ></div>
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

  .diff-width-handle {
    flex: 0 0 4px;
    cursor: col-resize;
    background: transparent;
    transition: background 0.15s;
    border-left: 1px solid var(--border);
  }
  .diff-width-handle:hover {
    background: var(--overlay-accent-blue);
  }
  .diff-width-handle.is-dragging {
    background: var(--accent-primary);
  }
</style>
