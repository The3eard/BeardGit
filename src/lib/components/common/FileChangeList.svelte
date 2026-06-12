<!--
  FileChangeList — Selectable file list with status icons and path highlighting.

  Shared component used by tag detail, graph commit detail, stash detail,
  and branch commit detail. Displays repo-relative paths with directory
  portions dimmed and the filename highlighted. Emits `onSelect` when a
  file is clicked.
-->
<script lang="ts">
  import type { CommitFileChange } from "../../types";

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

  function handleClick(path: string) {
    if (onSelect) {
      onSelect(path);
    }
    selectedPath = selectedPath === path ? null : path;
  }

  function fileStatusIcon(status: string): string {
    switch (status) {
      case "added":    return "+";
      case "deleted":  return "-";
      case "renamed":  return "R";
      case "copied":   return "C";
      default:         return "~";
    }
  }

  function fileStatusClass(status: string): string {
    switch (status) {
      case "added":    return "status-added";
      case "deleted":  return "status-deleted";
      case "renamed":  return "status-renamed";
      case "copied":   return "status-copied";
      default:         return "status-modified";
    }
  }

  function splitPath(path: string): { dir: string; name: string } {
    const idx = path.lastIndexOf("/");
    if (idx === -1) return { dir: "", name: path };
    return { dir: path.slice(0, idx + 1), name: path.slice(idx + 1) };
  }
</script>

{#if files.length > 0}
  <ul class="file-list">
    {#each files as file (file.path)}
      {@const parts = splitPath(file.path)}
      <li>
        <button
          class="file-item"
          class:selected={selectedPath === file.path}
          onclick={() => handleClick(file.path)}
          oncontextmenu={onContextMenu ? (e) => onContextMenu!(e, file.path) : undefined}
        >
          <span class="file-status {fileStatusClass(file.status)}">{fileStatusIcon(file.status)}</span>
          <span class="file-path">
            {#if parts.dir}<span class="file-dir">{parts.dir}</span>{/if}<span class="file-name">{parts.name}</span>
          </span>
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .file-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .file-item {
    display: flex;
    align-items: baseline;
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

  .file-status {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    font-weight: 700;
    flex-shrink: 0;
    width: 12px;
    text-align: center;
  }

  .status-added    { color: var(--accent-green); }
  .status-deleted  { color: var(--accent-red); }
  .status-modified { color: var(--accent-orange); }
  .status-renamed  { color: var(--accent-purple); }
  .status-copied   { color: var(--accent-primary); }

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
