<!--
  FileChangeList — Selectable file list with status icons and path highlighting.

  Shared component used by tag detail, graph commit detail, stash detail,
  and branch commit detail. Displays repo-relative paths with directory
  portions dimmed and the filename highlighted. Emits `onSelect` when a
  file is clicked.
-->
<script lang="ts">
  import type { CommitFileChange } from "../../types";
  import FileStatusBadge from "./FileStatusBadge.svelte";
  import {
    computeVirtualWindow,
    findScroller,
    measureAgainstScroller,
    virtualRowStyle,
  } from "../../utils/virtualWindow";

  let {
    files,
    onSelect,
    onContextMenu,
  }: {
    files: CommitFileChange[];
    onSelect?: (path: string) => void;
    onContextMenu?: (e: MouseEvent, path: string) => void;
  } = $props();

  let selectedPath = $state<string | null>(null);

  // Reset selection when files change
  $effect(() => {
    if (files) {
      selectedPath = null;
    }
  });

  // ── Virtualization ────────────────────────────────────────────────────
  // A commit's file list has no cap: an initial commit or a wide merge can
  // carry thousands of paths, and each row mounts a status badge.
  //
  // 26 px measured in the browser (`padding: 4px` plus content), uniform.
  // The scroll container is the detail `<aside>`, not this list — see
  // `measureAgainstScroller`. Only engages above 500 rows, so every existing
  // baseline renders through the plain `{#each}`.
  const ROW_HEIGHT = 26;

  let listEl = $state<HTMLUListElement | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(0);

  let virtualWindow = $derived(
    computeVirtualWindow({
      count: files.length,
      rowHeight: ROW_HEIGHT,
      scrollTop,
      viewportHeight,
      threshold: 500,
    }),
  );

  $effect(() => {
    void files.length;
    if (!listEl) return;
    const scroller = findScroller(listEl);
    if (!scroller) return;

    const measure = () => {
      if (!listEl) return;
      ({ scrollTop, viewportHeight } = measureAgainstScroller(listEl, scroller));
    };
    measure();
    scroller.addEventListener("scroll", measure, { passive: true });
    return () => scroller.removeEventListener("scroll", measure);
  });

  function handleClick(path: string) {
    if (onSelect) {
      onSelect(path);
    }
    selectedPath = selectedPath === path ? null : path;
  }

  function splitPath(path: string): { dir: string; name: string } {
    const idx = path.lastIndexOf("/");
    if (idx === -1) return { dir: "", name: path };
    return { dir: path.slice(0, idx + 1), name: path.slice(idx + 1) };
  }
</script>

{#if files.length > 0}
  <ul class="file-list" bind:this={listEl}>
    {#if virtualWindow}
      <!-- Windowed: the sizer holds the full scroll height, and only the
           visible slice is mounted, anchored at (index * ROW_HEIGHT). -->
      <li
        class="virt-sizer"
        style="height: {virtualWindow.totalHeight}px"
        aria-hidden="true"
      >
        {#each files.slice(virtualWindow.start, virtualWindow.end) as file, offset (file.path)}
          {@render fileRow(file, virtualWindow.start + offset, true)}
        {/each}
      </li>
    {:else}
      {#each files as file (file.path)}
        {@render fileRow(file, 0, false)}
      {/each}
    {/if}
  </ul>
{/if}

{#snippet fileRow(file: CommitFileChange, index: number, positioned: boolean)}
  {@const parts = splitPath(file.path)}
  <li style={positioned ? virtualRowStyle(index, ROW_HEIGHT) : undefined}>
    <button
      class="file-item"
      class:selected={selectedPath === file.path}
      onclick={() => handleClick(file.path)}
      oncontextmenu={onContextMenu ? (e) => onContextMenu!(e, file.path) : undefined}
    >
      <FileStatusBadge status={file.status} />
      <span class="file-path">
        {#if parts.dir}<span class="file-dir">{parts.dir}</span>{/if}<span class="file-name">{parts.name}</span>
      </span>
    </button>
  </li>
{/snippet}

<style>
  .file-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  /* Sizer for the windowed path: carries the full scroll height while only
     the visible slice is mounted, absolutely positioned inside it. */
  .virt-sizer {
    position: relative;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    min-width: 0;
    width: 100%;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    border-radius: 0;
    transition: background 0.1s;
  }

  .file-item:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .file-item.selected {
    background: var(--overlay-accent-blue);
  }

  .file-path {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .file-dir {
    color: var(--text-secondary);
  }

  .file-name {
    color: var(--text-primary);
  }
</style>
