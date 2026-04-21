<script lang="ts">
  import type { ProjectInfo, ProjectSnapshot } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import TabTooltip from "./TabTooltip.svelte";
  import { getSnapshotForHover } from "$lib/stores/project-cache";
  import { openRepoConfigDialog } from "$lib/stores/repoConfig";

  interface Props {
    project: ProjectInfo;
    isActive: boolean;
    index: number;
    onSwitch: (index: number) => void;
    onClose: (index: number) => void;
  }

  let { project, isActive, index, onSwitch, onClose }: Props = $props();

  /**
   * Open the per-repo "Repo settings" dialog for this tab's project.
   * Only visible on the active tab — non-active tabs route the cog to
   * the repo-tab context menu, keeping visual chrome tight.
   *
   * Stops propagation so clicking the cog never also fires the tab's
   * primary onclick (which would re-activate an already-active tab).
   */
  function handleSettings(event: MouseEvent) {
    event.stopPropagation();
    openRepoConfigDialog();
  }

  /**
   * Right-click on the tab opens a lightweight context menu hosting
   * "Repo settings" — the keyboard/screen-reader-accessible secondary
   * entry point to the same dialog. Rendered as a native `<menu>`
   * popup positioned at the click coordinates.
   */
  let contextMenuVisible = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);

  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
    contextMenuX = event.clientX;
    contextMenuY = event.clientY;
    contextMenuVisible = true;
  }

  function closeContextMenu() {
    contextMenuVisible = false;
  }

  function handleContextSettings() {
    closeContextMenu();
    // Switch to this tab first so the dialog loads the right repo.
    if (!isActive) onSwitch(index);
    openRepoConfigDialog();
  }

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
  oncontextmenu={handleContextMenu}
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
  {#if isActive}
    <button
      class="tab-settings"
      onclick={handleSettings}
      title="Repo settings"
      aria-label="Repo settings"
      data-testid="project-tab-settings"
    >
      {"\uF013"}
    </button>
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

{#if contextMenuVisible}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div
    class="context-menu-backdrop"
    role="presentation"
    onclick={closeContextMenu}
    oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
  ></div>
  <menu
    class="context-menu"
    style="left: {contextMenuX}px; top: {contextMenuY}px"
    data-testid="project-tab-context-menu"
  >
    <li>
      <button
        type="button"
        class="context-menu-item"
        onclick={handleContextSettings}
        data-testid="project-tab-context-settings"
      >
        <span class="nf">{"\uF013"}</span>
        <span>Repo settings</span>
      </button>
    </li>
  </menu>
{/if}

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
    background: var(--overlay-accent-green);
    color: var(--accent-green);
    font-weight: 600;
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

  .tab-settings {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 11px;
    font-family: var(--font-icons);
    cursor: pointer;
    padding: 0;
    line-height: 1;
    flex-shrink: 0;
    opacity: 0.6;
  }

  .tab-settings:hover {
    color: var(--text-primary);
    opacity: 1;
  }

  .context-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 998;
  }

  .context-menu {
    position: fixed;
    z-index: 999;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 6px 20px var(--overlay-shadow);
    min-width: 180px;
  }

  .context-menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    border-radius: 4px;
  }

  .context-menu-item:hover {
    background: var(--overlay-hover);
  }
</style>
