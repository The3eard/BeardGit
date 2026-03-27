<script lang="ts">
  import { debounce } from "../../utils/debounce";
  import ContextMenu from "../common/ContextMenu.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import BranchTreeNode from "./BranchTreeNode.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import type { BranchTreeNode as TreeNode } from "./branch-tree";
  import {
    branches,
    branchesLoading,
    selectedBranchName,
    localBranches,
    remoteBranches,
    selectBranch,
    refreshBranches,
    doCheckout,
    doDeleteBranch,
    doMergeBranch,
  } from "../../stores/branches";
  import type { BranchInfo } from "../../types";

  function buildTree(branchList: BranchInfo[]): TreeNode[] {
    const root: TreeNode[] = [];
    const childMaps = new WeakMap<TreeNode[], Map<string, TreeNode>>();

    function getMap(children: TreeNode[]): Map<string, TreeNode> {
      let map = childMaps.get(children);
      if (!map) {
        map = new Map();
        childMaps.set(children, map);
      }
      return map;
    }

    for (const branch of branchList) {
      const parts = branch.name.split("/");
      let current = root;

      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const isLeaf = i === parts.length - 1;
        const key = `${part}:${isLeaf ? "leaf" : "folder"}`;
        const map = getMap(current);

        let existing = map.get(key);
        if (!existing) {
          existing = {
            name: part,
            fullPath: isLeaf ? branch.name : parts.slice(0, i + 1).join("/"),
            isFolder: !isLeaf,
            isHead: isLeaf && branch.is_head,
            isRemote: branch.is_remote,
            oid: isLeaf ? branch.oid : "",
            children: [],
          };
          current.push(existing);
          map.set(key, existing);
        }
        if (!isLeaf) {
          current = existing.children;
        }
      }
    }
    return root;
  }

  let filterInput = $state("");
  let filterValue = $state("");

  const applyFilter = debounce((value: string) => {
    filterValue = value;
  }, 150);

  function onFilterInput(value: string) {
    filterInput = value;
    applyFilter(value);
  }

  let filteredLocal = $derived(
    filterValue
      ? $localBranches.filter((b) => b.name.toLowerCase().includes(filterValue.toLowerCase()))
      : $localBranches,
  );

  let filteredRemote = $derived(
    filterValue
      ? $remoteBranches.filter((b) => b.name.toLowerCase().includes(filterValue.toLowerCase()))
      : $remoteBranches,
  );

  let localTree = $derived(buildTree(filteredLocal));
  let remoteTree = $derived(buildTree(filteredRemote));

  let localCollapsed = $state(false);
  let remoteCollapsed = $state(false);

  // Context menu state
  let menuVisible = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let contextBranch = $state("");
  let contextIsRemote = $state(false);
  let confirmDelete = $state<string | null>(null);

  let menuItems: MenuItem[] = $derived.by(() => {
    const items: MenuItem[] = [];
    if (!contextIsRemote) {
      items.push({ label: "Checkout", action: () => doCheckout(contextBranch) });
    }
    items.push({ label: "New branch from here [WIP]", action: () => {} });
    items.push({ label: "Merge into current", action: () => doMergeBranch(contextBranch) });
    items.push({ label: "Rebase onto [WIP]", action: () => {} });
    if (!contextIsRemote) {
      items.push({
        label: "Delete",
        action: () => {
          confirmDelete = contextBranch;
        },
      });
    }
    items.push({ label: "Push [WIP]", action: () => {} });
    return items;
  });

  function handleContextMenu(e: MouseEvent, node: TreeNode) {
    if (node.isFolder) return;
    e.preventDefault();
    contextBranch = node.fullPath;
    contextIsRemote = node.isRemote;
    menuX = e.clientX;
    menuY = e.clientY;
    menuVisible = true;
  }

  function handleRefresh() {
    filterInput = "";
    filterValue = "";
    refreshBranches();
  }
</script>

