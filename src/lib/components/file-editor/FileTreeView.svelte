<!--
  FileTreeView.svelte — left pane of the in-app editor.

  Renders the workdir tree — one level at a time via `WorkdirTree` — and
  exposes a context menu with file-system CRUD actions on each row.
  Selection mirrors the active editor tab so clicking a file opens it,
  and right-click on directory rows offers "New file/folder here" with
  the directory pre-filled as the parent in the dialogs.

  Typing in the filter switches the pane to a flat result list from
  `search_workdir_files`, which walks the working directory on the Rust
  side. The filter used to be a substring test over whatever the tree had
  loaded, which could not match a file the tree had not reached.

  This component is presentational: it reads from / writes to the
  `fileEditor` store and never talks to the backend directly. All
  mutating actions go through helper wrappers that funnel through
  `runMutation` so failures surface a sticky toast.
-->
<script lang="ts">
  import { addToast } from "$lib/stores/toast";
  import { IconButton, SearchInput } from "$lib/components/ui";
  import ContextMenu from "$lib/components/common/ContextMenu.svelte";
  import type { MenuItem } from "$lib/components/common/ContextMenu.svelte";
  import type { WorkdirTreeEntry } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import { fileGlyphFor } from "./file-icons";
  import WorkdirTree from "./WorkdirTree.svelte";
  import {
    activeTabPath,
    knownEntries,
    openTab,
    refreshTree,
    searchLoading,
    searchResults,
    searchTree,
    searchTruncated,
    treeLoading,
  } from "$lib/stores/fileEditor";

  /**
   * Open the rename / new-path / delete dialogs by setting these stores
   * from the parent. We keep them as callback props so the dialogs can
   * live next to the panel shell rather than inside the tree.
   */
  interface Props {
    /** Whether the file tree should hide gitignored entries. */
    respectGitignore: boolean;
    /** Caller-provided dialog openers. */
    onNewFile: (parentDir: string) => void;
    onNewFolder: (parentDir: string) => void;
    onRename: (entry: WorkdirTreeEntry) => void;
    onDelete: (entry: WorkdirTreeEntry) => void;
  }

  let {
    respectGitignore,
    onNewFile,
    onNewFolder,
    onRename,
    onDelete,
  }: Props = $props();

  let filterQuery = $state("");
  let searching = $derived(filterQuery.trim() !== "");

  /**
   * Debounce before hitting the backend. Every keystroke would otherwise
   * start a working-directory walk; 180ms is under the threshold where
   * typing feels laggy and long enough that a burst of keys is one query.
   * Stale answers are dropped in the store by request id, so an early
   * search finishing late cannot overwrite a later one.
   */
  const SEARCH_DEBOUNCE_MS = 180;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  /**
   * True from the keystroke until the query it produced has been answered.
   * Without it the pane shows "No files match." for the length of the
   * debounce on every fresh search — the query is non-empty, the request
   * has not started, and the previous results are empty.
   */
  let searchPending = $state(false);

  $effect(() => {
    const q = filterQuery;
    clearTimeout(searchTimer);
    searchPending = q.trim() !== "";
    searchTimer = setTimeout(() => {
      void searchTree(q, respectGitignore).finally(() => {
        // Only if the box still holds the query this timer was armed for:
        // an earlier search resolving under a newer pending one would
        // otherwise clear the flag and flash the empty state.
        if (q === filterQuery) searchPending = false;
      });
    }, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(searchTimer);
  });

  /**
   * Lookup by path for context-menu actions and rename / delete flows.
   * Covers both what the tree has expanded and the current search hits —
   * a right-click on a search result has to find its entry too.
   */
  let entryByPath = $derived.by(() => {
    const map = new Map<string, WorkdirTreeEntry>($knownEntries);
    for (const e of $searchResults) map.set(e.path, e);
    return map;
  });

  // Context-menu state.
  let menuVisible = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<MenuItem[]>([]);

  /** Build the menu for `entry`. Directory-vs-file shapes differ slightly. */
  function buildMenuItems(entry: WorkdirTreeEntry): MenuItem[] {
    const items: MenuItem[] = [];
    if (!entry.is_directory) {
      items.push({
        label: m.editor_open(),
        action: () => void openTab(entry.path),
      });
    }
    items.push({
      label: m.editor_rename(),
      action: () => onRename(entry),
    });
    items.push({
      label: m.editor_delete(),
      action: () => onDelete(entry),
    });
    items.push({ separator: true });
    const parentDir = entry.is_directory
      ? entry.path
      : entry.path.includes("/")
      ? entry.path.slice(0, entry.path.lastIndexOf("/"))
      : "";
    items.push({
      label: m.editor_new_file_here(),
      action: () => onNewFile(parentDir),
    });
    items.push({
      label: m.editor_new_folder_here(),
      action: () => onNewFolder(parentDir),
    });
    items.push({ separator: true });
    items.push({
      label: m.editor_copy_path(),
      action: () => {
        void navigator.clipboard.writeText(entry.path);
        addToast({ message: m.editor_copy_path_done(), type: "info" });
      },
    });
    return items;
  }

  /** Right-click hook attached to each PathTree leaf via a delegating handler. */
  function onTreeContext(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    const btn = target?.closest<HTMLButtonElement>(
      "[data-pathtree-leaf], [data-pathtree-folder]",
    );
    if (!btn) return;
    const path = btn.getAttribute("aria-label") ?? "";
    if (!path) return;
    const entry = entryByPath.get(path);
    if (!entry) return;
    e.preventDefault();
    menuItems = buildMenuItems(entry);
    menuX = e.clientX;
    menuY = e.clientY;
    menuVisible = true;
  }

  /** Header reload — fires `refreshTree` and surfaces nothing on success. */
  function reload() {
    void refreshTree(respectGitignore);
  }

  function onSelect(path: string) {
    void openTab(path);
  }

  /** Parent directory of a search hit, shown dimmed beside the file name. */
  function dirOf(path: string): string {
    const cut = path.lastIndexOf("/");
    return cut < 0 ? "" : path.slice(0, cut);
  }
</script>

<div class="file-tree-view">
  <header class="tree-header">
    <div class="search">
      <SearchInput
        bind:value={filterQuery}
        placeholder={m.editor_tree_filter_placeholder()}
      />
    </div>
    <IconButton
      icon={""}
      description={m.editor_refresh_tree()}
      size="sm"
      onclick={reload}
    />
  </header>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="tree-body" oncontextmenu={onTreeContext}>
    {#if searching}
      {#if (searchPending || $searchLoading) && $searchResults.length === 0}
        <div class="tree-state">
          <span class="muted">{m.editor_tree_searching()}</span>
        </div>
      {:else if $searchResults.length === 0}
        <div class="tree-state">
          <span class="muted">{m.editor_tree_no_matches()}</span>
        </div>
      {:else}
        {#each $searchResults as hit (hit.path)}
          <button
            type="button"
            class="result"
            class:is-selected={hit.path === $activeTabPath}
            aria-label={hit.path}
            data-pathtree-leaf="true"
            onclick={() => onSelect(hit.path)}
          >
            <span class="glyph" aria-hidden="true">{fileGlyphFor(hit.name)}</span>
            <span class="result-name">{hit.name}</span>
            <span class="result-dir">{dirOf(hit.path)}</span>
          </button>
        {/each}
      {/if}
    {:else if $treeLoading}
      <div class="tree-state">
        <span class="muted">{m.editor_loading_tree()}</span>
      </div>
    {:else}
      <WorkdirTree
        selectedPath={$activeTabPath}
        {respectGitignore}
        {onSelect}
      />
    {/if}
  </div>
  {#if searching && $searchTruncated}
    <footer class="tree-footer" role="status">
      {m.editor_search_truncated()}
    </footer>
  {/if}
</div>

<ContextMenu
  items={menuItems}
  x={menuX}
  y={menuY}
  visible={menuVisible}
  onClose={() => (menuVisible = false)}
/>

<style>
  .file-tree-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-secondary);
    min-width: 0;
  }
  .tree-header {
    display: flex;
    align-items: center;
    gap: 6px;
    /* Fixed to the shared header height rather than derived from the
       search input plus padding, which made this 46px against the content
       pane's 36px — a 10px step that broke the divider line in two. The
       input centres itself in what is left. */
    height: var(--panel-header-height);
    box-sizing: border-box;
    padding: 0 10px;
    border-bottom: 1px solid var(--border);
  }
  .tree-header .search {
    flex: 1;
    min-width: 0;
  }
  .tree-body {
    flex: 1;
    overflow: auto;
    /* Extra left gutter so the file/folder icons aren't flush against
       the panel edge — paired with the per-row 10 px button padding it
       gives the icons ~14 px of breathing room from the divider. */
    padding: 6px 0 6px 4px;
  }
  .tree-state {
    padding: 10px 12px;
    font-size: var(--font-size-sm);
  }
  .muted {
    color: var(--text-secondary);
  }
  .result {
    display: flex;
    align-items: baseline;
    gap: 6px;
    width: 100%;
    padding: 3px 8px 3px 6px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    text-align: left;
    cursor: pointer;
    overflow: hidden;
  }
  .result:hover {
    background: var(--overlay-hover);
  }
  .result.is-selected {
    background: var(--overlay-selected);
  }
  .glyph {
    flex-shrink: 0;
    width: 1.1em;
    font-family: var(--font-icons);
    color: var(--text-secondary);
  }
  .result-name {
    flex-shrink: 0;
  }
  /* The directory is context, not the answer: it gets the dim rung and
     loses its head rather than pushing the file name out of view. */
  .result-dir {
    min-width: 0;
    overflow: hidden;
    direction: rtl;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-size-xs);
    color: var(--text-muted);
  }
  .tree-footer {
    padding: 6px 10px;
    border-top: 1px solid var(--border);
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    background: var(--bg-toolbar);
  }
</style>
