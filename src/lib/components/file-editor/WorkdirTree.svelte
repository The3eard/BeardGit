<!--
  WorkdirTree.svelte — the editor's file tree, expanded one level at a time.

  Deliberately not `PathTree`. That component builds its hierarchy by
  splitting a complete list of leaf paths, which is a fine model for a
  diff's file list and the wrong one here: it needs every path up front,
  and needing every path up front is what produced a 10,000-entry cap
  applied to a depth-first walk, folders that opened onto nothing, and a
  filter that could only search what had survived the cut. `PathTree` is
  also load-bearing for the PR/MR diff lists, so it stays as it is.

  Here a directory row knows only its own children, fetched when it is
  first opened. Nothing on the interactive path is recursive: opening a
  folder costs one `read_dir` of that folder.

  Presentational with respect to the backend — it reads and writes the
  `fileEditor` store and never calls the API itself.
-->
<script lang="ts">
  import type { WorkdirTreeEntry } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import { fileGlyphFor } from "./file-icons";
  import {
    expandedDirs,
    loadingDirs,
    toggleDirectory,
    treeChildren,
  } from "$lib/stores/fileEditor";

  interface Props {
    /** Directory whose children this level renders. `""` is the repo root. */
    prefix?: string;
    /** Nesting depth, purely for the indent. */
    depth?: number;
    /** Path of the row to mark as selected (the active editor tab). */
    selectedPath: string | null;
    /** Whether listings should hide gitignored entries. */
    respectGitignore: boolean;
    /** Called when a file row is activated. */
    onSelect: (path: string) => void;
  }

  let {
    prefix = "",
    depth = 0,
    selectedPath,
    respectGitignore,
    onSelect,
  }: Props = $props();

  let entries = $derived($treeChildren.get(prefix) ?? []);
  let isLoadingThisLevel = $derived($loadingDirs.has(prefix));

  function onRowClick(entry: WorkdirTreeEntry) {
    if (entry.is_directory) {
      void toggleDirectory(entry.path, respectGitignore);
    } else {
      onSelect(entry.path);
    }
  }

  /**
   * Left padding per level. The `aria-label` carries the full repo-relative
   * path because the context menu in `FileTreeView` reads it back off the
   * clicked button — same contract `PathTree` had, so the menu did not have
   * to change.
   */
  function indent(level: number): string {
    return `padding-left: ${6 + level * 14}px`;
  }
</script>

{#each entries as entry (entry.path)}
  {@const open = $expandedDirs.has(entry.path)}
  <button
    type="button"
    class="row"
    class:is-dir={entry.is_directory}
    class:is-selected={!entry.is_directory && entry.path === selectedPath}
    style={indent(depth)}
    aria-label={entry.path}
    aria-expanded={entry.is_directory ? open : undefined}
    data-pathtree-leaf={entry.is_directory ? undefined : true}
    data-pathtree-folder={entry.is_directory ? true : undefined}
    onclick={() => onRowClick(entry)}
  >
    <span class="glyph" aria-hidden="true">
      {#if entry.is_directory}
        {#if $loadingDirs.has(entry.path)}{:else}{open ? "" : ""}{/if}
      {:else}
        {fileGlyphFor(entry.name)}
      {/if}
    </span>
    <span class="name">{entry.name}</span>
  </button>

  {#if entry.is_directory && open}
    {#if $loadingDirs.has(entry.path) && !$treeChildren.has(entry.path)}
      <div class="level-state" style={indent(depth + 1)}>
        {m.editor_loading_tree()}
      </div>
    {:else if ($treeChildren.get(entry.path) ?? []).length === 0}
      <div class="level-state" style={indent(depth + 1)}>
        {m.editor_tree_empty_folder()}
      </div>
    {:else}
      <svelte:self
        prefix={entry.path}
        depth={depth + 1}
        {selectedPath}
        {respectGitignore}
        {onSelect}
      />
    {/if}
  {/if}
{/each}

{#if depth === 0 && entries.length === 0 && !isLoadingThisLevel}
  <div class="level-state" style={indent(0)}>{m.editor_tree_empty()}</div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding-top: 3px;
    padding-bottom: 3px;
    padding-right: 8px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row:hover {
    background: var(--overlay-hover);
  }

  .row.is-selected {
    background: var(--overlay-selected);
  }

  .glyph {
    flex-shrink: 0;
    width: 1.1em;
    font-family: var(--font-icons);
    color: var(--text-secondary);
  }

  .row.is-dir .glyph {
    color: var(--accent-primary);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .level-state {
    padding-top: 3px;
    padding-bottom: 3px;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }
</style>
