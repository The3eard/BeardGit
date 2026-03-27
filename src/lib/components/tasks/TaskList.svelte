<script lang="ts">
  import type { TaskInfo, TaskId } from "../../types";
  import { cancelTask, selectTask, selectedTaskId } from "../../stores/tasks";
  import * as m from "$lib/paraglide/messages";

  let { tasks }: { tasks: TaskInfo[] } = $props();

  function statusIcon(task: TaskInfo): string {
    switch (task.status.state) {
      case "queued": return "\uF017";   // nf-fa-clock_o
      case "running": return "";
      case "completed": return "\uF00C"; // nf-fa-check
      case "failed": return "\uF00D";    // nf-fa-times
      case "cancelled": return "\uEA76"; // nf-cod-close
    }
  }

  function formatElapsed(secs: number | null): string {
    if (secs === null) return "";
    return m.tasks_elapsed({ secs: secs.toFixed(1) });
  }

  function statusTitle(task: TaskInfo): string {
    switch (task.status.state) {
      case "queued": return m.tasks_status_queued();
      case "running": return m.tasks_status_running();
      case "completed": return m.tasks_status_completed();
      case "failed": return m.tasks_status_failed();
      case "cancelled": return m.tasks_status_cancelled();
    }
  }

  function handleCancel(e: MouseEvent, taskId: TaskId) {
    e.stopPropagation();
    cancelTask(taskId);
  }
</script>

<div class="task-list">
  {#each tasks as task (task.id)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="task-row"
      class:selected={$selectedTaskId === task.id}
      onclick={() => selectTask(task.id)}
    >
      <span class="task-icon" title={statusTitle(task)} class:running={task.status.state === "running"} class:completed={task.status.state === "completed"} class:failed={task.status.state === "failed"} class:cancelled={task.status.state === "cancelled"}>
        {#if task.status.state === "running"}
          <span class="task-spinner-small"></span>
        {:else}
          {statusIcon(task)}
        {/if}
      </span>
      <div class="task-info">
        <span class="task-label">{task.label}</span>
        <span class="task-meta">{formatElapsed(task.elapsed_secs)}</span>
      </div>
      {#if task.cancellable && task.status.state === "running"}
        <button class="cancel-btn" onclick={(e) => handleCancel(e, task.id)}>
          {m.tasks_cancel()}
        </button>
      {/if}
    </div>
  {:else}
    <div class="empty">{m.tasks_no_tasks()}</div>
  {/each}
</div>

<style>
  .task-list {
    display: flex;
    flex-direction: column;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    cursor: pointer;
    transition: background 0.1s;
  }

  .task-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .task-row.selected {
    background: rgba(255, 255, 255, 0.08);
  }

  .task-icon {
    font-size: 12px;
    font-family: var(--font-icons);
    flex-shrink: 0;
    width: 16px;
    text-align: center;
  }

  .task-icon.completed { color: var(--accent-green); }
  .task-icon.failed { color: var(--accent-red); }
  .task-icon.cancelled { color: var(--text-secondary); }
  .task-icon.running { color: var(--accent-orange); }

  .task-info {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .task-label {
    color: var(--text-primary);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .task-row:not(.selected):not(:hover) .task-label {
    color: var(--text-secondary);
  }

  .task-row.selected .task-label,
  .task-row:has(.running) .task-label {
    color: var(--text-primary);
  }

  .task-meta {
    color: var(--text-secondary);
    font-size: 10px;
    flex-shrink: 0;
  }

  .cancel-btn {
    font-size: 10px;
    color: var(--accent-red);
    border: 1px solid var(--accent-red);
    background: none;
    padding: 1px 6px;
    border-radius: 3px;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.1s;
  }

  .cancel-btn:hover {
    background: rgba(248, 81, 73, 0.1);
  }

  .empty {
    padding: 12px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 12px;
    font-style: italic;
  }

  .task-spinner-small {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 1.5px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
