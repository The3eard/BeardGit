<!--
  GitGraph — Canvas-rendered commit graph with virtual scrolling.

  Renders the commit DAG using the graph-renderer module. Handles scroll
  events to load new viewport chunks, click/double-click for commit selection,
  keyboard navigation, and a context menu for per-commit operations
  (cherry-pick, checkout, create branch). The search bar supports branch,
  author, message, and SHA filters.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { viewport, selectedOid, selectedGroup, graphOffset, loadViewport, selectCommit, userEmails } from "../../stores/graph";
  import { repoInfo } from "../../stores/repo";
  import { renderGraph, hitTest, graphHitTest, ROW_HEIGHT, DEFAULT_COLUMNS, DEFAULT_GRAPH_THEME, type GraphColumn } from "./graph-renderer";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import { cherryPick, checkoutBranch, createBranch } from "../../api/tauri";
  import { debounce } from "../../utils/debounce";
  import SearchBar from "../common/SearchBar.svelte";
  import { activeTheme, buildGraphTheme } from "../../stores/theme";
  import type { SearchTag } from "../../search/types";
  import type { GraphViewport as GraphViewportType } from "../../types";
  import { graphFilters, filterGraphRemote } from "../../search/graph-provider";
  import * as m from "$lib/paraglide/messages";

  // Column visibility state
  let columns = $state<GraphColumn[]>(DEFAULT_COLUMNS.map(c => ({ ...c })));
  let showColumnMenu = $state(false);
  let columnToggleEl: HTMLDivElement | undefined = $state();

  function toggleColumn(id: string) {
    columns = columns.map(c => c.id === id ? { ...c, visible: !c.visible } : c);
  }

  let canvas: HTMLCanvasElement;
  let container!: HTMLDivElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let graphSearchTags = $state<SearchTag[]>([]);
  let filteredViewport = $state<GraphViewportType | null>(null);
  let filteredOffset = $state(0);
  let searchLoading = $state(false);

  // Hover state — tracks which segment group the mouse is over
  let hoveredGroup = $state<number | null>(null);

  // Context menu state
  let contextMenuVisible = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let contextMenuItems = $state<MenuItem[]>([]);

  // Scroll state — accumulates sub-row delta for proportional scrolling
  let scrollAccumulator = 0;

  let drawRafId: number | null = null;

  function getActiveNodes() {
    const activeVp = filteredViewport ?? $viewport;
    if (!activeVp || activeVp.nodes.length === 0) return [];
    return activeVp.nodes;
  }

  let filteredNodes = $derived(getActiveNodes());
  let displayNodes = $derived.by(() => {
    if (filteredViewport) {
      const visibleCount = container ? Math.ceil(container.clientHeight / ROW_HEIGHT) + 2 : 20;
      const sliced = filteredViewport.nodes.slice(filteredOffset, filteredOffset + visibleCount);
      return sliced.map((n, i) => ({ ...n, row: filteredOffset + i }));
    }
    return filteredNodes;
  });
  let isFiltering = $derived(graphSearchTags.length > 0);

  async function handleGraphSearch(tags: SearchTag[]) {
    graphSearchTags = tags;
    filteredOffset = 0;
    if (tags.length === 0) {
      filteredViewport = null;
      return;
    }
    searchLoading = true;
    try {
      const result = await filterGraphRemote(tags);
      filteredViewport = result;
    } catch {
      filteredViewport = null;
    } finally {
      searchLoading = false;
    }
  }

  function draw() {
    if (!ctx || !canvas) return;

    const activeVp = filteredViewport ?? $viewport;
    if (!activeVp || filteredNodes.length === 0) {
      ctx.clearRect(0, 0, canvas.width / window.devicePixelRatio, canvas.height / window.devicePixelRatio);
      ctx.fillStyle = "#888888";
      ctx.font = "13px -apple-system, BlinkMacSystemFont, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      const msg = searchLoading ? m.graph_searching() : isFiltering ? m.graph_no_matching() : m.graph_no_commits();
      ctx.fillText(msg, (canvas.width / window.devicePixelRatio) / 2, (canvas.height / window.devicePixelRatio) / 2);
      ctx.textAlign = "left";
      return;
    }

    const canvasW = canvas.width / window.devicePixelRatio;
    const canvasH = canvas.height / window.devicePixelRatio;

    const graphTheme = $activeTheme ? buildGraphTheme($activeTheme) : DEFAULT_GRAPH_THEME;

    if (filteredViewport) {
      // Slice filtered nodes for offset-based scrolling within filtered results
      const visibleCount = Math.ceil(canvasH / ROW_HEIGHT) + 2;
      const slicedNodes = filteredViewport.nodes.slice(filteredOffset, filteredOffset + visibleCount);
      const displayNodes = slicedNodes.map((n, i) => ({ ...n, row: filteredOffset + i }));
      renderGraph(
        ctx,
        displayNodes,
        filteredOffset,
        canvasW,
        canvasH,
        filteredViewport.total_lane_count,
        $selectedOid,
        columns,
        filteredViewport.lane_segments ?? [],
        filteredViewport.merge_curves ?? [],
        graphTheme,
        filteredViewport.head_lane ?? null,
        $userEmails,
        $selectedGroup,
        hoveredGroup,
      );
    } else {
      renderGraph(
        ctx,
        filteredNodes,
        activeVp.offset,
        canvasW,
        canvasH,
        activeVp.total_lane_count,
        $selectedOid,
        columns,
        activeVp.lane_segments ?? [],
        activeVp.merge_curves ?? [],
        graphTheme,
        activeVp.head_lane ?? null,
        $userEmails,
        $selectedGroup,
        hoveredGroup,
      );
    }
  }

  function resizeCanvas() {
    if (!canvas || !container) return;
    const dpr = window.devicePixelRatio || 1;
    const rect = container.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    if (ctx) {
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }
    draw();
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();

    if (filteredViewport) {
      const rowDelta = Math.sign(e.deltaY) * Math.max(1, Math.round(Math.abs(e.deltaY) / ROW_HEIGHT));
      const visibleRows = container ? Math.floor(container.clientHeight / ROW_HEIGHT) : 20;
      const maxOffset = Math.max(0, filteredViewport.total_count - visibleRows);
      filteredOffset = Math.max(0, Math.min(filteredOffset + rowDelta, maxOffset));
      draw();
      return;
    }

    const vp = $viewport;
    if (!vp) return;

    // Accumulate delta — convert to rows when enough accumulates
    scrollAccumulator += e.deltaY;
    const rowDelta = Math.trunc(scrollAccumulator / ROW_HEIGHT);
    if (rowDelta === 0) return;
    scrollAccumulator -= rowDelta * ROW_HEIGHT;

    const newOffset = Math.max(0, Math.min($graphOffset + rowDelta, vp.total_count - 1));
    if (newOffset !== $graphOffset) {
      loadViewport(newOffset);
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!canvas || !ctx) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const activeVp = filteredViewport ?? $viewport;
    if (!activeVp || activeVp.nodes.length === 0) {
      hoveredGroup = null;
      canvas.style.cursor = "default";
      return;
    }

    const currentOffset = filteredViewport ? filteredOffset : activeVp.offset;
    const activeNodes = displayNodes;
    const activeSegments = activeVp.lane_segments ?? [];

    const hit = graphHitTest(x, y, currentOffset, activeNodes, activeVp.visible_lane_count ?? activeVp.total_lane_count, activeSegments);

    if (hit.type === "node") {
      canvas.style.cursor = "pointer";
      hoveredGroup = null;
    } else if (hit.type === "segment" && hit.groupId !== undefined) {
      canvas.style.cursor = "pointer";
      hoveredGroup = hit.groupId;
    } else {
      canvas.style.cursor = "default";
      hoveredGroup = null;
    }
  }

  function handleClick(e: MouseEvent) {
    if (!canvas || !ctx) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const activeVp = filteredViewport ?? $viewport;
    if (!activeVp || activeVp.nodes.length === 0) return;
    const currentOffset = filteredViewport ? filteredOffset : activeVp.offset;

    const activeNodes = displayNodes;
    const activeSegments = activeVp.lane_segments ?? [];

    const hit = graphHitTest(x, y, currentOffset, activeNodes, activeVp.visible_lane_count ?? activeVp.total_lane_count, activeSegments);

    if (hit.type === "node" && hit.row !== undefined) {
      selectedGroup.set(null);
      if (filteredViewport) {
        const nodeIdx = hit.row - filteredOffset;
        const node = filteredViewport.nodes[filteredOffset + nodeIdx];
        if (node) selectCommit(node.oid);
      } else {
        const node = filteredNodes.find((n) => n.row === hit.row);
        if (node) selectCommit(node.oid);
      }
    } else if (hit.type === "segment" && hit.groupId !== undefined) {
      const currentGroup = get(selectedGroup);
      selectedGroup.set(currentGroup === hit.groupId ? null : hit.groupId);
    } else {
      selectedGroup.set(null);
    }
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    const activeVp = filteredViewport ?? $viewport;
    if (!activeVp || filteredNodes.length === 0) return;

    const rect = canvas.getBoundingClientRect();
    const y = e.clientY - rect.top;

    const offset = filteredViewport ? filteredOffset : activeVp.offset;
    const rowIndex = hitTest(y, offset, displayNodes.length);
    if (rowIndex === null) return;

    const node = displayNodes.find((n) => n.row === rowIndex);
    if (!node) return;

    // Only highlight visually — don't open detail panel on right-click
    selectedOid.set(node.oid);

    const shortOid = node.oid.substring(0, 7);

    contextMenuItems = [
      {
        label: m.graph_copy_sha({ sha: shortOid }),
        action: () => navigator.clipboard.writeText(node.oid),
      },
      {
        label: m.graph_copy_message(),
        action: () => navigator.clipboard.writeText(node.summary),
      },
      { label: "", action: () => {}, separator: true },
      {
        label: m.graph_create_branch({ sha: shortOid }),
        action: async () => {
          const name = prompt(m.graph_branch_name_prompt());
          if (name) {
            try {
              await createBranch(name);
              // TODO: refresh graph after branch creation
            } catch (err) {
              alert(m.graph_branch_failed({ error: String(err) }));
            }
          }
        },
      },
      {
        label: m.graph_cherry_pick({ sha: shortOid }),
        action: async () => {
          try {
            await cherryPick(node.oid);
          } catch (err) {
            alert(m.graph_cherry_pick_failed({ error: String(err) }));
          }
        },
      },
      { label: "", action: () => {}, separator: true },
      {
        label: m.graph_checkout({ sha: shortOid }),
        action: async () => {
          try {
            // For commits with refs, checkout the branch name
            if (node.refs.length > 0) {
              const branchRef = node.refs.find(r => !r.startsWith("refs/remotes/") && !r.startsWith("refs/tags/"));
              if (branchRef) {
                const branchName = branchRef.replace("refs/heads/", "");
                await checkoutBranch(branchName);
                return;
              }
            }
            alert(m.graph_checkout_detached());
          } catch (err) {
            alert(m.graph_checkout_failed({ error: String(err) }));
          }
        },
      },
    ];

    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
    contextMenuVisible = true;
  }

  function handleWindowClick(e: MouseEvent) {
    if (showColumnMenu && columnToggleEl && !columnToggleEl.contains(e.target as Node)) {
      showColumnMenu = false;
    }
  }

  onMount(() => {
    ctx = canvas.getContext("2d");
    resizeCanvas();

    const debouncedResize = debounce(resizeCanvas, 100);
    const resizeObserver = new ResizeObserver(() => debouncedResize());
    resizeObserver.observe(container);

    window.addEventListener("click", handleWindowClick);

    if ($repoInfo) {
      loadViewport(0);
    }

    return () => {
      resizeObserver.disconnect();
      window.removeEventListener("click", handleWindowClick);
    };
  });

  $effect(() => {
    $viewport;
    $selectedOid;
    $selectedGroup;
    $userEmails;
    $activeTheme;
    columns;
    filteredNodes;
    filteredViewport;
    filteredOffset;
    searchLoading;
    hoveredGroup;
    if (drawRafId !== null) cancelAnimationFrame(drawRafId);
    drawRafId = requestAnimationFrame(() => {
      drawRafId = null;
      draw();
    });
  });

  $effect(() => {
    if ($repoInfo) {
      scrollAccumulator = 0;
      selectedGroup.set(null);
      loadViewport(0);
    }
  });

  let activeVpDerived = $derived(filteredViewport ?? $viewport);
  let rangeStart = $derived(
    filteredViewport ? filteredOffset + 1 : (activeVpDerived ? activeVpDerived.offset + 1 : 0)
  );
  let rangeEnd = $derived(
    filteredViewport
      ? Math.min(filteredOffset + Math.ceil((container?.clientHeight ?? 600) / ROW_HEIGHT), filteredViewport.total_count)
      : (activeVpDerived
        ? Math.min(activeVpDerived.offset + activeVpDerived.nodes.length, activeVpDerived.total_count)
        : 0)
  );
  let totalCount = $derived(activeVpDerived?.total_count ?? 0);
