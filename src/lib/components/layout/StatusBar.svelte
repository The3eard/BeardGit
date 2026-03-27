<script lang="ts">
  import { repoInfo, branches } from "../../stores/repo";
  import { activeProvider } from "../../stores/provider";
  import { runningTasks, hasRunningTasks, tasks, togglePopover } from "../../stores/tasks";
  import { isInConflict, conflictStateLabel } from "../../stores/conflict";
  import * as m from "$lib/paraglide/messages";

  let branchCount = $derived($branches.length);

  let recentFailure = $state(false);
  let failureTimer: ReturnType<typeof setTimeout> | null = null;

  let failedCount = $derived(
    $tasks.filter((t) => t.status.state === "failed").length
  );

  $effect(() => {
    if (failedCount > 0) {
      recentFailure = true;
      if (failureTimer) clearTimeout(failureTimer);
      failureTimer = setTimeout(() => {
        if (!$hasRunningTasks) {
          recentFailure = false;
        }
      }, 5000);
    }
  });
</script>

<footer class="status-bar">
  <div class="status-left">
    {#if $repoInfo}
      <span class="status-indicator repo-open"></span>
      <span>{branchCount === 1 ? m.statusbar_branch({ count: String(branchCount) }) : m.statusbar_branches({ count: String(branchCount) })}</span>
    {:else}
      <span class="status-indicator disconnected"></span>
      <span>{m.statusbar_no_repo()}</span>
    {/if}

    {#if $isInConflict}
      <span class="status-separator">|</span>
      <span class="conflict-badge">{$conflictStateLabel}</span>
    {/if}

    {#if $hasRunningTasks}
      <span class="status-separator">|</span>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span class="task-indicator task-running" onclick={togglePopover}>
        <span class="task-spinner"></span>
        {m.tasks_running({ count: String($runningTasks.length) })}
      </span>
    {:else if recentFailure}
      <span class="status-separator">|</span>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span class="task-indicator task-failed" onclick={togglePopover}>
        <span class="nf">{"\uF00D"}</span> {m.tasks_failed({ count: String(failedCount) })}
      </span>
    {/if}

    {#if $activeProvider}
      <span class="status-separator">|</span>
      <span class="status-indicator provider-connected"></span>
      <span>{$activeProvider.kind === 'github' ? m.statusbar_github_connected() : m.statusbar_gitlab_connected()}</span>
      {#if $activeProvider.project_name}
        <span class="status-separator">|</span>
        <span>{$activeProvider.project_name}</span>
      {/if}
    {/if}
  </div>
  <div class="status-right">
    {#if $repoInfo}
      <span class="status-path" title={$repoInfo.path}>{$repoInfo.path}</span>
    {/if}
  </div>
</footer>

<style>
  .status-bar {
    height: 24px;
    min-height: 24px;
    background: var(--bg-toolbar);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    font-size: 11px;
    color: var(--text-secondary);
    user-select: none;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-indicator {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }

  .status-indicator.repo-open {
    background: var(--accent-blue);
  }

  .status-indicator.provider-connected {
    background: var(--accent-green);
  }

  .status-indicator.disconnected {
    background: var(--text-secondary);
  }

  .status-separator {
    opacity: 0.4;
  }

  .conflict-badge {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.5px;
    padding: 0 6px;
    border-radius: 3px;
    background: rgba(210, 153, 34, 0.2);
    color: var(--accent-orange);
    line-height: 16px;
  }

  .status-path {
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: right;
  }

  .task-indicator {
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    transition: opacity 0.15s;
  }

  .task-indicator:hover {
    opacity: 0.8;
  }

  .task-running {
    color: var(--accent-orange);
  }

  .task-failed {
    color: var(--accent-red);
  }

  .task-spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
