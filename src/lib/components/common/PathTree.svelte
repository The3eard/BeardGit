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
    /**
     * When `true`, render Nerd Font folder / file glyphs alongside each
     * row. Off by default so existing consumers (PR diff lists) keep
     * their compact layout; the in-app file editor opts in.
     */
    showIcons?: boolean;
    /**
     * Pluggable per-leaf icon picker used only when `showIcons` is on.
     * Receives the leaf's basename and returns the glyph string. When
     * omitted, leaves fall back to the generic Nerd Font "file" glyph.
     */
    fileIconResolver?: (name: string) => string;
  }
  let {
    items,
    autoFlattenThreshold = 20,
    selectedPath,
    onSelect,
    aggregateLabel,
    showIcons = false,
    fileIconResolver,
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
    sortNodes(root);
    return root;
  }

  /** Recursively sort sibling nodes: folders first, then files, case-insensitive alpha. */
  function sortNodes(level: Node[]): void {
    level.sort((a, b) => {
      if (a.isFolder !== b.isFolder) return a.isFolder ? -1 : 1;
      return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
    });
    for (const node of level) {
      if (node.children.length > 0) sortNodes(node.children);
    }
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
        style="padding-left: {depth * 16}px"
        onclick={() => toggle(node.fullPath)}
        aria-label={node.fullPath}
      >
        {#if showIcons}
          <span class="ftype folder-icon"
            >{expanded.has(node.fullPath) ? "" : ""}</span
          >
        {/if}
        <span class="folder-name">{node.name}{showIcons ? "" : "/"}</span>
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
        style="padding-left: {depth * 16}px"
        class:selected={selectedPath === node.fullPath}
        onclick={() => onSelect?.(node.fullPath)}
        aria-label={node.fullPath}
      >
        {#if showIcons}
          <span class="ftype file-icon"
            >{fileIconResolver ? fileIconResolver(node.name) : ""}</span
          >
        {/if}
        <span class="leaf-name">{node.name}</span>
      </button>
    {/if}
  </li>
{/snippet}

<style>
  .path-tree, .path-flat { list-style: none; padding: 0; margin: 0; }
  .path-tree :global(ul), .path-flat :global(ul) { list-style: none; padding: 0; margin: 0; }
  .path-tree li, .path-flat li { list-style: none; }
  .folder, .leaf {
    display: flex; align-items: center; gap: 6px;
    width: 100%; background: none; border: none;
    text-align: left; cursor: pointer;
    font-family: var(--font-mono); font-size: 12px;
    color: var(--text-primary); padding: 3px 10px;
    line-height: 1.4;
  }
  .folder:hover, .leaf:hover { background: color-mix(in srgb, var(--text-primary) 4%, transparent); }
  .leaf.selected { background: var(--overlay-accent-blue); }
  .ftype { font-family: var(--font-icons); font-size: 14px; width: 16px; flex-shrink: 0; text-align: center; line-height: 1; }
  .folder-icon { color: var(--accent-primary); }
  .file-icon { color: var(--text-secondary); }
  .folder-name, .leaf-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .agg { margin-left: auto; font-size: 11px; color: var(--text-secondary); }
</style>
