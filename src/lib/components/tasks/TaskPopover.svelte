<script lang="ts">
  import { onMount } from "svelte";
  import { sortedTasks, expandPanel, closePanel, selectTask } from "../../stores/taskPanel";
  import TaskList from "./TaskList.svelte";
  import * as m from "$lib/paraglide/messages";
  import type { TaskId } from "../../types";

  let popoverEl: HTMLDivElement | undefined = $state();
  let ready = $state(false);

  // Delay click-outside detection by one frame so the opening click
  // (on the StatusBar indicator) doesn't immediately close the popover.
  onMount(() => {
    requestAnimationFrame(() => { ready = true; });
  });

  function handleClickOutside(e: MouseEvent) {
    if (!ready) return;
    if (popoverEl && !popoverEl.contains(e.target as Node)) {
      closePanel();
    }
  }

  function handleTaskClick(taskId: TaskId) {
    selectTask(taskId);
  }
</script>

<svelte:window onclick={handleClickOutside} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="task-popover" bind:this={popoverEl} onclick={(e) => e.stopPropagation()}>
  <div class="popover-header">
    <div class="header-left">
      <span class="header-title">{m.tasks_title()}</span>
      <span class="header-badge">{$sortedTasks.length}</span>
    </div>
    <div class="header-actions">
      <button class="btn-icon" onclick={expandPanel} title={m.tasks_expand_tooltip()}>
        <span class="nf">{"\uF065"}</span>
      </button>
      <button class="btn-icon" onclick={closePanel} title="Close">
        <span class="nf">{"\uF00D"}</span>
      </button>
    </div>
  </div>

  <TaskList tasks={$sortedTasks} onTaskClick={handleTaskClick} />
</div>

<style>
  .task-popover {
    position: fixed;
    bottom: 28px;
    left: 8px;
    width: min(360px, 90vw);
    max-height: min(400px, 50vh);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 -4px 24px var(--overlay-shadow);
    z-index: 100;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .popover-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .header-title {
    font-weight: 600;
    font-size: 12px;
    color: var(--text-primary);
  }

  .header-badge {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    padding: 0 6px;
    border-radius: 8px;
    font-size: 10px;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  /* Glyphs are in <span class="nf">, not on the button itself */
  .btn-icon {
    font-family: inherit;
    padding: 2px 4px;
    display: flex;
    align-items: center;
  }
</style>
