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

  let {
    refreshFn,
    left,
    right,
  }: {
    refreshFn: () => void | Promise<void>;
    left: Snippet;
    right: Snippet;
  } = $props();

  let sidebarWidth = $state(300);

  function startResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = sidebarWidth;

    function onMouseMove(e: MouseEvent) {
      const delta = e.clientX - startX;
      const minW = Math.max(220, window.innerWidth * 0.15);
      const maxW = Math.min(500, window.innerWidth * 0.45);
      sidebarWidth = Math.max(minW, Math.min(maxW, startWidth + delta));
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

<div class="split-view">
  <div class="split-sidebar" style="width: {sidebarWidth}px">
    {@render left()}
  </div>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="resize-handle" onmousedown={startResize}></div>
  <div class="split-main">
    {@render right()}
  </div>
</div>

<style>
  .split-view {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .split-sidebar {
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .resize-handle {
    width: 4px;
    cursor: col-resize;
    background: transparent;
    transition: background 0.15s;
    flex-shrink: 0;
  }

  .resize-handle:hover {
    background: var(--accent-blue);
  }

  .split-main {
    flex: 1;
    overflow: hidden;
  }
</style>