</script>

<div class="git-graph">
  <div class="graph-header">
    <SearchBar
      filters={graphFilters}
      bind:tags={graphSearchTags}
      placeholder={m.graph_search_placeholder()}
      onSearch={handleGraphSearch}
    />
    <div class="column-toggle" bind:this={columnToggleEl}>
      <button class="columns-btn" onclick={() => showColumnMenu = !showColumnMenu}>
        {m.graph_columns()}
      </button>
      {#if showColumnMenu}
        <div class="column-dropdown">
          {#each columns as col}
            <label class="column-option">
              <input type="checkbox" checked={col.visible} onchange={() => toggleColumn(col.id)} />
              {col.label}
            </label>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <div class="graph-canvas-container" bind:this={container}>
    <canvas
      bind:this={canvas}
      onwheel={handleWheel}
      onclick={handleClick}
      onmousemove={handleMouseMove}
      oncontextmenu={handleContextMenu}
    ></canvas>
  </div>

  <div class="graph-footer">
    {#if searchLoading}
      <span>{m.graph_searching()}</span>
    {:else if totalCount > 0}
      {#if isFiltering}
        <span>{m.graph_results_range({ start: String(rangeStart), end: String(rangeEnd), total: String(totalCount) })}</span>
      {:else}
        <span>{m.graph_commits_range({ start: String(rangeStart), end: String(rangeEnd), total: String(totalCount) })}</span>
      {/if}
    {:else}
      <span>{m.graph_no_commits_short()}</span>
    {/if}
  </div>
</div>

<ContextMenu
  items={contextMenuItems}
  x={contextMenuX}
  y={contextMenuY}
  visible={contextMenuVisible}
  onClose={() => contextMenuVisible = false}
/>

<style>
  .git-graph {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .graph-header {
    display: flex;
    gap: 8px;
    align-items: center;
    background: var(--bg-secondary);
    padding-right: 8px;
  }

  .graph-header :global(.search-bar) {
    flex: 1;
    min-width: 0;
  }

  .column-toggle {
    position: relative;
  }

  .columns-btn {
    padding: 4px 8px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
  }

  .columns-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }

  .column-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 0;
    min-width: 140px;
    z-index: 100;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .column-option {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .column-option:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .column-option input[type="checkbox"] {
    accent-color: var(--accent-blue);
  }


  .graph-canvas-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }

  .graph-canvas-container canvas {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    cursor: default;
  }

  .graph-footer {
    padding: 4px 8px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
    font-size: 11px;
    color: var(--text-secondary);
    text-align: center;
  }
</style>
