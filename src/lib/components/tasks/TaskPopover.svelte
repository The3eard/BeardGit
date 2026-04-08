<script lang="ts">
  import { sortedTasks, selectedOutput, expandPanel, closePanel, selectTask, panelMode } from "../../stores/tasks";
  import { ansiToHtml } from "../../utils/ansi";
  import TaskList from "./TaskList.svelte";
  import * as m from "$lib/paraglide/messages";
  import type { TaskId } from "../../types";

  let popoverEl: HTMLDivElement | undefined = $state();

  function handleClickOutside(e: MouseEvent) {
    if (popoverEl && !popoverEl.contains(e.target as Node)) {
      closePanel();
    }
  }

  function handleTaskClick(taskId: TaskId) {
    selectTask(taskId);
    expandPanel();
  }

  let previewLines = $derived(
    $selectedOutput.slice(-5).map((l) => l.text).join("\n")
  );

  let previewHtml = $derived(previewLines ? ansiToHtml(previewLines) : null);
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
      <button class="icon-btn" onclick={expandPanel} title={m.tasks_expand_tooltip()}>{"\uEA67"}</button>
      <button class="icon-btn close-btn" onclick={closePanel}>{"\uEA76"}</button>
    </div>
  </div>

  <TaskList tasks={$sortedTasks} onTaskClick={handleTaskClick} />

  {#if previewHtml}
    <div class="output-preview">
      <div class="preview-content">{@html previewHtml}</div>
    </div>
  {/if}
</div>

<style>
  .task-popover {
    position: absolute;
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
    gap: 6px;
  }

  .icon-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--accent-blue);
    font-size: 13px;
    font-family: var(--font-icons);
    padding: 0 4px;
    border-radius: 3px;
    cursor: pointer;
    transition: background 0.1s;
    line-height: 1;
  }

  .icon-btn:hover {
    background: var(--overlay-hover);
  }

  .close-btn {
    color: var(--text-secondary);
    border: none;
  }

  .output-preview {
    border-top: 1px solid var(--border);
    padding: 6px 12px;
    flex-shrink: 0;
  }

  .preview-content {
    background: var(--bg-primary);
    border-radius: 4px;
    padding: 6px 8px;
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: 10px;
    color: var(--text-secondary);
    max-height: 64px;
    overflow: hidden;
    white-space: pre-wrap;
    line-height: 1.4;
  }
</style>
