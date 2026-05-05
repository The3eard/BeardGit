<!--
  Collections tree for the Requests panel.

  Lists project-local `.http` files under the active repo's
  `.beardgit/requests/` folder. Selecting a file sets `currentSource`
  so the editor panes can load it.

  Scope walkback (issue: single-tree request): the tree is
  project-only in this iteration. The global library still lives in
  `requests.db` (used by background features such as Paste cURL into
  the global drawer) but is not surfaced in the sidebar tree —
  surfacing it doubled cognitive load without delivering value when
  every starter pack is project-scoped. The backend storage primitives
  for global items remain so we can bring back a dedicated global
  drawer later without a schema migration.

  Visual contract: mirrors `BranchList` + `BranchTreeNode` exactly so
  the Requests panel feels like the rest of the sidebar.

  - Single collapsible section (PROJECT) using the same `section-header`
    chrome as BranchList's LOCAL bar: bg `--bg-secondary`, bottom border,
    chevron + uppercase label + count pill in
    `color-mix(--text-primary 6%, transparent)`.
  - Folder rows use `BranchTreeNode`'s folder recipe: chevron, folder
    glyph in `--accent-blue` at 0.7 opacity, hover in
    `color-mix(--text-primary 3%, transparent)`.
  - File rows use the BranchTreeNode leaf recipe: 5 px vertical
    padding, gap 5, `--border` separator under each row, hover in the
    3% tonal, selected in `var(--overlay-accent-blue)`. Indentation is
    `depth * 16 + 12` px to match `BranchTreeNode.indent` exactly.

  The verb badges (GET / POST / PUT / DELETE) introduced in commit
  1253561 stay; their tonal recipe (color-mix on the accent token) is
  the same vocabulary used by BranchList's `track-pip` ahead/behind
  indicators, so they share visual DNA.

  Project list re-fetches whenever the active project path changes.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Button } from "$lib/components/ui";
  import ContextMenu from "$lib/components/common/ContextMenu.svelte";
  import type { MenuItem } from "$lib/components/common/ContextMenu.svelte";
  import ConfirmDialog from "$lib/components/common/ConfirmDialog.svelte";
  import NewRequestDialog from "./NewRequestDialog.svelte";
  import {
    currentSource,
    currentEnv,
    treeReloadSignal,
    newRequestOpen,
  } from "./stores";
  import { activeProject } from "$lib/stores/projects";
  import { addToast } from "$lib/stores/toast";

  type Node = {
    kind: "folder" | "file";
    name: string;
    rel_path: string;
    /** HTTP method for `.http` leaf nodes; null for folders. */
    method?: string | null;
    children: Node[];
  };

  let project: Node[] = [];

  /** Section collapse state — matches BranchList's local/remote pattern. */
  let projectCollapsed = false;

  // Open state of the New Request dialog comes from the shared
  // `newRequestOpen` store. Both the in-tree "+ New" button and the
  // SeedPrompt's "Create new request" button toggle the same store;
  // this component owns the actual dialog markup.

  /**
   * Per-folder open/closed state, keyed by relative path. We default
   * everything to "open" and only flip when the user toggles a
   * folder, so deep trees still surface their leaves on first paint.
   * The previous `<scope>:<rel_path>` keying was kept when the global
   * tree shared this state — now project-only.
   */
  let folderOpen: Record<string, boolean> = {};

  function isFolderOpen(rel: string): boolean {
    // Undefined → default open. Explicit false → user collapsed it.
    return folderOpen[rel] !== false;
  }

  function toggleFolder(rel: string): void {
    folderOpen = { ...folderOpen, [rel]: !isFolderOpen(rel) };
  }

  $: projectPath = $activeProject?.path ?? "";

  /**
   * Reload the project tree from the backend. When no project is
   * active the list clears. Global items are no longer loaded into
   * the sidebar tree — the storage primitives are still there for
   * future surfaces but the sidebar is project-only.
   */
  async function reload() {
    if (!projectPath) {
      project = [];
      return;
    }
    project = await invoke<Node[]>("requests_list_project", { projectPath });
  }

  /** Total file count under a tree (for the section count pill). */
  function fileCount(nodes: Node[]): number {
    let n = 0;
    for (const node of nodes) {
      if (node.kind === "file") n++;
      n += fileCount(node.children);
    }
    return n;
  }

  /** Open a file node by setting it as `currentSource`. Folder clicks toggle. */
  function open(node: Node) {
    if (node.kind !== "file") return;
    currentSource.set({ kind: "project", path: node.rel_path });
  }

  /**
   * Open the "+ New" dialog (project-only in this iteration).
   */
  function openNewRequest(e?: MouseEvent) {
    // The button lives inside the section-header row, which is itself
    // a `role="button"` toggling collapse. Stop propagation so opening
    // the dialog does not also collapse/expand the section.
    e?.stopPropagation();
    newRequestOpen.set(true);
  }

  /** True when this node is the currently-selected source. */
  function isSelected(node: Node): boolean {
    return (
      $currentSource?.kind === "project" && $currentSource?.path === node.rel_path
    );
  }

  /* ------------------------------------------------------------------
   * Context menu — right-click on a leaf row.
   *
   * The menu mirrors the affordances every other forge tree exposes
   * (Branches' BranchList, MR/PR list): Copy as cURL / Duplicate /
   * Rename / Delete, plus an "Open in editor" item that's only valid
   * for project-scoped files (the global library lives in SQLite, not
   * on disk).
   *
   * Folder rows are deliberately left alone for v1 — see TODO below.
   * ------------------------------------------------------------------ */

  /** Context-menu cursor position + addressed leaf. `null` when closed. */
  let ctxMenu: {
    x: number;
    y: number;
    node: Node;
  } | null = null;

  /** Pending delete; while non-null the ConfirmDialog overlay is open. */
  let pendingDelete: { node: Node } | null = null;

  /** Pending rename; while non-null an inline rename input replaces the row. */
  let pendingRename: { node: Node } | null = null;
  /** Live value of the rename input (relative path). */
  let renameValue = "";

  /**
   * Build the right-click menu items for a single leaf. Always
   * project-scoped now that the tree only renders project files.
   */
  function buildLeafMenu(node: Node): MenuItem[] {
    const items: MenuItem[] = [];
    items.push({ label: "Copy as cURL", action: () => copyAsCurl(node) });
    items.push({ label: "Duplicate", action: () => duplicate(node) });
    items.push({
      label: "Rename",
      action: () => startRename(node),
    });
    items.push({
      label: "Open in editor",
      action: () => openInEditor(node),
    });
    items.push({ separator: true });
    items.push({
      label: "Delete",
      action: () => {
        pendingDelete = { node };
      },
    });
    return items;
  }

  /** Show the context menu at the cursor for a given leaf. */
  function showCtxMenu(e: MouseEvent, node: Node) {
    if (node.kind !== "file") return;
    e.preventDefault();
    ctxMenu = { x: e.clientX, y: e.clientY, node };
  }

  /**
   * Run "Copy as cURL" using the same backend command CopyAsMenu uses.
   *
   * Backend resolution and clipboard write are wrapped in separate
   * try/catch blocks so errors from either stage surface as a red
   * toast instead of being silently swallowed (the previous code lost
   * any clipboard-write failure as part of the same generic "Copy
   * failed" message). On success the toast also reports the length so
   * the user has explicit confirmation that something landed on the
   * clipboard — this is the fix for the silent-failure bug report.
   */
  async function copyAsCurl(node: Node) {
    let out: string;
    try {
      out = await invoke<string>("requests_copy_as", {
        args: {
          source_kind: "project",
          source_path: node.rel_path,
          project_path: projectPath || null,
          env_name: $currentEnv,
          target: "curl",
          overrides: {},
        },
      });
    } catch (err) {
      addToast({ message: `Copy as cURL failed: ${err}`, type: "error" });
      return;
    }
    try {
      await navigator.clipboard.writeText(out);
    } catch (err) {
      addToast({ message: `Clipboard write failed: ${err}`, type: "error" });
      return;
    }
    addToast({
      message: `Copied ${out.length}-char cURL command`,
      type: "success",
    });
  }

  /** Duplicate a leaf in place; selects the new copy on success. */
  async function duplicate(node: Node) {
    try {
      const newPath = await invoke<string>("requests_duplicate", {
        sourceKind: "project",
        sourcePath: node.rel_path,
        projectPath: projectPath || null,
      });
      treeReloadSignal.update((n) => n + 1);
      currentSource.set({ kind: "project", path: newPath });
    } catch (err) {
      addToast({ message: `Duplicate failed: ${err}`, type: "error" });
    }
  }

  /**
   * Begin renaming a leaf. The row is replaced by an inline input via
   * `pendingRename`. The seed value is the relative path so users can
   * also re-folder during the rename.
   */
  function startRename(node: Node) {
    pendingRename = { node };
    renameValue = node.rel_path;
  }

  /** Cancel rename without firing the IPC. */
  function cancelRename() {
    pendingRename = null;
    renameValue = "";
  }

  /** Commit rename: invoke the backend, refresh the tree. */
  async function commitRename() {
    if (!pendingRename) return;
    const { node } = pendingRename;
    const trimmed = renameValue.trim();
    if (!trimmed) {
      cancelRename();
      return;
    }
    const finalValue = trimmed.toLowerCase().endsWith(".http")
      ? trimmed
      : `${trimmed}.http`;
    if (finalValue === node.rel_path) {
      cancelRename();
      return;
    }
    try {
      await invoke("requests_rename", {
        sourceKind: "project",
        fromPath: node.rel_path,
        toPath: finalValue,
        projectPath: projectPath || null,
      });
      // If the renamed item was the active source, follow it so the
      // editor doesn't lose its document.
      if (
        $currentSource?.kind === "project" &&
        $currentSource.path === node.rel_path
      ) {
        currentSource.set({ kind: "project", path: finalValue });
      }
      treeReloadSignal.update((n) => n + 1);
    } catch (err) {
      addToast({ message: `Rename failed: ${err}`, type: "error" });
    } finally {
      cancelRename();
    }
  }

  /**
   * Reveal a project leaf in the OS' default editor for `.http` files.
   *
   * Uses the dedicated backend command instead of `tauri-plugin-opener`
   * because the latter enforces a capability allowlist and rejects
   * paths under `<project>/.beardgit/requests/...` ("not allowed to
   * open path" errors). The backend command shells out to the
   * platform-native opener (`open` / `xdg-open` / `start`) directly.
   */
  async function openInEditor(node: Node) {
    if (!projectPath) return;
    try {
      await invoke("requests_open_in_editor", {
        sourceKind: "project",
        sourcePath: node.rel_path,
        projectPath,
      });
    } catch (err) {
      addToast({ message: `Open failed: ${err}`, type: "error" });
    }
  }

  /** Run the delete after the user confirmed in the ConfirmDialog overlay. */
  async function confirmDelete() {
    const target = pendingDelete;
    pendingDelete = null;
    if (!target) return;
    try {
      await invoke("requests_delete", {
        sourceKind: "project",
        sourcePath: target.node.rel_path,
        projectPath: projectPath || null,
      });
      // Drop the active source if it was the deleted item, else the
      // editor would render a stale doc.
      if (
        $currentSource?.kind === "project" &&
        $currentSource.path === target.node.rel_path
      ) {
        currentSource.set(null);
      }
      treeReloadSignal.update((n) => n + 1);
    } catch (err) {
      addToast({ message: `Delete failed: ${err}`, type: "error" });
    }
  }

  $: projectPath, $treeReloadSignal, reload();
  onMount(reload);
