<!--
  CleanDialog.svelte — Full git clean dialog with filter toggles and file selection.

  Opens as a modal overlay. Runs git clean dry-run on mount and whenever
  filter toggles change. User selects files via checkboxes, then confirms
  deletion with a destructive action button.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { cleanDryRun, cleanPaths } from "$lib/api/tauri";
  import { refreshStatuses, refreshDiffs } from "$lib/stores/changes";
  import type { CleanItem } from "$lib/types";

  let {
    onClose,
  }: {
    onClose: () => void;
  } = $props();

  let items = $state<CleanItem[]>([]);
  let selected = $state<Set<string>>(new Set());
  let loading = $state(true);
  let errorMessage = $state<string | null>(null);

  // Filter toggles
  let includeDirs = $state(false);
  let includeIgnored = $state(false);
  let onlyIgnored = $state(false);

  let allSelected = $derived(items.length > 0 && selected.size === items.length);

  async function loadItems() {
    loading = true;
    try {
      items = await cleanDryRun(includeDirs, includeIgnored, onlyIgnored);
      // Pre-select all items
      selected = new Set(items.map(i => i.path));
    } catch {
      items = [];
      selected = new Set();
    }
    loading = false;
  }

  onMount(() => {
    loadItems();
  });

  function handleToggleDirs() {
    includeDirs = !includeDirs;
    loadItems();
  }

  function handleToggleIncludeIgnored() {
    includeIgnored = !includeIgnored;
    if (includeIgnored) onlyIgnored = false;
    loadItems();
  }

  function handleToggleOnlyIgnored() {
    onlyIgnored = !onlyIgnored;
    if (onlyIgnored) includeIgnored = false;
    loadItems();
  }

  function toggleItem(path: string) {
    const next = new Set(selected);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    selected = next;
  }

  function toggleAll() {
    if (allSelected) {
      selected = new Set();
    } else {
      selected = new Set(items.map(i => i.path));
    }
  }

  async function handleDelete() {
    if (selected.size === 0) return;
    errorMessage = null;
    try {
      await cleanPaths([...selected]);
      await Promise.all([refreshStatuses(), refreshDiffs()]);
      onClose();
    } catch (err) {
      errorMessage = String(err);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="backdrop" onclick={onClose} onkeydown={(e) => { if (e.key === "Escape") onClose(); }} role="button" tabindex="-1"></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={m.clean_dialog_title()}>
  <h3 class="dialog-title">{m.clean_dialog_title()}</h3>

  <div class="filter-row">
    <label class="toggle">
      <input type="checkbox" checked={includeDirs} onchange={handleToggleDirs} />
      <span>{m.clean_dialog_include_dirs()}</span>
    </label>
    <label class="toggle">
      <input type="checkbox" checked={includeIgnored} onchange={handleToggleIncludeIgnored} />
      <span>{m.clean_dialog_include_ignored()}</span>
    </label>
    <label class="toggle">
      <input type="checkbox" checked={onlyIgnored} onchange={handleToggleOnlyIgnored} />
      <span>{m.clean_dialog_only_ignored()}</span>
    </label>
  </div>

  <div class="file-list-container">
    {#if loading}
      <div class="empty-state">{m.clean_dialog_loading()}</div>
    {:else if items.length === 0}
      <div class="empty-state">{m.clean_dialog_no_files()}</div>
    {:else}
      <div class="select-row">
        <label class="toggle">
          <input type="checkbox" checked={allSelected} onchange={toggleAll} />
          <span>{allSelected ? m.clean_dialog_deselect_all() : m.clean_dialog_select_all()}</span>
        </label>
      </div>
      <div class="file-list">
        {#each items as item}
          <label class="file-item">
            <input
              type="checkbox"
              checked={selected.has(item.path)}
              onchange={() => toggleItem(item.path)}
            />
            <span class="file-icon">{item.is_directory ? "\uF4D3" : "\uF4A5"}</span>
            <span class="file-path">{item.path}{item.is_directory ? "/" : ""}</span>
            {#if item.is_ignored}
              <span class="ignored-badge">ignored</span>
            {/if}
          </label>
        {/each}
      </div>
    {/if}
  </div>

  {#if errorMessage}
    <div class="dialog-error">{errorMessage}</div>
  {/if}

  <div class="dialog-warning">{m.clean_dialog_warning()}</div>

  <div class="dialog-actions">
    <button class="btn btn-cancel" onclick={onClose}>{m.confirm_cancel()}</button>
    <button
      class="btn btn-destructive"
      disabled={selected.size === 0}
      onclick={handleDelete}
    >
      {m.clean_dialog_delete_selected({ count: String(selected.size) })}
    </button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions, .btn, .btn-cancel, .btn-destructive */

  .dialog {
    min-width: 420px;
    max-width: 560px;
    max-height: 70vh;
    display: flex;
    flex-direction: column;
  }

  .dialog-title {
    margin-bottom: 12px;
  }

  .filter-row {
    display: flex;
    gap: 16px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 8px;
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .toggle input[type="checkbox"] {
    margin: 0;
    accent-color: var(--accent-blue);
  }

  .file-list-container {
    flex: 1;
    overflow-y: auto;
    min-height: 100px;
    max-height: 300px;
  }

  .select-row {
    padding: 4px 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }

  .file-list {
    display: flex;
    flex-direction: column;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 4px;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
    border-radius: 4px;
  }

  .file-item:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .file-icon {
    font-family: var(--font-icons);
    font-size: 13px;
    color: var(--text-secondary);
    width: 16px;
    text-align: center;
  }

  .file-path {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ignored-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .dialog-error {
    padding: 6px 10px;
    margin-bottom: 4px;
    font-size: 12px;
    color: var(--accent-red, #f85149);
    background: var(--overlay-accent-red);
    border-radius: 4px;
    word-break: break-word;
  }

  .dialog-warning {
    padding: 8px 0;
    font-size: 12px;
    color: var(--accent-red, #f85149);
    line-height: 1.4;
  }

  .dialog-actions {
    padding-top: 8px;
  }
</style>
