<!--
  FileTreeView.svelte — left pane of the in-app editor.

  Renders the workdir tree (filtered by an optional substring search) and
  exposes a context menu with file-system CRUD actions on each row.
  Selection mirrors the active editor tab so clicking a file opens it,
  and right-click on directory rows offers "New file/folder here" with
  the directory pre-filled as the parent in the dialogs.

  This component is presentational: it reads from / writes to the
  `fileEditor` store and never talks to the backend directly. All
  mutating actions go through helper wrappers that funnel through
  `runMutation` so failures surface a sticky toast.
-->
<script lang="ts">
  import { addToast } from "$lib/stores/toast";
  import { IconButton, SearchInput } from "$lib/components/ui";
  import PathTree from "$lib/components/common/PathTree.svelte";
  import ContextMenu from "$lib/components/common/ContextMenu.svelte";
  import type { MenuItem } from "$lib/components/common/ContextMenu.svelte";
  import type { WorkdirTreeEntry } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import { fileGlyphFor } from "./file-icons";
  import {
    activeTabPath,
    openTab,
    refreshTree,
    treeEntries,
    treeLoading,
    treeTruncated,
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

  /** Items shown in the PathTree — file rows only, filtered by the query. */
  let filteredItems = $derived.by(() => {
    const q = filterQuery.trim().toLowerCase();
    return $treeEntries
      .filter((e) => !e.is_directory)
      .filter((e) => (q === "" ? true : e.path.toLowerCase().includes(q)))
      .map((e) => ({ path: e.path, meta: e }));
  });

  /** Lookup by path for context-menu actions and rename / delete flows. */
  let entryByPath = $derived.by(() => {
    const map = new Map<string, WorkdirTreeEntry>();
    for (const e of $treeEntries) map.set(e.path, e);
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
    {#if $treeLoading && $treeEntries.length === 0}
      <div class="tree-state">
        <span class="muted">{m.editor_loading_tree()}</span>
      </div>
    {:else if filteredItems.length === 0}
      <div class="tree-state">
        <span class="muted">{m.editor_no_tab_open()}</span>
      </div>
    {:else}
      <PathTree
        items={filteredItems}
        selectedPath={$activeTabPath}
        showIcons
        fileIconResolver={fileGlyphFor}
        autoFlattenThreshold={0}
        {onSelect}
      />
    {/if}
  </div>
  {#if $treeTruncated}
    <footer class="tree-footer" role="status">
      {m.editor_tree_truncated()}
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
    padding: 8px 10px;
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
  .tree-footer {
    padding: 6px 10px;
    border-top: 1px solid var(--border);
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    background: var(--bg-toolbar);
  }
</style>
