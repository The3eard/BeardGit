<script lang="ts">
  import type { ProjectInfo, TerminalTabInfo } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    project: ProjectInfo;
    terminal: TerminalTabInfo;
    activeSegment: "project" | "terminal";
    isActiveTab: boolean;
    onSwitchSegment: (segment: "project" | "terminal") => void;
    onCloseProject: () => void;
    onCloseTerminal: () => void;
  }

  let {
    project,
    terminal,
    activeSegment,
    isActiveTab,
    onSwitchSegment,
    onCloseProject,
    onCloseTerminal,
  }: Props = $props();

  let statusColor = $derived(
    isActiveTab && activeSegment === "project"
      ? "var(--accent-blue)"
      : project.change_count > 0
        ? "var(--accent-orange)"
        : "var(--accent-green)"
  );

  function handleProjectClick() {
    onSwitchSegment("project");
  }

  function handleTerminalClick() {
    onSwitchSegment("terminal");
  }

  function handleCloseProject(e: MouseEvent) {
    e.stopPropagation();
    onCloseProject();
  }

  function handleCloseTerminal(e: MouseEvent) {
    e.stopPropagation();
    onCloseTerminal();
  }

  function handleProjectMiddleClick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
      onCloseProject();
    }
  }

  function handleTerminalMiddleClick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
      onCloseTerminal();
    }
  }

  /** Extract the short label from terminal title (e.g. "Terminal" from "Terminal · BeardGit") */
  let terminalLabel = $derived(
    terminal.title.includes(" · ") ? terminal.title.split(" · ")[0] : terminal.title
  );
</script>

<div class="composite-tab" role="tab" tabindex="0">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="segment"
    class:active={isActiveTab && activeSegment === "project"}
    class:dimmed={isActiveTab && activeSegment !== "project"}
    onclick={handleProjectClick}
    onauxclick={handleProjectMiddleClick}
    onkeydown={(e) => { if (e.key === "Enter") handleProjectClick(); }}
  >
    <span class="status-dot" style="background: {statusColor}"></span>
    <span class="segment-name">{project.name}</span>
    {#if project.change_count > 0}
      <span class="tab-badge">{project.change_count}</span>
    {/if}
    <button class="tab-close" onclick={handleCloseProject} title={m.tab_close()}>
      {"\uF00D"}
    </button>
  </div>

  <div class="divider"></div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="segment"
    class:active={isActiveTab && activeSegment === "terminal"}
    class:dimmed={isActiveTab && activeSegment !== "terminal"}
    onclick={handleTerminalClick}
    onauxclick={handleTerminalMiddleClick}
    onkeydown={(e) => { if (e.key === "Enter") handleTerminalClick(); }}
  >
    <span class="terminal-icon">{"\uF489"}</span>
    <span class="segment-name">{terminalLabel}</span>
    <button class="tab-close" onclick={handleCloseTerminal} title={m.tab_close()}>
      {"\uF00D"}
    </button>
  </div>
</div>

<style>
  .composite-tab {
    display: flex;
    align-items: center;
    height: 28px;
    border-radius: 14px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.04);
    flex-shrink: 0;
    user-select: none;
  }

  .segment {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 10px;
    height: 100%;
    cursor: pointer;
    transition: background 0.15s, opacity 0.15s;
  }

  .segment.active {
    background: rgba(255, 255, 255, 0.12);
  }

  .segment.dimmed {
    opacity: 0.5;
  }

  .segment:not(.active):not(.dimmed) {
    opacity: 0.7;
  }

  .segment:hover {
    opacity: 1;
  }

  .divider {
    width: 1px;
    height: 60%;
    background: var(--border);
    flex-shrink: 0;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .terminal-icon {
    font-family: var(--font-icons);
    font-size: 12px;
    color: var(--accent-purple);
    flex-shrink: 0;
  }

  .segment-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
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
    font-size: 10px;
    font-family: var(--font-icons);
    cursor: pointer;
    padding: 0;
    line-height: 1;
    flex-shrink: 0;
  }

  .tab-close:hover {
    color: var(--text-primary);
  }

  .composite-tab:hover .tab-close {
    display: inline;
  }
</style>
