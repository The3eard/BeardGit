<script lang="ts">
  import type { TerminalTabInfo } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    terminal: TerminalTabInfo;
    isActive: boolean;
    onSwitch: () => void;
    onClose: () => void;
  }

  let { terminal, isActive, onSwitch, onClose }: Props = $props();

  function handleClick() {
    if (!isActive) onSwitch();
  }

  function handleClose(e: MouseEvent) {
    e.stopPropagation();
    onClose();
  }

  function handleMiddleClick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
      onClose();
    }
  }
</script>

<div
  class="terminal-tab"
  class:active={isActive}
  onclick={handleClick}
  onauxclick={handleMiddleClick}
  onkeydown={(e) => { if (e.key === "Enter") handleClick(); }}
  role="tab"
  tabindex="0"
>
  <span class="status-dot"></span>
  <span class="terminal-icon">{"\uF489"}</span>
  <span class="tab-name">{terminal.title}</span>
  <button
    class="tab-close"
    onclick={handleClose}
    title={m.tab_close()}
  >
    {"\uF00D"}
  </button>
</div>

<style>
  .terminal-tab {
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

  .terminal-tab:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .terminal-tab.active {
    background: rgba(255, 255, 255, 0.12);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--accent-purple);
  }

  .terminal-icon {
    font-family: var(--font-icons);
    font-size: 13px;
    color: var(--accent-purple);
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

  .tab-close {
    display: none;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 8px;
    font-family: var(--font-icons);
    cursor: pointer;
    padding: 0;
    line-height: 1;
    flex-shrink: 0;
  }

  .tab-close:hover {
    color: var(--text-primary);
  }

  .terminal-tab:hover .tab-close {
    display: inline;
  }
</style>
