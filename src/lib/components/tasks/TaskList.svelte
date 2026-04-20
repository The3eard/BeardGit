<script lang="ts">
  import type { TaskInfo, TaskId } from "../../types";
  import { cancelTask, selectTask, selectedTaskId } from "../../stores/taskPanel";
  import { formatRelativeTimeMs } from "../../utils/time";
  import * as m from "$lib/paraglide/messages";

  let { tasks, onTaskClick }: { tasks: TaskInfo[]; onTaskClick?: (id: TaskId) => void } = $props();

  function statusColor(task: TaskInfo): string {
    switch (task.status.state) {
      case "running": return "var(--accent-orange)";
      case "completed": return "var(--accent-green)";
      case "failed": return "var(--accent-red)";
      case "cancelled": return "var(--text-secondary)";
      default: return "var(--text-secondary)";
    }
  }

  function formatDuration(secs: number | null): string {
    if (secs === null) return "";
    if (secs < 60) return `${secs.toFixed(1)}s`;
    const mins = Math.floor(secs / 60);
    const remainSecs = (secs % 60).toFixed(0);
    return `${mins}m ${remainSecs}s`;
  }

  function formatTimeAgo(ms: number | null): string {
    if (ms === null) return "";
    return formatRelativeTimeMs(ms);
  }

  function handleCancel(e: MouseEvent, taskId: TaskId) {
    e.stopPropagation();
    cancelTask(taskId);
  }

  function handleClick(task: TaskInfo) {
    if (onTaskClick) {
      onTaskClick(task.id);
    } else {
      selectTask(task.id);
    }
  }
</script>

<div class="task-list">
  {#each tasks as task (task.id)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="task-card"
      class:selected={$selectedTaskId === task.id}
      onclick={() => handleClick(task)}
    >
      <div class="card-status-bar" style="background: {statusColor(task)}"></div>
      <div class="card-content">
        <div class="card-line-1">
          <span class="card-label">{task.label}</span>
          {#if task.cancellable && task.status.state === "running"}
            <button class="cancel-btn" onclick={(e) => handleCancel(e, task.id)}>
              {m.tasks_cancel()}
            </button>
          {/if}
        </div>
        <div class="card-line-2">
          <span class="card-command" title={task.command}>{task.command}</span>
          <span class="card-meta">
            {#if task.elapsed_secs !== null}
              <span>{formatDuration(task.elapsed_secs)}</span>
            {/if}
            {#if task.started_at_ms !== null}
              <span class="card-separator">&middot;</span>
              <span>{formatTimeAgo(task.started_at_ms)}</span>
            {/if}
          </span>
        </div>
      </div>
    </div>
  {:else}
    <div class="empty">{m.tasks_no_tasks()}</div>
  {/each}
</div>

<style>
  .task-list {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    flex: 1;
  }

  .task-card {
    display: flex;
    cursor: pointer;
    transition: background 0.1s;
  }

  .task-card:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .task-card.selected {
    background: rgba(255, 255, 255, 0.08);
  }

  .card-status-bar {
    width: 3px;
    flex-shrink: 0;
    border-radius: 2px;
    margin: 4px 0 4px 6px;
  }

  .card-content {
    flex: 1;
    min-width: 0;
    padding: 6px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .card-line-1 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .card-label {
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-line-2 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .card-command {
    color: var(--text-secondary);
    font-size: 10px;
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: 10px;
    flex-shrink: 0;
  }

  .card-separator {
    opacity: 0.5;
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
</style>
