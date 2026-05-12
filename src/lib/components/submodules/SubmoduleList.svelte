<script lang="ts">
  import { onMount } from "svelte";
  import {
    submodules,
    submodulesLoading,
    refreshSubmodules,
    initSubmodule,
    updateSubmodule,
    updateAllSubmodules,
    deinitSubmodule,
    addSubmodule,
    removeSubmodule,
    getSubmoduleAbsPath,
  } from "../../stores/submodules";
  import { openProjectTab } from "../../stores/projects";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import List from "../common/List.svelte";
  import * as m from "$lib/paraglide/messages";
  import { Button } from "$lib/components/ui";
  import type { SubmoduleInfo } from "../../types";
  import { shortOid } from "../../utils/git";

  // Refresh on mount
  onMount(() => {
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

  // Add submodule form state
  let showAddForm = $state(false);
  let addUrl = $state("");
  let addPath = $state("");
  let addError = $state<string | null>(null);
  let adding = $state(false);

  async function handleAdd() {
    if (!addUrl.trim() || !addPath.trim()) return;
    adding = true;
    addError = null;
    try {
      await addSubmodule(addUrl.trim(), addPath.trim());
      showAddForm = false;
      addUrl = "";
      addPath = "";
    } catch (err) {
      addError = String(err);
    } finally {
      adding = false;
    }
  }

  function cancelAdd() {
    showAddForm = false;
    addUrl = "";
    addPath = "";
    addError = null;
  }

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
      label: m.submodule_remove(),
      action: () => {
        confirmProps = {
          title: m.submodule_remove(),
          message: m.submodule_remove_confirm({ name: sub.name }),
          onConfirm: async () => {
            try {
              await removeSubmodule(sub.path);
            } catch (err) {
              alert(m.submodule_remove_failed({ error: String(err) }));
            }
            confirmProps = null;
          },
        };
      },
    });
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

  function getKey(sub: SubmoduleInfo): string {
    return sub.path;
  }

  function handleDoubleClick(sub: SubmoduleInfo) {
    if (sub.status !== "uninitialized") {
      handleOpenInTab(sub);
    }
  }
</script>

<List
  items={$submodules}
  loading={$submodulesLoading}
  title={m.submodule_title()}
  selectedKey={null}
  {getKey}
  emptyMessage={m.submodule_empty()}
  onDoubleClick={handleDoubleClick}
  onContextMenu={handleContextMenu}
>
  {#snippet headerActions()}
    {#if $submodules.length > 0}
      <Button variant="neutral" size="sm" onclick={handleUpdateAll}>
        {m.submodule_update_all()}
      </Button>
    {/if}
    <Button
      variant="primary"
      size="sm"
      onclick={() => {
        showAddForm = !showAddForm;
      }}
    >
      {m.submodule_add()}
    </Button>
  {/snippet}

  {#snippet afterHeader()}
    {#if showAddForm}
      <div class="add-form">
        <input
          type="text"
          class="add-input"
          placeholder={m.submodule_add_url_placeholder()}
          bind:value={addUrl}
        />
        <input
          type="text"
          class="add-input"
          placeholder={m.submodule_add_path_placeholder()}
          bind:value={addPath}
        />
        {#if addError}
          <div class="add-error">{addError}</div>
        {/if}
        <div class="add-actions">
          <Button
            variant="primary"
            size="sm"
            onclick={handleAdd}
            disabled={adding || !addUrl.trim() || !addPath.trim()}
          >
            {adding ? "Adding..." : m.submodule_add()}
          </Button>
          <Button variant="neutral" size="sm" onclick={cancelAdd}>Cancel</Button>
        </div>
      </div>
    {/if}
  {/snippet}

  {#snippet row({ item })}
    <div class="sub-info">
      <span class="sub-path">{item.path}</span>
      <span class="sub-url">{item.url}</span>
    </div>
    <div class="sub-meta">
      {#if item.oid}
        <span class="sub-sha">{shortOid(item.oid)}</span>
      {/if}
      <span class="status-badge" style="color: {statusColor(item.status)}">
        {statusLabel(item.status)}
      </span>
    </div>
  {/snippet}
</List>

<ContextMenu
  items={contextMenuItems}
  x={contextMenuX}
  y={contextMenuY}
  visible={contextMenuVisible}
  onClose={() => {
    contextMenuVisible = false;
  }}
/>

{#if confirmProps}
  <ConfirmDialog
    title={confirmProps.title}
    message={confirmProps.message}
    onConfirm={confirmProps.onConfirm}
    onCancel={() => {
      confirmProps = null;
    }}
  />
{/if}

<style>
  .add-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .add-input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 10px;
    font-size: 12px;
    color: var(--text-primary);
    font-family: var(--font-mono);
  }

  .add-input::placeholder {
    color: var(--text-secondary);
  }

  .add-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .add-actions {
    display: flex;
    gap: 6px;
  }

  .add-error {
    font-size: 11px;
    color: var(--accent-red);
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
