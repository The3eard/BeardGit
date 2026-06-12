<script lang="ts">
  /**
   * AiConfigFileTree — dual file tree for project and user AI config files.
   *
   * Renders two collapsible sections (Project / User), each with a
   * hierarchical tree built from flat `AiConfigFile[]` paths. Supports
   * expand/collapse on folders, file selection, dirty indicator, and
   * a + button for creating new files.
   */
  import {
    configFiles,
    activeFilePath,
    activeFileDirty,
  } from "../../stores/aiConfig";
  import type { AiConfigFile } from "../../types";
  import * as m from "$lib/paraglide/messages";

  // ─── Props ───

  interface Props {
    onSelectFile: (path: string) => void;
    onCreateFile: (scope: string) => void;
  }

  let { onSelectFile, onCreateFile }: Props = $props();

  // ─── Tree types ───

  interface TreeNode {
    name: string;
    path: string;
    isFolder: boolean;
    kind: AiConfigFile["kind"] | null;
    children: TreeNode[];
  }

  // ─── Local state ───

  let projectCollapsed = $state(false);
  let userCollapsed = $state(false);

  /** Track collapsed state per folder path. Folders default to expanded. */
  let collapsedFolders = $state<Set<string>>(new Set());

  // ─── Derived: split files by scope ───

  let projectFiles = $derived(
    $configFiles.filter((f) => f.scope === "project" || f.scope === "local"),
  );

  let userFiles = $derived(
    $configFiles.filter((f) => f.scope === "user"),
  );

  // ─── Tree building ───

  /**
   * Extract the relative display path from an absolute file path.
   * Splits on `.claude/` to get the portion after it. For files like
   * CLAUDE.md at the repo root (no `.claude/` segment), uses the filename.
   */
  function relativePath(absPath: string): string {
    const claudeIdx = absPath.indexOf(".claude/");
    if (claudeIdx !== -1) {
      return absPath.substring(claudeIdx + ".claude/".length);
    }
    // Fallback: just the filename (for CLAUDE.md at repo root)
    const lastSlash = absPath.lastIndexOf("/");
    return lastSlash >= 0 ? absPath.substring(lastSlash + 1) : absPath;
  }

  /**
   * Build a hierarchical tree from a flat list of config files.
   * Groups intermediate path segments as folders.
   */
  function buildTree(files: AiConfigFile[]): TreeNode[] {
    const root: TreeNode[] = [];
    const folderMap = new Map<string, TreeNode>();

    for (const file of files) {
      const rel = relativePath(file.path);
      const parts = rel.split("/");
      let current = root;
      let pathSoFar = "";

      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const isLeaf = i === parts.length - 1;
        pathSoFar = pathSoFar ? `${pathSoFar}/${part}` : part;

        if (isLeaf) {
          // Check for duplicate
          if (!current.some((n) => n.path === file.path && !n.isFolder)) {
            current.push({
              name: part,
              path: file.path,
              isFolder: false,
              kind: file.kind,
              children: [],
            });
          }
        } else {
          let folder = folderMap.get(pathSoFar);
          if (!folder) {
            folder = {
              name: part,
              path: pathSoFar,
              isFolder: true,
              kind: null,
              children: [],
            };
            current.push(folder);
            folderMap.set(pathSoFar, folder);
          }
          current = folder.children;
        }
      }
    }

    return root;
  }

  let projectTree = $derived(buildTree(projectFiles));
  let userTree = $derived(buildTree(userFiles));

  // ─── Kind icons ───

  function kindIcon(kind: AiConfigFile["kind"] | null): string {
    switch (kind) {
      case "settings":
        return "\uF013";
      case "instructions":
        return "\uF15C";
      case "agent":
        return "\uF007";
      case "skill":
        return "\uF005";
      default:
        return "\uF15C";
    }
  }

  // ─── Folder toggle ───

  function toggleFolder(folderPath: string): void {
    const next = new Set(collapsedFolders);
    if (next.has(folderPath)) {
      next.delete(folderPath);
    } else {
      next.add(folderPath);
    }
    collapsedFolders = next;
  }

  function isFolderOpen(folderPath: string): boolean {
    return !collapsedFolders.has(folderPath);
  }
</script>

<!-- ─── Template ─── -->