</script>

<div class="collections" data-testid="requests-collections-tree">
  <!-- PROJECT section -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="section-header"
    role="button"
    tabindex="0"
    onclick={() => (projectCollapsed = !projectCollapsed)}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") projectCollapsed = !projectCollapsed;
    }}
  >
    <span class="section-chevron nf" class:collapsed={projectCollapsed}>{""}</span>
    <span class="section-label">REQUESTS</span>
    <span class="section-count">{fileCount(project)}</span>
    <span class="section-actions">
      <Button
        variant="neutral"
        size="xs"
        icon={""}
        onclick={(e) => openNewRequest(e)}
        description="New request"
        testid="new-project-request-btn"
      >
        New
      </Button>
    </span>
  </div>

  {#if !projectCollapsed}
    {#if project.length === 0}
      <div class="list-empty">No requests yet.</div>
    {:else}
      {#each project as node (node.rel_path)}
        {@render treeNode(node, 0)}
      {/each}
    {/if}
  {/if}

  {#snippet treeNode(node: Node, depth: number)}
    {#if node.kind === "folder"}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="tree-folder"
        style="padding-left: {depth * 16 + 12}px"
        role="button"
        tabindex="0"
        onclick={() => toggleFolder(node.rel_path)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") toggleFolder(node.rel_path);
        }}
      >
        <span class="folder-chevron nf" class:open={folderOpen[node.rel_path] !== false}
          >{""}</span
        >
        <span class="folder-icon nf"
          >{folderOpen[node.rel_path] !== false ? "" : ""}</span
        >
        <span class="folder-name">{node.name}</span>
      </div>
      {#if folderOpen[node.rel_path] !== false}
        {#each node.children as child (child.rel_path)}
          {@render treeNode(child, depth + 1)}
        {/each}
      {/if}
    {:else if pendingRename && pendingRename.node.rel_path === node.rel_path}
      <div class="tree-leaf rename" style="padding-left: {depth * 16 + 12}px">
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          class="rename-input"
          bind:value={renameValue}
          onkeydown={(e) => {
            if (e.key === "Enter") commitRename();
            else if (e.key === "Escape") cancelRename();
          }}
          onblur={commitRename}
          autofocus
          data-testid="rename-input"
        />
      </div>
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="tree-leaf"
        class:selected={isSelected(node)}
        style="padding-left: {depth * 16 + 12}px"
        role="button"
        tabindex="0"
        onclick={() => open(node)}
        onkeydown={(e) => {
          if (e.key === "Enter") open(node);
        }}
        oncontextmenu={(e) => showCtxMenu(e, node)}
      >
        <span class="leaf-icon nf" aria-hidden="true">{""}</span>
        {#if node.method}
          <span
            class="method-badge method-badge--{node.method.toLowerCase()}"
            >{node.method}</span
          >
        {/if}
        <span class="leaf-name">{node.name}</span>
      </div>
    {/if}
  {/snippet}

  <!--
    TODO: folder context menu (Rename / Delete recursive). Skipped for
    v1 — folder rename and recursive delete need extra confirmation
    UX that's out of scope for the leaf-CRUD pass.
  -->
</div>

<NewRequestDialog
  bind:open={$newRequestOpen}
  existingNodes={project}
  onClose={() => newRequestOpen.set(false)}
/>

{#if ctxMenu}
  <ContextMenu
    items={buildLeafMenu(ctxMenu.node)}
    x={ctxMenu.x}
    y={ctxMenu.y}
    visible={true}
    onClose={() => (ctxMenu = null)}
  />
{/if}

{#if pendingDelete}
  <ConfirmDialog
    title="Delete request?"
    detail={pendingDelete.node.name}
    message={`Permanently delete "${pendingDelete.node.name}". This cannot be undone.`}
    confirmLabel="Delete"
    destructive={true}
    onConfirm={confirmDelete}
    onCancel={() => (pendingDelete = null)}
  />
{/if}

<style>
  .collections {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  /* ------------------------------------------------------------------
   * Section header — same recipe as BranchList's LOCAL/REMOTE bar so
   * the Requests panel sidebar lines up visually with the Branches
   * sidebar.
   * ------------------------------------------------------------------ */

  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    cursor: pointer;
    user-select: none;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .section-header:hover {
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
  }

  .section-chevron {
    font-size: 9px;
    color: var(--text-secondary);
    transition: transform 0.15s;
    display: inline-block;
  }

  .section-chevron.collapsed {
    transform: rotate(0deg);
  }

  .section-chevron:not(.collapsed) {
    transform: rotate(90deg);
  }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    flex: 1;
  }

  .section-count {
    font-size: 10px;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    padding: 1px 6px;
    border-radius: 10px;
  }

  /*
   * Trailing slot for section-level CTAs (currently the per-section
   * "+ New" button). Rendered after the count pill so the visual
   * order is: chevron — label — count — action. Matches the spacing
   * of `.section-count` so the button sits flush against the section
   * edge without extra padding.
   */
  .section-actions {
    display: inline-flex;
    align-items: center;
    margin-left: 4px;
  }

  .list-empty {
    padding: 8px 12px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  /* ------------------------------------------------------------------
   * Tree rows — mirrors `BranchTreeNode` styles 1:1.
   * ------------------------------------------------------------------ */

  .tree-folder {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-top: 4px;
    padding-bottom: 4px;
    padding-right: 12px;
    cursor: pointer;
    user-select: none;
    color: var(--text-secondary);
    font-size: 12px;
    border-bottom: 1px solid transparent;
  }

  .tree-folder:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
    color: var(--text-primary);
  }

  .folder-chevron {
    font-size: 9px;
    transition: transform 0.15s;
    flex-shrink: 0;
  }

  .folder-chevron.open {
    transform: rotate(90deg);
  }

  .folder-icon {
    font-size: 13px;
    flex-shrink: 0;
    color: var(--accent-blue);
    opacity: 0.7;
  }

  .folder-name {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tree-leaf {
    display: flex;
    align-items: center;
    gap: 5px;
    padding-top: 5px;
    padding-bottom: 5px;
    padding-right: 12px;
    cursor: pointer;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-primary);
  }

  .tree-leaf:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .tree-leaf.selected {
    background: var(--overlay-accent-blue);
  }

  /*
   * Inline rename row: replaces the leaf body with a single text
   * input. Padding/height match the leaf row so the tree doesn't
   * jump when the input mounts. The input itself uses --bg-primary
   * for contrast against the surrounding section.
   */
  .tree-leaf.rename {
    cursor: text;
    padding-top: 2px;
    padding-bottom: 2px;
  }

  .rename-input {
    width: 100%;
    padding: 3px 6px;
    font-size: 12px;
    background: var(--bg-primary);
    border: 1px solid var(--accent-blue);
    border-radius: 3px;
    color: var(--text-primary);
    box-sizing: border-box;
  }

  .rename-input:focus {
    outline: none;
  }

  .leaf-icon {
    font-size: 12px;
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .tree-leaf.selected .leaf-icon,
  .tree-leaf:hover .leaf-icon {
    color: var(--accent-blue);
  }

  .leaf-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  /*
   * Tiny verb badge before the file name. Mirrors the tonal recipe used
   * by Button / BranchList's track-pip (color-mix tint + accent foreground)
   * so the colors track any theme tweaks applied to the accent palette.
   */
  .method-badge {
    flex-shrink: 0;
    display: inline-block;
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.4px;
    line-height: 1;
    padding: 2px 4px;
    border-radius: 3px;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--text-secondary) 14%, transparent);
  }

  .method-badge--get {
    color: var(--accent-blue);
    background: color-mix(in srgb, var(--accent-blue) 18%, transparent);
  }

  .method-badge--post {
    color: var(--accent-green);
    background: color-mix(in srgb, var(--accent-green) 18%, transparent);
  }

  .method-badge--put,
  .method-badge--patch {
    color: var(--accent-orange);
    background: color-mix(in srgb, var(--accent-orange) 18%, transparent);
  }

  .method-badge--delete {
    color: var(--accent-red);
    background: color-mix(in srgb, var(--accent-red) 18%, transparent);
  }
</style>
