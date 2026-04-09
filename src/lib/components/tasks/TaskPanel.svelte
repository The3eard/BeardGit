<script lang="ts">
  import { sortedTasks, selectedOutput, selectedTask, collapsePanel, closePanel } from "../../stores/tasks";
  import { ansiToHtml } from "../../utils/ansi";
  import TaskList from "./TaskList.svelte";
  import * as m from "$lib/paraglide/messages";

  let logContainer: HTMLDivElement | undefined = $state();

  let outputHtml = $derived(
    $selectedOutput.length > 0
      ? ansiToHtml($selectedOutput.map((l) => l.text).join("\n"))
      : null
  );

  let taskCommand = $derived($selectedTask?.command ?? null);

  $effect(() => {
    if (outputHtml && logContainer) {
      requestAnimationFrame(() => {
        if (logContainer) {
          logContainer.scrollTop = logContainer.scrollHeight;
        }
      });
    }
  });
</script>

<div class="task-panel">
  <div class="panel-sidebar">
    <div class="panel-header">
      <span class="header-title">{m.tasks_title()}</span>
      <div class="header-actions">
        <button class="panel-icon-btn" onclick={collapsePanel} title={m.tasks_collapse_tooltip()}>
          <span class="nf">{"\uF066"}</span>
        </button>
        <button class="panel-icon-btn" onclick={closePanel} title="Close">
          <span class="nf">{"\uF00D"}</span>
        </button>
      </div>
    </div>
    <div class="panel-list">
      <TaskList tasks={$sortedTasks} />
    </div>
  </div>

  <div class="panel-output">
    <div class="output-header">
      <span class="output-label">{m.tasks_output()}</span>
    </div>
    {#if taskCommand}
      <div class="output-command">
        <span class="command-prompt">$</span> {taskCommand}
      </div>
    {/if}
    {#if outputHtml}
      <div class="output-content" bind:this={logContainer}>{@html outputHtml}</div>
    {:else if $selectedTask}
      <div class="output-empty">{m.tasks_no_output()}</div>
    {:else}
      <div class="output-empty">{m.tasks_select_task()}</div>
    {/if}
  </div>
</div>

<style>
  .task-panel {
    display: flex;
    height: 100%;
    overflow: hidden;
    background: var(--bg-secondary);
  }

  .panel-sidebar {
    width: clamp(160px, 15vw, 220px);
    min-width: 0;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .header-title {
    font-weight: 600;
    font-size: 11px;
    color: var(--text-primary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .panel-icon-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
  }

  .panel-icon-btn:hover {
    color: var(--text-primary);
  }

  .panel-list {
    flex: 1;
    overflow-y: auto;
  }

  .panel-output {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .output-header {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .output-label {
    color: var(--text-secondary);
    font-size: 11px;
  }

  .output-command {
    padding: 6px 8px;
    background: var(--bg-primary);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-blue);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: all;
  }

  .command-prompt {
    color: var(--text-secondary);
    margin-right: 4px;
  }

  .output-content {
    flex: 1;
    padding: 8px;
    background: var(--bg-primary);
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.5;
    color: var(--text-primary);
    white-space: pre-wrap;
    overflow-y: auto;
    -webkit-user-select: text;
    user-select: text;
  }

  .output-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 12px;
    font-style: italic;
  }
</style>
