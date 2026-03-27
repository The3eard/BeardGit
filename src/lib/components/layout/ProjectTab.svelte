<script lang="ts">
  import type { ProjectInfo } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

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
</script>

<div
  class="project-tab"
  class:active={isActive}
  onclick={handleClick}
  onauxclick={handleMiddleClick}
  onkeydown={(e) => { if (e.key === "Enter") handleClick(); }}
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
    {"\uEA76"}
  </button>
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
    display: none;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    font-family: var(--font-icons);
    cursor: pointer;
    padding: 0 2px;
    line-height: 1;
    flex-shrink: 0;
    margin-left: auto;
    border-radius: 50%;
  }

  .tab-close:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.1);
  }

  .project-tab:hover .tab-close {
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