<div class="file-tree">
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
    <span class="section-chevron nf" class:collapsed={projectCollapsed}>{"\uF054"}</span>
    <span class="section-label">{m.ai_config_project()}</span>
    <span class="section-count">{projectFiles.length}</span>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <button
      class="section-add nf"
      title={m.ai_config_create()}
      onclick={(e) => { e.stopPropagation(); onCreateFile("project"); }}
    >
      {"\uF067"}
    </button>
  </div>

  {#if !projectCollapsed}
    {#if projectFiles.length === 0}
      <div class="no-claude-banner">
        <span class="banner-icon nf">{"\uF449"}</span>
        <span class="banner-text">{m.ai_config_no_claude_md()}</span>
      </div>
    {:else}
      {#each projectTree as node (node.path)}
        {@render treeNode(node, 0)}
      {/each}
    {/if}
  {/if}

  <!-- USER section -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="section-header"
    role="button"
    tabindex="0"
    onclick={() => (userCollapsed = !userCollapsed)}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") userCollapsed = !userCollapsed;
    }}
  >
    <span class="section-chevron nf" class:collapsed={userCollapsed}>{"\uF054"}</span>
    <span class="section-label">{m.ai_config_user()}</span>
    <span class="section-count">{userFiles.length}</span>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <button
      class="section-add nf"
      title={m.ai_config_create()}
      onclick={(e) => { e.stopPropagation(); onCreateFile("user"); }}
    >
      {"\uF067"}
    </button>
  </div>

  {#if !userCollapsed}
    {#if userTree.length === 0}
      <div class="list-empty">{m.ai_config_user()}</div>
    {:else}
      {#each userTree as node (node.path)}
        {@render treeNode(node, 0)}
      {/each}
    {/if}
  {/if}
</div>

<!-- ─── Recursive tree node snippet ─── -->

{#snippet treeNode(node: TreeNode, depth: number)}
  {#if node.isFolder}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tree-folder"
      style:padding-left="{12 + depth * 16}px"
      onclick={() => toggleFolder(node.path)}
      onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") toggleFolder(node.path); }}
      role="button"
      tabindex="0"
    >
      <span class="folder-chevron nf" class:open={isFolderOpen(node.path)}>{"\uF054"}</span>
      <span class="folder-icon nf">{isFolderOpen(node.path) ? "\uF115" : "\uF114"}</span>
      <span class="folder-name">{node.name}</span>
    </div>
    {#if isFolderOpen(node.path)}
      {#each node.children as child (child.path)}
        {@render treeNode(child, depth + 1)}
      {/each}
    {/if}
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tree-leaf"
      class:selected={$activeFilePath === node.path}
      style:padding-left="{12 + depth * 16}px"
      onclick={() => onSelectFile(node.path)}
      onkeydown={(e) => { if (e.key === "Enter") onSelectFile(node.path); }}
      role="button"
      tabindex="0"
      title={node.path}
    >
      <span class="file-icon nf">{kindIcon(node.kind)}</span>
      <span class="file-name">{node.name}</span>
      {#if $activeFilePath === node.path && $activeFileDirty}
        <span class="dirty-dot" title="Unsaved changes"></span>
      {/if}
    </div>
  {/if}
{/snippet}

<style>
  .file-tree {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    overflow-x: hidden;
  }

  /* ─── Section header ─── */

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
    font-size: var(--font-size-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    flex: 1;
  }

  .section-count {
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    padding: 1px 6px;
    border-radius: 10px;
  }

  .section-add {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    line-height: 1;
  }

  .section-add:hover {
    color: var(--accent-primary);
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }

  /* ─── No CLAUDE.md banner ─── */

  .no-claude-banner {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    margin: 6px 8px;
    background: color-mix(in srgb, var(--accent-primary) 6%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent-primary) 15%, transparent);
    border-radius: 4px;
  }

  .banner-icon {
    font-size: var(--font-size-md);
    color: var(--accent-primary);
    flex-shrink: 0;
    margin-top: 1px;
  }

  .banner-text {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    line-height: 1.4;
  }

  /* ─── Empty state ─── */

  .list-empty {
    padding: 12px 16px;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    font-style: italic;
  }

  /* ─── Tree folder ─── */

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
    font-size: var(--font-size-sm);
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
    font-size: var(--font-size-md);
    flex-shrink: 0;
    color: var(--accent-yellow);
    opacity: 0.7;
  }

  .folder-name {
    font-size: var(--font-size-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ─── Tree leaf (file) ─── */

  .tree-leaf {
    display: flex;
    align-items: center;
    gap: 5px;
    padding-top: 5px;
    padding-bottom: 5px;
    padding-right: 12px;
    cursor: pointer;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .tree-leaf:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .tree-leaf.selected {
    background: var(--overlay-accent-blue);
  }

  .file-icon {
    font-size: var(--font-size-sm);
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .tree-leaf.selected .file-icon,
  .tree-leaf:hover .file-icon {
    color: var(--accent-primary);
  }

  .file-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .dirty-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-yellow);
    flex-shrink: 0;
  }
</style>
