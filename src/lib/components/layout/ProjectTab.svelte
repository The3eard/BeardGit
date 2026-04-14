<script lang="ts">
  import type { ProjectInfo, ProjectSnapshot } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import TabTooltip from "./TabTooltip.svelte";
  import { getSnapshotForHover } from "$lib/stores/project-cache";

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
      ? "var(--accent-blue)"
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
  <span class="tab-name">{project.name}</span>
  {#if project.change_count > 0}
    <span class="tab-badge">{project.change_count}</span>
  {/if}
  <button
    class="tab-close"
    onclick={handleClose}
    title={m.tab_close()}
  >
    {"\uF00D"}
  </button>
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
    background: rgba(255, 255, 255, 0.04);
    transition: background 0.15s;
    flex-shrink: 0;
    user-select: none;
    position: relative;
  }

  .project-tab:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .project-tab.active {
    background: rgba(255, 255, 255, 0.12);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .tab-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-badge {
    font-size: 10px;
    background: var(--accent-orange);
    color: #fff;
    padding: 0 5px;
    border-radius: 8px;
    line-height: 16px;
    flex-shrink: 0;
  }

  .tab-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 8px;
    font-family: var(--font-icons);
    cursor: pointer;
    padding: 0;
    line-height: 1;
    flex-shrink: 0;
    opacity: 0.5;
  }

  .tab-close:hover {
    color: var(--text-primary);
    opacity: 1;
  }
</style>
