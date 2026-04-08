<script lang="ts">
  import {
    submodules,
    submodulesLoading,
    refreshSubmodules,
    initSubmodule,
    updateSubmodule,
    updateAllSubmodules,
    deinitSubmodule,
    getSubmoduleAbsPath,
  } from "../../stores/submodules";
  import { openProjectTab } from "../../stores/projects";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import * as m from "$lib/paraglide/messages";
  import type { SubmoduleInfo } from "../../types";

  // Refresh on mount
  $effect(() => {
    refreshSubmodules();
  });

  // Context menu state
  let contextMenuItems = $state<MenuItem[]>([]);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let contextMenuVisible = $state(false);

  // Confirm dialog state
  let confirmProps = $state<{
    title: string;
    message: string;
    onConfirm: () => void;
  } | null>(null);

  function statusLabel(status: string): string {
    switch (status) {
      case "uninitialized":
        return m.submodule_status_uninitialized();
      case "clean":
        return m.submodule_status_clean();
      case "outdated":
        return m.submodule_status_outdated();
      case "dirty":
        return m.submodule_status_dirty();
      default:
        return status;
    }
  }

  function statusColor(status: string): string {
    switch (status) {
      case "uninitialized":
        return "var(--text-secondary)";
      case "clean":
        return "var(--accent-green)";
      case "outdated":
        return "var(--accent-orange)";
      case "dirty":
        return "var(--accent-red)";
      default:
        return "var(--text-secondary)";
    }
  }

  async function handleOpenInTab(sub: SubmoduleInfo) {
    try {
      const absPath = await getSubmoduleAbsPath(sub.path);
      await openProjectTab(absPath);
    } catch (err) {
      alert(String(err));
    }
  }

  function handleContextMenu(e: MouseEvent, sub: SubmoduleInfo) {
    e.preventDefault();
    const items: MenuItem[] = [];

    if (sub.status !== "uninitialized") {
      items.push({
        label: m.submodule_open_tab(),
        action: () => handleOpenInTab(sub),
      });
      items.push({ separator: true });
    }

    if (sub.status === "uninitialized") {
      items.push({
        label: m.submodule_init(),
        action: async () => {
          try {
            await initSubmodule(sub.path);
          } catch (err) {
            alert(m.submodule_init_failed({ error: String(err) }));
          }
        },
      });
    }

    if (sub.status !== "uninitialized") {
      items.push({
        label: m.submodule_update(),
        action: () => updateSubmodule(sub.path),
      });
    }

    if (sub.status !== "uninitialized") {
      items.push({ separator: true });
      items.push({
        label: m.submodule_deinit(),
        action: () => {
          confirmProps = {
            title: m.submodule_deinit(),
            message: m.submodule_deinit_confirm({ name: sub.name }),
            onConfirm: async () => {
              try {
                await deinitSubmodule(sub.path, false);
              } catch (err) {
                alert(m.submodule_deinit_failed({ error: String(err) }));
              }
              confirmProps = null;
            },
          };
        },
      });
      items.push({
        label: m.submodule_deinit_force(),
        action: () => {
          confirmProps = {
            title: m.submodule_deinit_force(),
            message: m.submodule_deinit_force_confirm({ name: sub.name }),
            onConfirm: async () => {
              try {
                await deinitSubmodule(sub.path, true);
              } catch (err) {
                alert(m.submodule_deinit_failed({ error: String(err) }));
              }
              confirmProps = null;
            },
          };
        },
      });
    }

    items.push({ separator: true });
    items.push({
      label: m.submodule_copy_path(),
      action: () => navigator.clipboard.writeText(sub.path),
    });
    items.push({
      label: m.submodule_copy_url(),
      action: () => navigator.clipboard.writeText(sub.url),
    });

    contextMenuItems = items;
    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
    contextMenuVisible = true;
  }

  async function handleUpdateAll() {
    await updateAllSubmodules();
  }
</script>

<div class="submodule-view">
  <div class="header">
    <h2 class="title">{m.submodule_title()}</h2>
    {#if $submodules.length > 0}
      <button class="action-btn" onclick={handleUpdateAll}>
        {m.submodule_update_all()}
      </button>
    {/if}
  </div>

  {#if $submodulesLoading}
    <div class="empty-state">{m.submodule_title()}...</div>
  {:else if $submodules.length === 0}
    <div class="empty-state">{m.submodule_empty()}</div>
  {:else}
    <div class="submodule-list">
      {#each $submodules as sub}
        <button
          class="submodule-row"
          oncontextmenu={(e) => handleContextMenu(e, sub)}
          ondblclick={() => {
            if (sub.status !== "uninitialized") handleOpenInTab(sub);
          }}
        >
          <div class="sub-info">
            <span class="sub-path">{sub.path}</span>
            <span class="sub-url">{sub.url}</span>
          </div>
          <div class="sub-meta">
            {#if sub.oid}
              <span class="sub-sha">{sub.oid.substring(0, 7)}</span>
            {/if}
            <span class="status-badge" style="color: {statusColor(sub.status)}">
              {statusLabel(sub.status)}
            </span>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>

<ContextMenu
  items={contextMenuItems}
  x={contextMenuX}
  y={contextMenuY}
  visible={contextMenuVisible}
  onClose={() => { contextMenuVisible = false; }}
/>

{#if confirmProps}
  <ConfirmDialog
    title={confirmProps.title}
    message={confirmProps.message}
    onConfirm={confirmProps.onConfirm}
    onCancel={() => { confirmProps = null; }}
  />
{/if}

<style>
  .submodule-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .action-btn {
    padding: 4px 10px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 11px;
    cursor: pointer;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .empty-state {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .submodule-list {
    flex: 1;
    overflow-y: auto;
  }

  .submodule-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 16px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
  }

  .submodule-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .sub-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .sub-path {
    font-size: 13px;
    font-weight: 500;
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub-url {
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    margin-left: 12px;
  }

  .sub-sha {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .status-badge {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.5px;
  }
</style>
