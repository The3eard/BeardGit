<!--
  SplitView — Resizable horizontal split panel with left/right snippets.

  Used by TagView, StashView, BranchView, and other two-pane layouts.
  The resize handle enforces min/max constraints via clamp(). Listens for
  `repo-changed` events to auto-refresh via the provided `refreshFn`.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import type { Snippet } from "svelte";
  import { writable } from "svelte/store";
  import { remembered } from "$lib/stores/viewMemory";

  let {
    refreshFn,
    left,
    right,
    defaultWidth = 304,
    memoryKey,
  }: {
    refreshFn: () => void | Promise<void>;
    left: Snippet;
    right: Snippet;
    /** Initial width of the left panel in px. On resize the width is
     *  clamped between 220px and 80% of the split container. */
    defaultWidth?: number;
    /** When set, the dragged width survives leaving the view (see
     *  `stores/viewMemory`). Global key — layout is not per repo. */
    memoryKey?: string;
  } = $props();

  // svelte-ignore state_referenced_locally
  // `defaultWidth` seeds the initial width; parent-side updates are intentionally ignored
  // because the pane width becomes user-controlled once resizing starts.
  const sidebarWidth = memoryKey
    ? remembered(memoryKey, defaultWidth)
    : writable(defaultWidth);

  function startResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = $sidebarWidth;
    // Measure the split container at drag start: the left pane may grow
    // up to 80% of it, so the right pane always keeps ~20%.
    const containerWidth =
      (e.currentTarget as HTMLElement).parentElement?.clientWidth ??
      window.innerWidth;

    function onMouseMove(e: MouseEvent) {
      const delta = e.clientX - startX;
      const minW = Math.max(220, window.innerWidth * 0.15);
      const maxW = containerWidth * 0.8;
      $sidebarWidth = Math.max(minW, Math.min(maxW, startWidth + delta));
    }

    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    }

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  onMount(() => {
    refreshFn();

    const unlisten = listen("repo-changed", () => {
      refreshFn();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  });
</script>

<div class="split-view" style="--split-x: {$sidebarWidth}px">
  <div class="split-sidebar" style="width: {$sidebarWidth}px">
    {@render left()}
  </div>
  <!-- The handle is positioned over the seam rather than laid out in it —
       see `.resize-handle` below and lib/styles/resize-handle.css. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="resize-handle" onmousedown={startResize}></div>
  <div class="split-main">
    {@render right()}
  </div>
</div>

<style>
  .split-view {
    display: flex;
    /* Anchor for the absolutely-positioned resize handle. */
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  /* Straddles the seam instead of sitting in it. Laid out in the flex row,
     this was a 4px band between the two panels that had to be *some*
     colour, and no colour was right: the neighbouring surface differs per
     view. Out of flow, the panels meet at the sidebar's own 1px border and
     the grab zone floats over it. Same approach as `DiffEditor`'s inner
     split, which already did this. */
  .resize-handle {
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--split-x);
    width: 8px;
    margin-left: -4px;
    z-index: 2;
  }

  .split-sidebar {
    flex-shrink: 0;
    /* The separator line lives here rather than on the handle. Moving it
       onto the handle is tidier and shifts every pane by 1px under the
       global `box-sizing: border-box` — see lib/styles/resize-handle.css. */
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .split-main {
    flex: 1;
    overflow: hidden;
  }
</style>
