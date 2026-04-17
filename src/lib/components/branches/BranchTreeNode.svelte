<script lang="ts">
  import BranchTreeNode from "./BranchTreeNode.svelte";
  import type { BranchTreeNode as TreeNode } from "./branch-tree";
  import { shortOid } from "../../utils/git";

  let {
    node,
    depth,
    selected,
    onSelect,
    onContext,
  }: {
    node: TreeNode;
    depth: number;
    selected: string | null;
    onSelect: (name: string) => void;
    onContext: (e: MouseEvent, node: TreeNode) => void;
  } = $props();

  let folderOpen = $state(true);

  let indent = $derived(depth * 16 + 12);
</script>

{#if node.isFolder}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="tree-folder"
    style="padding-left: {indent}px"
    onclick={() => (folderOpen = !folderOpen)}
    onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") folderOpen = !folderOpen; }}
    role="button"
    tabindex="0"
  >
    <span class="folder-chevron nf" class:open={folderOpen}>{"\uF054"}</span>
    <span class="folder-icon nf">{folderOpen ? "\uF115" : "\uF114"}</span>
    <span class="folder-name">{node.name}</span>
  </div>
  {#if folderOpen}
    {#each node.children as child (child.fullPath)}
      <BranchTreeNode
        node={child}
        depth={depth + 1}
        {selected}
        {onSelect}
        {onContext}
      />
    {/each}
  {/if}
{:else}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="tree-leaf"
    class:selected={selected === node.fullPath}
    class:is-head={node.isHead}
    class:is-remote={node.isRemote}
    style="padding-left: {indent}px"
    onclick={() => onSelect(node.fullPath)}
    oncontextmenu={(e) => onContext(e, node)}
    onkeydown={(e) => { if (e.key === "Enter") onSelect(node.fullPath); }}
    role="button"
    tabindex="0"
    data-testid="branch-row-{node.fullPath.replace(/\//g, '-')}"
  >
    <span class="branch-icon nf">{"\uF418"}</span>
    <span class="branch-name" class:head-name={node.isHead}>{node.name}</span>
    {#if node.isHead}
      <span class="head-dot" title="Current branch"></span>
    {/if}
    <span class="branch-oid">{shortOid(node.oid)}</span>
  </div>
{/if}

<style>
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
    background: rgba(255, 255, 255, 0.03);
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
    background: rgba(255, 255, 255, 0.03);
  }

  .tree-leaf.selected {
    background: rgba(88, 166, 255, 0.1);
  }

  .tree-leaf.is-remote {
    color: var(--text-secondary);
  }

  .branch-icon {
    font-size: 12px;
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .tree-leaf.selected .branch-icon,
  .tree-leaf:hover .branch-icon {
    color: var(--accent-blue);
  }

  .branch-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .branch-name.head-name {
    font-weight: 600;
    color: var(--text-primary);
  }

  .head-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-blue);
    flex-shrink: 0;
  }

  .branch-oid {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    flex-shrink: 0;
    opacity: 0.7;
  }
</style>
