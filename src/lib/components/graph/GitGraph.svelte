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
  import { viewport, selectedOid, selectedGroup, graphOffset, loadViewport, selectCommit, userEmails, reloadGraph } from "../../stores/graph";
  import { repoInfo } from "../../stores/repo";
  import { renderGraph, hitTest, graphHitTest, getResizeTarget, ROW_HEIGHT, DEFAULT_COLUMNS, DEFAULT_GRAPH_THEME, type GraphColumn } from "./graph-renderer";
  import { getLastMetrics, getRollingFps } from "./graph-perf";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import { cherryPick, checkoutBranch, revertCommit, resetToCommit, rebaseBranch, getGraphColumns, setGraphColumns, createCommitPatches } from "../../api/tauri";
  import { runMutation } from "../../api/runMutation";
  import { openCreateBranchDialog } from "../../stores/createBranchDialog";
  import { buildCreateBranchSource } from "./GitGraph.helpers";
  import { save } from "@tauri-apps/plugin-dialog";
  import RebaseEditor from "../rebase/RebaseEditor.svelte";
  import { debounce } from "../../utils/debounce";
  import SearchBar from "../common/SearchBar.svelte";
  import { activeTheme, buildGraphTheme } from "../../stores/theme";
  import type { SearchTag } from "../../search/types";
  import type { GraphViewport as GraphViewportType } from "../../types";
  import { graphFilters, filterGraphRemote } from "../../search/graph-provider";
  import { mrPrByBranch } from "../../stores/mr-pr";
  import { activeProvider } from "../../stores/provider";
  import { shortOid } from "../../utils/git";
  import { bisectState, markGood, markBad, skipCommit } from "../../stores/bisect";
  import * as m from "$lib/paraglide/messages";
  import { Button } from "$lib/components/ui";

  // Column visibility state
  let columns = $state<GraphColumn[]>(DEFAULT_COLUMNS.map(c => ({ ...c })));
  // Stored DPR used for current canvas backing store — keeps draw() consistent with resizeCanvas()
  let canvasDpr = $state(window.devicePixelRatio || 1);
  let showColumnMenu = $state(false);
  let columnToggleEl: HTMLDivElement | undefined = $state();

  // Row hover state
  let hoveredRow = $state<number | null>(null);

  // Column resize state
  let resizingCol = $state<number>(-1);
  let resizeStartX = $state(0);
  let resizeStartWidth = $state(0);

  function toggleColumn(id: string) {
    columns = columns.map(c => c.id === id ? { ...c, visible: !c.visible } : c);
    persistColumns();
  }

  function persistColumns() {
    setGraphColumns(columns.map(c => ({ id: c.id, width: c.width, visible: c.visible })));
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

  // Dev-only perf overlay (toggle with Ctrl+Shift+P)
  let showPerfOverlay = $state(false);

  // Bisect overlay sets — derived from bisect store
  let bisectGoodSet = $derived(new Set($bisectState.good_commits));
  let bisectBadSet = $derived(new Set($bisectState.bad_commits));
  let bisectSkipSet = $derived(new Set<string>());
  let bisectCurrentOid = $derived($bisectState.active ? $bisectState.current_commit : null);

  // Context menu state
  let contextMenuVisible = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let contextMenuItems = $state<MenuItem[]>([]);

  // Interactive rebase editor state
  let showRebaseEditor = $state(false);
  let rebaseBaseOid = $state('');

  // Confirm dialog state
  let showConfirm = $state(false);
  let confirmProps = $state<{
    title: string;
    detail?: string;
    message: string;
    confirmLabel?: string;
    destructive?: boolean;
    onConfirm: () => void;
  }>({
    title: '',
    message: '',
    onConfirm: () => {},
  });

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

  /** Dev-only perf overlay — paints last-recorded metrics on top of the graph. */
  function drawPerfOverlay(ctx2: CanvasRenderingContext2D, canvasW: number) {
    if (!showPerfOverlay || !import.meta.env.DEV) return;
    const metrics = getLastMetrics();
    if (!metrics) return;

    const fps = getRollingFps();
    const lines = [
      `Total: ${metrics.totalMs.toFixed(2)}ms`,
      `Lanes: ${metrics.lanesMs.toFixed(2)}ms`,
      `Merges: ${metrics.mergesMs.toFixed(2)}ms`,
      `Nodes: ${metrics.nodesMs.toFixed(2)}ms`,
      `Badges+Text: ${metrics.badgesMs.toFixed(2)}ms`,
      `FPS: ${fps.toFixed(0)}`,
    ];

    const padding = 8;
    const lineHeight = 16;
    const overlayWidth = 160;
    const overlayHeight = lines.length * lineHeight + padding * 2;
    const overlayX = canvasW - overlayWidth - 8;
    const overlayY = 8;

    // Semi-transparent background
    ctx2.fillStyle = 'rgba(0, 0, 0, 0.75)'; /* beardgit:allow-hex: canvas-modal-backdrop, always dark */
    ctx2.fillRect(overlayX, overlayY, overlayWidth, overlayHeight);

    // Border
    ctx2.strokeStyle = 'rgba(255, 255, 255, 0.2)'; /* beardgit:allow-hex: canvas ctx requires concrete color */
    ctx2.lineWidth = 1;
    ctx2.strokeRect(overlayX, overlayY, overlayWidth, overlayHeight);

    // Text
    ctx2.font = '11px "SF Mono", "Fira Code", monospace';
    ctx2.fillStyle = '#00ff88'; /* beardgit:allow-hex: canvas perf overlay accent, not a UI surface */
    ctx2.textAlign = 'left';
    ctx2.textBaseline = 'top';

    for (let i = 0; i < lines.length; i++) {
      ctx2.fillText(
        lines[i],
        overlayX + padding,
        overlayY + padding + i * lineHeight,
      );
    }

    // Reset text alignment for subsequent draws
    ctx2.textAlign = 'left';
    ctx2.textBaseline = 'alphabetic';
  }

  function draw() {
    if (!ctx || !canvas) return;

    const activeVp = filteredViewport ?? $viewport;
    if (!activeVp || filteredNodes.length === 0) {
      ctx.clearRect(0, 0, canvas.width / canvasDpr, canvas.height / canvasDpr);
      // Cold-start path: skeleton DOM overlays the canvas and owns the
      // visual — bail out silently so we don't paint "No commits" text
      // over the lane stripes.
      if ($viewport === null && !filteredViewport && !isFiltering && !searchLoading) {
        return;
      }
      ctx.fillStyle = "#888888"; /* beardgit:allow-hex: canvas ctx requires concrete color; read from theme at call site would require async */
      ctx.font = "14px -apple-system, BlinkMacSystemFont, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      const msg = searchLoading ? m.graph_searching() : isFiltering ? m.graph_no_matching() : m.graph_no_commits();
      ctx.fillText(msg, (canvas.width / canvasDpr) / 2, (canvas.height / canvasDpr) / 2);
      ctx.textAlign = "left";
      return;
    }

    const canvasW = canvas.width / canvasDpr;
    const canvasH = canvas.height / canvasDpr;

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
        hoveredRow,
        $mrPrByBranch,
        $activeProvider?.kind === "github",
        bisectGoodSet,
        bisectBadSet,
        bisectSkipSet,
        bisectCurrentOid,
      );
      drawPerfOverlay(ctx, canvasW);
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
        hoveredRow,
        $mrPrByBranch,
        $activeProvider?.kind === "github",
        bisectGoodSet,
        bisectBadSet,
        bisectSkipSet,
        bisectCurrentOid,
      );
      drawPerfOverlay(ctx, canvasW);
    }
  }

  /** Schedule a draw on the next animation frame, coalescing rapid calls. */
  function scheduleDraw() {
    if (drawRafId !== null) cancelAnimationFrame(drawRafId);
    drawRafId = requestAnimationFrame(() => {
      drawRafId = null;
      draw();
    });
  }

  function resizeCanvas() {
    if (!canvas || !container) return;
    const dpr = window.devicePixelRatio || 1;
    canvasDpr = dpr;
    const rect = container.getBoundingClientRect();
    // Round to integer physical pixels to avoid subpixel blurriness
    const pxWidth = Math.round(rect.width * dpr);
    const pxHeight = Math.round(rect.height * dpr);
    canvas.width = pxWidth;
    canvas.height = pxHeight;
    // Derive CSS size from rounded physical pixels for exact alignment
    canvas.style.width = `${pxWidth / dpr}px`;
    canvas.style.height = `${pxHeight / dpr}px`;
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

    // Handle active resize drag
    if (resizingCol >= 0) {
      const delta = resizeStartX - e.clientX;  // moving left = wider
      const visibleCols = columns.filter(c => c.visible);
      const col = visibleCols[resizingCol];
      if (col) {
        const newWidth = Math.max(50, resizeStartWidth + delta);
        columns = columns.map(c => c.id === col.id ? { ...c, width: newWidth } : c);
      }
      return;
    }

    // Track hovered row from mouse Y position
    const activeVp = filteredViewport ?? $viewport;
    const currentOffset = filteredViewport ? filteredOffset : (activeVp?.offset ?? 0);
    const newHoveredRow = Math.floor(y / ROW_HEIGHT) + currentOffset;
    const prevHovered = hoveredRow;
    hoveredRow = (activeVp && activeVp.nodes.length > 0) ? newHoveredRow : null;

    // Check for resize cursor
    const canvasW = canvas.width / canvasDpr;
    const resizeTarget = getResizeTarget(x, columns, canvasW);
    if (resizeTarget >= 0) {
      canvas.style.cursor = "col-resize";
      hoveredGroup = null;
      if (hoveredRow !== prevHovered) scheduleDraw();
      return;
    }

    if (!activeVp || activeVp.nodes.length === 0) {
      hoveredGroup = null;
      canvas.style.cursor = "default";
      return;
    }

    const activeNodes = displayNodes;
    const activeSegments = activeVp.lane_segments ?? [];

    const hit = graphHitTest(x, y, currentOffset, activeNodes, activeVp.visible_lane_count ?? activeVp.total_lane_count, activeSegments);

    if (hit.type === "segment" && hit.groupId !== undefined) {
      hoveredGroup = hit.groupId;
    } else {
      hoveredGroup = null;
    }
    canvas.style.cursor = "default";

    if (hoveredRow !== prevHovered) scheduleDraw();
  }

  function handleMouseLeave() {
    if (hoveredRow !== null) {
      hoveredRow = null;
      scheduleDraw();
    }
    hoveredGroup = null;
  }

  function handleMouseDown(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const canvasW = canvas.width / canvasDpr;

    const resizeTarget = getResizeTarget(x, columns, canvasW);
    if (resizeTarget >= 0) {
      e.preventDefault();
      resizingCol = resizeTarget;
      resizeStartX = e.clientX;
      const visibleCols = columns.filter(c => c.visible);
      resizeStartWidth = visibleCols[resizeTarget].width;

      const onMouseUp = () => {
        resizingCol = -1;
        persistColumns();
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", onMouseUp);
      };

      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", onMouseUp);
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

    const sha = shortOid(node.oid);

    contextMenuItems = [
      {
        label: m.graph_copy_sha({ sha }),
        action: () => navigator.clipboard.writeText(node.oid),
      },
      {
        label: m.graph_copy_message(),
        action: () => navigator.clipboard.writeText(node.summary),
      },
      {
        label: m.patch_create_commit(),
        action: async () => {
          try {
            const dir = await save({
              title: m.patch_save_dialog_title(),
              defaultPath: `${sha}.patch`,
              filters: [{ name: "Patch", extensions: ["patch", "diff"] }],
            });
            if (!dir) return;
            // save() returns the full file path; format-patch needs a directory
            const sep = dir.includes("/") ? "/" : "\\";
            const parentDir = dir.substring(0, dir.lastIndexOf(sep)) || ".";
            await runMutation({
              kind: "patch_create",
              invoke: () => createCommitPatches([node.oid], parentDir),
              successToast: () => `Saved patch for ${sha}`,
              failureToastPrefix: "Patch create failed",
            });
          } catch {
            // runMutation already surfaced the toast.
          }
        },
      },
      { label: "", action: () => {}, separator: true },
      {
        label: m.graph_create_branch({ sha }),
        action: () => {
          openCreateBranchDialog(buildCreateBranchSource(node.oid));
        },
      },
      {
        label: m.graph_cherry_pick({ sha }),
        action: async () => {
          try {
            await runMutation({
              kind: "cherry_pick",
              invoke: () => cherryPick(node.oid),
              successToast: () => `Cherry-picked ${sha}`,
              failureToastPrefix: "Cherry-pick failed",
            });
          } catch {
            // runMutation already surfaced the toast.
          }
        },
      },
      { label: "", action: () => {}, separator: true },
      {
        label: m.graph_checkout({ sha }),
        action: async () => {
          // For commits with refs, checkout the branch name
          if (node.refs.length > 0) {
            const branchRef = node.refs.find(r => !r.startsWith("refs/remotes/") && !r.startsWith("refs/tags/"));
            if (branchRef) {
              const branchName = branchRef.replace("refs/heads/", "");
              try {
                await runMutation({
                  kind: "checkout",
                  invoke: () => checkoutBranch(branchName),
                  successToast: () => `Checked out ${branchName}`,
                  failureToastPrefix: "Checkout failed",
                });
              } catch {
                // runMutation already surfaced the toast.
              }
              return;
            }
          }
          alert(m.graph_checkout_detached());
        },
      },
      { label: "", action: () => {}, separator: true },
      {
        label: m.graph_revert_commit(),
        action: () => {
          confirmProps = {
            title: m.graph_revert_commit(),
            detail: node.oid.slice(0, 8),
            message: m.graph_revert_confirm({ sha: node.oid.slice(0, 8) }),
            confirmLabel: m.graph_revert_commit(),
            destructive: false,
            onConfirm: async () => {
              try {
                await runMutation({
                  kind: "revert",
                  invoke: () => revertCommit(node.oid),
                  successToast: () => `Reverted ${node.oid.slice(0, 8)}`,
                  failureToastPrefix: "Revert failed",
                });
              } catch {
                // runMutation surfaced the toast; conflicts also caught by watcher.
              }
              showConfirm = false;
            },
          };
          showConfirm = true;
        },
      },
      { label: "", action: () => {}, separator: true },
      {
        label: m.graph_reset_soft(),
        action: () => {
          confirmProps = {
            title: m.graph_reset_to(),
            detail: node.oid.slice(0, 8) + ' — ' + (node.summary || ''),
            message: m.graph_reset_confirm_soft({ sha: node.oid.slice(0, 8) }),
            confirmLabel: m.graph_reset_soft(),
            destructive: false,
            onConfirm: async () => {
              try {
                await runMutation({
                  kind: "reset_soft",
                  invoke: () => resetToCommit(node.oid, 'soft'),
                  successToast: () => `Reset (soft) to ${node.oid.slice(0, 8)}`,
                  failureToastPrefix: "Reset failed",
                });
              } catch {
                // runMutation surfaced the toast.
              }
              showConfirm = false;
            },
          };
          showConfirm = true;
        },
      },
      {
        label: m.graph_reset_mixed(),
        action: () => {
          confirmProps = {
            title: m.graph_reset_to(),
            detail: node.oid.slice(0, 8) + ' — ' + (node.summary || ''),
            message: m.graph_reset_confirm_mixed({ sha: node.oid.slice(0, 8) }),
            confirmLabel: m.graph_reset_mixed(),
            destructive: false,
            onConfirm: async () => {
              try {
                await runMutation({
                  kind: "reset_mixed",
                  invoke: () => resetToCommit(node.oid, 'mixed'),
                  successToast: () => `Reset (mixed) to ${node.oid.slice(0, 8)}`,
                  failureToastPrefix: "Reset failed",
                });
              } catch {
                // runMutation surfaced the toast.
              }
              showConfirm = false;
            },
          };
          showConfirm = true;
        },
      },
      {
        label: m.graph_reset_hard(),
        action: () => {
          confirmProps = {
            title: m.graph_reset_to(),
            detail: node.oid.slice(0, 8) + ' — ' + (node.summary || ''),
            message: m.graph_reset_confirm_hard({ sha: node.oid.slice(0, 8) }),
            confirmLabel: m.graph_reset_hard(),
            destructive: true,
            onConfirm: async () => {
              try {
                await runMutation({
                  kind: "reset_hard",
                  invoke: () => resetToCommit(node.oid, 'hard'),
                  successToast: () => `Reset (hard) to ${node.oid.slice(0, 8)}`,
                  failureToastPrefix: "Reset failed",
                });
              } catch {
                // runMutation surfaced the toast.
              }
              showConfirm = false;
            },
          };
          showConfirm = true;
        },
      },
      { label: "", action: () => {}, separator: true },
      {
        label: m.graph_rebase_onto(),
        action: () => {
          confirmProps = {
            title: m.graph_rebase_onto(),
            detail: node.oid.slice(0, 8),
            message: m.graph_rebase_confirm({ sha: node.oid.slice(0, 8) }),
            confirmLabel: m.graph_rebase_onto(),
            destructive: false,
            onConfirm: async () => {
              try {
                await runMutation({
                  kind: "rebase",
                  invoke: () => rebaseBranch(node.oid),
                  successToast: () => `Rebased onto ${node.oid.slice(0, 8)}`,
                  failureToastPrefix: "Rebase failed",
                  trackAsTask: true,
                });
              } catch {
                // runMutation surfaced the toast.
              }
              showConfirm = false;
            },
          };
          showConfirm = true;
        },
      },
      {
        label: m.graph_interactive_rebase(),
        action: () => {
          rebaseBaseOid = node.oid;
          showRebaseEditor = true;
        },
      },
    ];

    // ── Bisect actions (when active) ──
    if ($bisectState.active) {
      const isGood = bisectGoodSet.has(node.oid);
      const isBad = bisectBadSet.has(node.oid);

      contextMenuItems.push(
        { label: "", action: () => {}, separator: true },
        {
          label: m.graph_bisect_mark_good(),
          action: async () => {
            try { await markGood(node.oid); } catch {}
          },
          disabled: isGood,
        },
        {
          label: m.graph_bisect_mark_bad(),
          action: async () => {
            try { await markBad(node.oid); } catch {}
          },
          disabled: isBad,
        },
        {
          label: m.graph_bisect_skip(),
          action: async () => {
            try { await skipCommit(); } catch {}
          },
        },
      );
    }

    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
    contextMenuVisible = true;
  }

  function handleWindowClick(e: MouseEvent) {
    if (showColumnMenu && columnToggleEl && !columnToggleEl.contains(e.target as Node)) {
      showColumnMenu = false;
    }
  }

  /** Dev-only perf overlay toggle: Ctrl+Shift+P. */
  function handleKeyDown(e: KeyboardEvent) {
    if (import.meta.env.DEV && e.ctrlKey && e.shiftKey && (e.key === 'P' || e.key === 'p')) {
      e.preventDefault();
      showPerfOverlay = !showPerfOverlay;
      scheduleDraw();
    }
  }

  onMount(() => {
    ctx = canvas.getContext("2d");
    resizeCanvas();

    const debouncedResize = debounce(resizeCanvas, 100);
    const resizeObserver = new ResizeObserver(() => debouncedResize());
    resizeObserver.observe(container);

    // Re-render canvas when DPI changes (e.g. moving window between screens)
    let dprQuery: MediaQueryList;
    function watchDpr() {
      dprQuery = window.matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
      dprQuery.addEventListener("change", onDprChange, { once: true });
    }
    function onDprChange() {
      resizeCanvas();
      watchDpr(); // re-register for the new DPR value
    }
    watchDpr();

    window.addEventListener("click", handleWindowClick);
    window.addEventListener("keydown", handleKeyDown);

    // Load persisted column config, merging with defaults for new columns
    getGraphColumns().then(saved => {
      if (saved.length > 0) {
        const merged = DEFAULT_COLUMNS.map(def => {
          const s = saved.find(c => c.id === def.id);
          return s ? { ...def, width: s.width, visible: s.visible } : { ...def };
        });
        columns = merged;
      }
    }).catch(() => {
      // Use defaults on error
    });

    if ($repoInfo) {
      loadViewport(0);
    }

    return () => {
      if (drawRafId !== null) cancelAnimationFrame(drawRafId);
      resizeObserver.disconnect();
      dprQuery.removeEventListener("change", onDprChange);
      window.removeEventListener("click", handleWindowClick);
      window.removeEventListener("keydown", handleKeyDown);
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
    $bisectState;
    scheduleDraw();
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

<div class="git-graph" data-testid="graph-container">
  <div class="graph-header">
    <SearchBar
      filters={graphFilters}
      bind:tags={graphSearchTags}
      placeholder={m.graph_search_placeholder()}
      onSearch={handleGraphSearch}
      testId="graph-search"
    />
    <div class="column-toggle" bind:this={columnToggleEl}>
      <Button variant="neutral" size="sm" active={showColumnMenu} onclick={() => showColumnMenu = !showColumnMenu}>{m.graph_columns()}</Button>
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
      data-testid="graph-canvas"
      onwheel={handleWheel}
      onclick={handleClick}
      onmousemove={handleMouseMove}
      onmouseleave={handleMouseLeave}
      onmousedown={handleMouseDown}
      oncontextmenu={handleContextMenu}
    ></canvas>
    <!--
      Skeleton paint — overlaid on the canvas when the viewport store is
      null (cold start, no cached slice). Shows three faint vertical
      lane stripes instead of a spinner or "Loading…" text so the graph
      feels stable and ready to receive data. Fades out as soon as
      `viewport` is set (tab cache, persisted slice, or fresh fetch).
    -->
    {#if $viewport === null}
      <div class="graph-skeleton" data-testid="graph-skeleton" aria-hidden="true">
        <span class="graph-skeleton__lane" style="left: 14px"></span>
        <span class="graph-skeleton__lane" style="left: 32px"></span>
        <span class="graph-skeleton__lane" style="left: 50px"></span>
      </div>
    {/if}
    <!--
      Hidden DOM mirror of the currently-rendered graph nodes.
      The graph itself paints to a <canvas> which is opaque to querySelectorAll,
      so E2E specs rely on this list to assert row counts (e.g. "commit added
      a new row") and to drive accessibility tech that cannot read canvas.
      Only the OID is exposed; layout lives on the canvas.
    -->
    <ol class="graph-rows-mirror" data-testid="graph-rows" aria-hidden="true">
      {#each filteredNodes as node (node.oid)}
        <li data-testid="graph-row" data-oid={node.oid}></li>
      {/each}
    </ol>
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

{#if showConfirm}
  <ConfirmDialog
    title={confirmProps.title}
    detail={confirmProps.detail}
    message={confirmProps.message}
    confirmLabel={confirmProps.confirmLabel}
    destructive={confirmProps.destructive}
    onConfirm={confirmProps.onConfirm}
    onCancel={() => showConfirm = false}
  />
{/if}

{#if showRebaseEditor}
  <RebaseEditor
    baseOid={rebaseBaseOid}
    onComplete={() => showRebaseEditor = false}
    onCancel={() => showRebaseEditor = false}
  />
{/if}

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
    border-bottom: 1px solid var(--border);
  }

  .graph-header :global(.search-bar) {
    flex: 1;
    min-width: 0;
    border-bottom: none;
  }

  .column-toggle {
    position: relative;
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
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3); /* beardgit:allow-hex: shadow neutral always-dark */
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
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
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

  /*
    Cold-start skeleton — three faint vertical lane stripes that
    approximate the default graph layout (lane width ~18px, first
    lane ~14px from left). Uses `--text-primary` tinted by opacity
    instead of a bespoke colour so it follows the active theme.
  */
  .graph-skeleton {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .graph-skeleton__lane {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--text-primary);
    opacity: 0.06;
    border-radius: 1px;
  }

  /*
   * Visually hidden — the mirror exists purely for E2E assertions and
   * accessibility tooling. Kept rendered (not display:none) so the
   * browser still reflects `.length` on querySelectorAll; clip
   * strategy cribbed from the standard a11y-only pattern.
   */
  .graph-rows-mirror {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: 0;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
    pointer-events: none;
    list-style: none;
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
