<!--
  PathTree — collapsible path tree or flat list of paths.

  Below `autoFlattenThreshold` items: flat list.
  Above:  collapsible folder tree with aggregate stats (file count +
  optional meta aggregator).

  Pure presentational; no store access. Callers own data + selection.
-->
<script lang="ts" generics="M">
  interface Item { path: string; meta?: M; }
  interface Props {
    items: Item[];
    autoFlattenThreshold?: number;
    selectedPath: string | null;
    onSelect?: (path: string) => void;
    /**
     * Optional reducer for folder aggregate rendering. Invoked once per
     * folder with the flat list of descendant items; return a string
     * rendered next to the folder name (e.g. "+12 −3 · 4 files").
     */
    aggregateLabel?: (descendants: Item[]) => string;
  }
  let {
    items,
    autoFlattenThreshold = 20,
    selectedPath,
    onSelect,
    aggregateLabel,
  }: Props = $props();

  interface Node { name: string; fullPath: string; isFolder: boolean; item?: Item; children: Node[]; }

  function buildTree(list: Item[]): Node[] {
    const root: Node[] = [];
    const getChild = (level: Node[], name: string, fullPath: string, isFolder: boolean) => {
      const existing = level.find((n) => n.name === name && n.isFolder === isFolder);
      if (existing) return existing;
      const created: Node = { name, fullPath, isFolder, children: [] };
      level.push(created);
      return created;
    };
    for (const item of list) {
      const parts = item.path.split("/");
      let level = root;
      for (let i = 0; i < parts.length; i++) {
        const isLeaf = i === parts.length - 1;
        const name = parts[i];
        const fullPath = parts.slice(0, i + 1).join("/");
        const node = getChild(level, name, fullPath, !isLeaf);
        if (isLeaf) node.item = item;
        level = node.children;
      }
    }
    return root;
  }

  let isTree = $derived(items.length > autoFlattenThreshold);
  let tree = $derived(isTree ? buildTree(items) : []);

  let expanded = $state(new Set<string>());
  function toggle(path: string) {
    expanded = new Set(expanded);
    if (expanded.has(path)) expanded.delete(path); else expanded.add(path);
  }

  /** Descend a folder and collect its leaf items — used by aggregateLabel. */
  function leaves(node: Node): Item[] {
    if (!node.isFolder && node.item) return [node.item];
    return node.children.flatMap(leaves);
  }
</script>

{#if isTree}
  <ul class="path-tree">
    {#each tree as node}
      {@render treeNode(node, 0)}
    {/each}
  </ul>
{:else}
  <ul class="path-flat">
    {#each items as item (item.path)}
      <li>
        <button
          data-pathtree-leaf
          class="leaf"
          class:selected={selectedPath === item.path}
          onclick={() => onSelect?.(item.path)}
          aria-label={item.path}
        >{item.path}</button>
      </li>
    {/each}
  </ul>
{/if}

{#snippet treeNode(node: Node, depth: number)}
  <li>
    {#if node.isFolder}
      <button
        data-pathtree-folder
        class="folder"
        style="padding-left: {depth * 12}px"
        onclick={() => toggle(node.fullPath)}
      >
        <span class="chev">{expanded.has(node.fullPath) ? "" : ""}</span>
        <span class="folder-name">{node.name}/</span>
        {#if aggregateLabel}<span class="agg">{aggregateLabel(leaves(node))}</span>{/if}
      </button>
      {#if expanded.has(node.fullPath)}
        <ul>
          {#each node.children as child}
            {@render treeNode(child, depth + 1)}
          {/each}
        </ul>
      {/if}
    {:else if node.item}
      <button
        data-pathtree-leaf
        class="leaf"
        style="padding-left: {depth * 12}px"
        class:selected={selectedPath === node.fullPath}
        onclick={() => onSelect?.(node.fullPath)}
        aria-label={node.fullPath}
      >{node.name}</button>
    {/if}
  </li>
{/snippet}

<style>
  .path-tree, .path-flat { list-style: none; padding: 0; margin: 0; }
  .folder, .leaf {
    display: flex; align-items: baseline; gap: 6px;
    width: 100%; background: none; border: none;
    text-align: left; cursor: pointer;
    font-family: var(--font-mono); font-size: 12px;
    color: var(--text-primary); padding: 3px 10px;
  }
  .folder:hover, .leaf:hover { background: rgba(255,255,255,0.04); }
  .leaf.selected { background: rgba(88,166,255,0.10); }
  .chev { font-family: var(--font-icons); font-size: 9px; width: 10px; color: var(--text-secondary); }
  .agg { margin-left: auto; font-size: 11px; color: var(--text-secondary); }
</style>
