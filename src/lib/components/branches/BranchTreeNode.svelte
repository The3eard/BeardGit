<script lang="ts">
  import BranchTreeNode from "./BranchTreeNode.svelte";
  import type { BranchTreeNode as TreeNode } from "./branch-tree";
  import { shortOid } from "../../utils/git";
  import * as m from "$lib/paraglide/messages";

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
    {#if node.ahead > 0}
      <span class="track-pip ahead" title="Ahead {node.ahead}">↑{node.ahead}</span>
    {/if}
    {#if node.behind > 0}
      <span class="track-pip behind" title="Behind {node.behind}">↓{node.behind}</span>
    {/if}
    {#if node.upstreamGone}
      <span class="track-pip gone" title={m.branch_gone_tooltip()}>{m.branch_gone_chip()}</span>
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
    font-size: var(--font-size-sm);
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
    font-size: var(--font-size-md);
    flex-shrink: 0;
    color: var(--accent-primary);
    opacity: 0.7;
  }

  .folder-name {
    font-size: var(--font-size-sm);
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
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .tree-leaf:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .tree-leaf.selected {
    background: var(--overlay-accent-blue);
  }

  .tree-leaf.is-remote {
    color: var(--text-secondary);
  }

  .branch-icon {
    font-size: var(--font-size-sm);
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .tree-leaf.selected .branch-icon,
  .tree-leaf:hover .branch-icon {
    color: var(--accent-primary);
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
    background: var(--accent-primary);
    flex-shrink: 0;
  }

  .branch-oid {
    font-size: var(--font-size-2xs);
    font-family: var(--font-mono);
    color: var(--text-secondary);
    flex-shrink: 0;
    opacity: 0.7;
  }

  .track-pip {
    font-size: var(--font-size-2xs);
    font-weight: 500;
    flex-shrink: 0;
    padding: 1px 5px;
    border-radius: 8px;
    line-height: 1;
  }

  .track-pip.ahead {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }

  .track-pip.behind {
    background: color-mix(in srgb, var(--accent-orange) 15%, transparent);
    color: var(--accent-orange);
  }

  .track-pip.gone {
    background: color-mix(in srgb, var(--accent-red) 15%, transparent);
    color: var(--accent-red);
    text-transform: lowercase;
    letter-spacing: 0.3px;
  }
</style>
