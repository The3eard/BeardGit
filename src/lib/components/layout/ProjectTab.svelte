<script lang="ts">
  import type { ProjectInfo, ProjectSnapshot } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import TabTooltip from "./TabTooltip.svelte";
  import TabStatusStrip from "./TabStatusStrip.svelte";
  import {
    getSnapshotForHover,
    refreshProjectSnapshot,
    projectSnapshots,
  } from "$lib/stores/project-cache";
  import IconButton from "$lib/components/ui/IconButton.svelte";

  interface Props {
    project: ProjectInfo;
    isActive: boolean;
    index: number;
    onSwitch: (index: number) => void;
    onClose: (index: number) => void;
  }

  let { project, isActive, index, onSwitch, onClose }: Props = $props();

  let statusColor = $derived(
    isActive
      ? "var(--accent-primary)"
      : project.change_count > 0
        ? "var(--accent-orange)"
        : "var(--accent-green)"
  );

  function handleClick() {
    if (!isActive) {
      onSwitch(index);
    }
  }

  function handleClose(e: MouseEvent) {
    e.stopPropagation();
    onClose(index);
  }

  function handleMiddleClick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
      onClose(index);
    }
  }

  let hoverSnapshot = $state<ProjectSnapshot | null>(null);
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;
  let tooltipX = $state(0);
  let tooltipY = $state(0);

  function handleMouseEnter(e: MouseEvent) {
    const target = e.currentTarget as HTMLElement;
    hoverTimer = setTimeout(async () => {
      const rect = target.getBoundingClientRect();
      tooltipX = rect.left;
      tooltipY = rect.bottom + 4;
      try {
        hoverSnapshot = await getSnapshotForHover(project.path);
      } catch {
        hoverSnapshot = null;
      }
    }, 300);
  }

  function handleMouseLeave() {
    if (hoverTimer) {
      clearTimeout(hoverTimer);
      hoverTimer = null;
    }
    hoverSnapshot = null;
  }

  // Non-active tabs: force a fresh snapshot from a temp repo handle
  // on the Rust side so the strip's data is correct even if the
  // cache file was poisoned by the older live-status fallback.
  // Subsequent updates flow automatically: every watcher-driven
  // saveCurrentSnapshot rewrites `projectSnapshots[path]` and Svelte
  // re-runs the `stripSnapshot` derived below.
  $effect(() => {
    if (!isActive) void refreshProjectSnapshot(project.path);
  });

  let stripSnapshot = $derived($projectSnapshots[project.path] ?? null);
</script>

<div
  class="project-tab"
  class:active={isActive}
  onclick={handleClick}
  onauxclick={handleMiddleClick}
  onkeydown={(e) => { if (e.key === "Enter") handleClick(); }}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  role="tab"
  tabindex="0"
>
  <span class="status-dot" style="background: {statusColor}"></span>
  {#if project.is_worktree}
    <span
      class="worktree-icon nf"
      title={m.project_tab_worktree_title()}
      aria-label={m.project_tab_worktree_title()}
      data-testid="project-tab-worktree-badge"
    >{""}</span>
  {/if}
  <span class="tab-name">{project.name}</span>
  {#if !isActive}
    <TabStatusStrip snapshot={stripSnapshot} />
  {/if}
  <IconButton tone="danger" size="xs" icon={""} description={m.tab_close()} onclick={handleClose} />
  {#if hoverSnapshot}
    <TabTooltip snapshot={hoverSnapshot} x={tooltipX} y={tooltipY} />
  {/if}
</div>

<style>
  .project-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    height: 28px;
    min-width: 0;
    max-width: 220px;
    cursor: pointer;
    border-radius: 14px;
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    transition: background 0.15s;
    flex-shrink: 0;
    user-select: none;
    position: relative;
  }

  .project-tab:hover {
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
  }

  .project-tab.active {
    background: color-mix(in srgb, var(--text-primary) 12%, transparent);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .worktree-icon {
    font-family: var(--font-icons);
    font-size: 10px;
    color: var(--accent-primary);
    line-height: 1;
    flex-shrink: 0;
  }

  .tab-name {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

</style>