<div class="branch-list">
  <!-- Header -->
  <div class="list-header">
    <span class="list-title">BRANCHES</span>
    <button
      class="refresh-btn nf"
      onclick={handleRefresh}
      disabled={$branchesLoading}
      title="Refresh"
    >
      {$branchesLoading ? "\uF110" : "\uF021"}
    </button>
  </div>

  <!-- Filter input -->
  <div class="filter-row">
    <input
      type="text"
      class="filter-input"
      placeholder="Filter branches…"
      value={filterInput}
      oninput={(e) => onFilterInput(e.currentTarget.value)}
    />
  </div>

  <!-- Branch tree -->
  <div class="list-items">
    <!-- LOCAL section -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="section-header"
      role="button"
      tabindex="0"
      onclick={() => (localCollapsed = !localCollapsed)}
      onkeydown={(e) => {
        if (e.key === "Enter" || e.key === " ") localCollapsed = !localCollapsed;
      }}
    >
      <span class="section-chevron nf" class:collapsed={localCollapsed}>{"\uF054"}</span>
      <span class="section-label">LOCAL</span>
      <span class="section-count">{$localBranches.length}</span>
    </div>

    {#if !localCollapsed}
      {#if $branchesLoading && $branches.length === 0}
        <div class="list-loading">
          <div class="spinner"></div>
        </div>
      {:else if localTree.length === 0}
        <div class="list-empty">No local branches</div>
      {:else}
        {#each localTree as node (node.fullPath)}
          <BranchTreeNode
            {node}
            depth={0}
            selected={$selectedBranchName}
            onSelect={selectBranch}
            onContext={handleContextMenu}
          />
        {/each}
      {/if}
    {/if}

    <!-- REMOTE section -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="section-header"
      role="button"
      tabindex="0"
      onclick={() => (remoteCollapsed = !remoteCollapsed)}
      onkeydown={(e) => {
        if (e.key === "Enter" || e.key === " ") remoteCollapsed = !remoteCollapsed;
      }}
    >
      <span class="section-chevron nf" class:collapsed={remoteCollapsed}>{"\uF054"}</span>
      <span class="section-label">REMOTE</span>
      <span class="section-count">{$remoteBranches.length}</span>
    </div>

    {#if !remoteCollapsed}
      {#if remoteTree.length === 0}
        <div class="list-empty">No remote branches</div>
      {:else}
        {#each remoteTree as node (node.fullPath)}
          <BranchTreeNode
            {node}
            depth={0}
            selected={$selectedBranchName}
            onSelect={selectBranch}
            onContext={handleContextMenu}
          />
        {/each}
      {/if}
    {/if}
  </div>
</div>

<ContextMenu
  items={menuItems}
  x={menuX}
  y={menuY}
  visible={menuVisible}
  onClose={() => (menuVisible = false)}
/>

{#if confirmDelete !== null}
  <ConfirmDialog
    title="Delete Branch"
    detail={confirmDelete}
    message={`Are you sure you want to delete branch "${confirmDelete}"? This action cannot be undone.`}
    confirmLabel="Delete"
    destructive={true}
    onConfirm={() => {
      doDeleteBranch(confirmDelete!);
      confirmDelete = null;
    }}
    onCancel={() => (confirmDelete = null)}
  />
{/if}

<style>
  .branch-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .list-header {
    flex-shrink: 0;
  }

  .list-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .filter-row {
    padding: 8px;
    flex-shrink: 0;
  }

  .filter-input {
    width: 100%;
    padding: 5px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    box-sizing: border-box;
  }

  .filter-input:focus {
    border-color: var(--accent-blue);
  }

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
    background: rgba(255, 255, 255, 0.04);
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
    background: rgba(255, 255, 255, 0.06);
    padding: 1px 6px;
    border-radius: 10px;
  }

  .list-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }

  .list-empty {
    padding: 12px 16px;
    font-size: 11px;
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
