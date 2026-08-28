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

  /**
   * The folder glyphs, as escapes rather than literal private-use
   * characters.
   *
   * This file is why `scripts/check-icon-glyphs.mjs` exists: a scripted
   * edit replaced all three of these with empty strings, and the whole
   * gate plus a re-rendered screenshot went green over it, because a
   * literal PUA character is invisible in a diff and in most terminals.
   * An escape is not — you can see it go missing, and you can see it
   * change. Named, so the markup says which icon it means.
   */
  const GLYPH_SPINNER = "\uF110";
  const GLYPH_FOLDER_OPEN = "\uF07C";
  const GLYPH_FOLDER_CLOSED = "\uF07B";
  // Self-import rather than `<svelte:self>`, which Svelte 5 deprecates.
  import WorkdirTree from "./WorkdirTree.svelte";
  import {
    DIRECTORY_ENTRY_CAP,
    expandedDirs,
    failedDirs,
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
  /**
   * A directory holding more children than the cap comes back cut, and the
   * cut happens during `read_dir` — before the sort — so what is missing is
   * arbitrary. Small-scale, that is the same shape as the bug this tree
   * replaced, and the difference that matters is that it says so.
   */
  let truncated = $derived(entries.length >= DIRECTORY_ENTRY_CAP);

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
        {#if $loadingDirs.has(entry.path)}{GLYPH_SPINNER}{:else}{open ? GLYPH_FOLDER_OPEN : GLYPH_FOLDER_CLOSED}{/if}
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
    {:else if $failedDirs.has(entry.path)}
      <!-- Not "empty": a listing that failed is not a fact about the
           filesystem, and saying so would be the tree lying again. -->
      <div class="level-state is-error" style={indent(depth + 1)}>
        {m.editor_tree_level_failed()}
      </div>
    {:else if ($treeChildren.get(entry.path) ?? []).length === 0}
      <div class="level-state" style={indent(depth + 1)}>
        {m.editor_tree_empty_folder()}
      </div>
    {:else}
      <WorkdirTree
        prefix={entry.path}
        depth={depth + 1}
        {selectedPath}
        {respectGitignore}
        {onSelect}
      />
    {/if}
  {/if}
{/each}

{#if truncated}
  <div class="level-state" style={indent(depth)} role="status">
    {m.editor_tree_level_truncated({ count: String(DIRECTORY_ENTRY_CAP) })}
  </div>
{/if}

{#if depth === 0 && entries.length === 0 && !isLoadingThisLevel}
  <div class="level-state" style={indent(0)}>
    {$failedDirs.has("") ? m.editor_tree_level_failed() : m.editor_tree_empty()}
  </div>
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
    padding-right: 8px;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .level-state.is-error {
    color: var(--accent-red);
  }
</style>
