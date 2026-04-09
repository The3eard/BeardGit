<script lang="ts">
  import { onMount } from "svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import CreateWorktreeDialog from "./CreateWorktreeDialog.svelte";
  import { worktrees, worktreeLoading, refreshWorktrees, deleteWorktree } from "../../stores/worktrees";
  import { openProjectTab } from "../../stores/projects";
  import { repoInfo } from "../../stores/repo";
  import * as m from "$lib/paraglide/messages";

  let showCreateDialog = $state(false);
  let confirmRemovePath = $state<string | null>(null);
  let forceRemove = $state(false);

  onMount(() => {
    refreshWorktrees();
  });

  function handleOpenInTab(path: string) {
    openProjectTab(path);
  }

  function handleRemove(path: string) {
    forceRemove = false;
    confirmRemovePath = path;
  }

  function confirmRemove() {
    if (confirmRemovePath) {
      deleteWorktree(confirmRemovePath, forceRemove);
      confirmRemovePath = null;
    }
  }

  /** Extract last path segment for compact display. */
  function shortPath(fullPath: string): string {
    const parts = fullPath.replace(/\\/g, "/").split("/");
    return parts.slice(-2).join("/");
  }
</script>

<div class="worktree-list">
  <!-- Header -->
  <div class="list-header">
    <span class="list-title">{m.sidebar_worktrees().toUpperCase()}</span>
    <div class="header-actions">
      <button
        class="action-btn nf"
        onclick={() => (showCreateDialog = true)}
        title={m.worktree_create()}
      >
        {"\uF067"}
      </button>
      <button
        class="action-btn nf"
        onclick={() => refreshWorktrees()}
        disabled={$worktreeLoading}
        title="Refresh"
      >
        {$worktreeLoading ? "\uF110" : "\uF021"}
      </button>
    </div>
  </div>

  <!-- List -->
  <div class="list-items">
    {#if $worktreeLoading && $worktrees.length === 0}
      <div class="list-loading">
        <div class="spinner"></div>
      </div>
    {:else if $worktrees.length === 0}
      <div class="list-empty">{m.worktree_list_empty()}</div>
    {:else}
      {#each $worktrees as wt (wt.path)}
        <div class="worktree-item">
          <div class="wt-info">
            <div class="wt-branch-row">
              <span class="wt-branch" class:main={wt.is_main}>
                {wt.branch ?? "detached"}
              </span>
              {#if wt.is_main}
                <span class="wt-badge main">{m.worktree_current()}</span>
              {/if}
              {#if wt.is_locked}
                <span class="wt-badge locked">{m.worktree_locked()}</span>
              {/if}
            </div>
            <div class="wt-path" title={wt.path}>{shortPath(wt.path)}</div>
          </div>
          <div class="wt-actions">
            {#if !wt.is_main}
              <button
                class="wt-action-btn"
                onclick={() => handleOpenInTab(wt.path)}
                title={m.worktree_open_tab()}
              >
                <span class="nf">{"\uF08E"}</span>
              </button>
              <button
                class="wt-action-btn destructive"
                onclick={() => handleRemove(wt.path)}
                title={m.worktree_remove()}
              >
                <span class="nf">{"\uF1F8"}</span>
              </button>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

{#if showCreateDialog && $repoInfo}
  <CreateWorktreeDialog
    repoPath={$repoInfo.path}
    onClose={() => (showCreateDialog = false)}
  />
{/if}

{#if confirmRemovePath !== null}
  <ConfirmDialog
    title={m.worktree_remove()}
    detail={confirmRemovePath}
    message={m.worktree_remove_confirm({ path: shortPath(confirmRemovePath) })}
    confirmLabel={forceRemove ? m.worktree_remove_force() : m.worktree_remove()}
    destructive={true}
    onConfirm={confirmRemove}
    onCancel={() => (confirmRemovePath = null)}
  />
{/if}

<style>
  .worktree-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 8px;
    flex-shrink: 0;
  }

  .list-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .header-actions {
    display: flex;
    gap: 4px;
  }

  .action-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    padding: 2px 4px;
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-icons);
    display: flex;
    align-items: center;
  }

  .action-btn:hover {
    color: var(--text-primary);
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .list-items {
    flex: 1;
    overflow-y: auto;
  }

  .list-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }

  .list-empty {
    padding: 12px 16px;
    font-size: 11px;
    color: var(--text-secondary);
    font-style: italic;
  }

  .worktree-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
  }

  .worktree-item:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .wt-info {
    flex: 1;
    min-width: 0;
  }

  .wt-branch-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .wt-branch {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent-blue);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .wt-branch.main {
    color: var(--accent-green);
  }

  .wt-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    padding: 1px 5px;
    border-radius: 8px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .wt-badge.main {
    background: rgba(63, 185, 80, 0.15);
    color: var(--accent-green);
  }

  .wt-badge.locked {
    background: rgba(210, 153, 34, 0.15);
    color: var(--accent-orange);
  }

  .wt-path {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .wt-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .worktree-item:hover .wt-actions {
    opacity: 1;
  }

  .wt-action-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    padding: 2px 4px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .wt-action-btn:hover {
    color: var(--text-primary);
  }

  .wt-action-btn.destructive:hover {
    color: var(--accent-red);
  }
</style>
