<script lang="ts">
  import { onMount } from "svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import CreateWorktreeDialog from "./CreateWorktreeDialog.svelte";
  import List from "../common/List.svelte";
  import {
    worktrees,
    worktreeLoading,
    refreshWorktrees,
    deleteWorktree,
    cleanupAiWorktree,
  } from "../../stores/worktrees";
  import { openProjectTab } from "../../stores/projects";
  import { repoInfo } from "../../stores/repo";
  import { lockWorktree, unlockWorktree } from "../../api/tauri";
  import { navigateToCommit } from "../../stores/graph";
  import type { EnrichedWorktree } from "$lib/types";
  import type { AiProviderKind } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    onNavigateToGraph?: (oid: string) => void;
  }
  let { onNavigateToGraph }: Props = $props();

  /** Display name per AI provider. */
  const PROVIDER_NAME: Record<AiProviderKind, string> = {
    claude_code: "Claude",
    codex: "Codex",
    open_code: "OpenCode",
  };

  /** Badge color per AI provider. These use CSS token references for theme compatibility. */
  const PROVIDER_COLOR: Record<AiProviderKind, string> = {
    claude_code: "var(--accent-orange)",
    codex: "var(--accent-green)",
    open_code: "var(--accent-purple)",
  };

  let showCreateDialog = $state(false);
  let confirmRemovePath = $state<string | null>(null);
  let forceRemove = $state(false);

  // Context menu state
  let menuVisible = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<MenuItem[]>([]);

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

  async function handleLock(wt: EnrichedWorktree) {
    try {
      await lockWorktree(wt.path);
      await refreshWorktrees();
    } catch (err) {
      // Lock failure is non-fatal — toast or alert
      console.error("Failed to lock worktree:", err);
    }
  }

  async function handleUnlock(wt: EnrichedWorktree) {
    try {
      await unlockWorktree(wt.path);
      await refreshWorktrees();
    } catch (err) {
      console.error("Failed to unlock worktree:", err);
    }
  }

  async function handleOpenInGraph(wt: EnrichedWorktree) {
    if (!wt.head_oid) return;
    const oid = wt.head_oid;
    if (onNavigateToGraph) {
      onNavigateToGraph(oid);
    } else {
      await navigateToCommit(oid);
    }
  }

  /** Extract last path segment for compact display. */
  function shortPath(fullPath: string): string {
    const parts = fullPath.replace(/\\/g, "/").split("/");
    return parts.slice(-2).join("/");
  }

  /** Build context menu items for a worktree. */
  function handleContextMenu(e: MouseEvent, wt: EnrichedWorktree) {
    e.preventDefault();

    const items: MenuItem[] = [
      { label: m.worktree_open_graph(), action: () => handleOpenInGraph(wt) },
      { label: m.worktree_open_tab(), action: () => handleOpenInTab(wt.path) },
      { separator: true },
      {
        label: wt.is_locked ? m.worktree_unlock() : m.worktree_lock(),
        action: () => wt.is_locked ? handleUnlock(wt) : handleLock(wt),
      },
      {
        label: m.worktree_remove(),
        action: () => handleRemove(wt.path),
        disabled: wt.is_main,
      },
    ];

    // AI-specific items
    if (wt.ai_provider) {
      items.push({ separator: true });
      if (wt.ai_status === "active") {
        items.push({
          label: m.worktree_focus_terminal(),
          action: () => {},
        });
      }
      items.push({
        label: m.worktree_cleanup(),
        action: () => cleanupAiWorktree(wt.ai_provider!, wt.path),
      });
    }

    menuItems = items;
    menuX = e.clientX;
    menuY = e.clientY;
    menuVisible = true;
  }

  function getKey(wt: EnrichedWorktree): string {
    return wt.path;
  }
</script>

<List
  items={$worktrees}
  loading={$worktreeLoading}
  title={m.sidebar_worktrees().toUpperCase()}
  selectedKey={null}
  {getKey}
  emptyMessage={m.worktree_list_empty()}
  onRefresh={refreshWorktrees}
  onContextMenu={handleContextMenu}
>
  {#snippet headerActions()}
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
  {/snippet}

  {#snippet row({ item })}
    <div
      class="worktree-row"
      class:ai-active={item.ai_status === "active"}
      style={item.ai_provider ? `--ai-color: ${PROVIDER_COLOR[item.ai_provider]}` : ""}
    >
      <div class="wt-info">
        <div class="wt-branch-row">
          <span class="wt-branch" class:main={item.is_main}>
            {item.branch ?? "detached"}
          </span>
          {#if item.is_main}
            <span class="wt-badge main">{m.worktree_current()}</span>
          {/if}
          {#if item.is_locked}
            <span class="wt-badge locked">{m.worktree_locked()}</span>
          {/if}
          {#if item.ai_provider}
            <span
              class="wt-badge ai"
              style="--badge-color: {PROVIDER_COLOR[item.ai_provider]}"
            >
              {PROVIDER_NAME[item.ai_provider]}
            </span>
          {/if}
          {#if item.ai_status === "active"}
            <span class="wt-badge ai-status active">ACTIVE</span>
          {:else if item.ai_status === "clean"}
            <span class="wt-badge ai-status clean">CLEAN</span>
          {:else if item.ai_status === "orphaned"}
            <span class="wt-badge ai-status orphaned">ORPHANED</span>
          {/if}
        </div>
        <div class="wt-path" title={item.path}>{shortPath(item.path)}</div>
      </div>
      <div class="wt-actions">
        {#if !item.is_main}
          <button
            class="wt-action-btn"
            onclick={(e: MouseEvent) => {
              e.stopPropagation();
              handleOpenInTab(item.path);
            }}
            title={m.worktree_open_tab()}
          >
            <span class="nf">{"\uF08E"}</span>
          </button>
          <button
            class="wt-action-btn destructive"
            onclick={(e: MouseEvent) => {
              e.stopPropagation();
              handleRemove(item.path);
            }}
            title={m.worktree_remove()}
          >
            <span class="nf">{"\uF1F8"}</span>
          </button>
        {/if}
      </div>
    </div>
  {/snippet}
</List>

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

<ContextMenu
  items={menuItems}
  x={menuX}
  y={menuY}
  visible={menuVisible}
  onClose={() => (menuVisible = false)}
/>

<style>
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

  .worktree-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
  }

  /* Extend the ai-active tint to cover the full list row padding */
  .worktree-row.ai-active {
    background: color-mix(in srgb, var(--ai-color) 4%, transparent);
    margin: -8px -12px;
    padding: 8px 12px;
  }

  .wt-info {
    flex: 1;
    min-width: 0;
  }

  .wt-branch-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
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
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }

  .wt-badge.locked {
    background: color-mix(in srgb, var(--accent-orange) 15%, transparent);
    color: var(--accent-orange);
  }

  .wt-badge.ai {
    background: color-mix(in srgb, var(--badge-color) 15%, transparent);
    color: var(--badge-color);
  }

  .wt-badge.ai-status.active {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }

  .wt-badge.ai-status.orphaned {
    background: color-mix(in srgb, var(--accent-red) 15%, transparent);
    color: var(--accent-red);
  }

  .wt-badge.ai-status.clean {
    background: color-mix(in srgb, var(--accent-blue) 15%, transparent);
    color: var(--accent-blue);
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

  :global(.list-row:hover) .wt-actions {
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
