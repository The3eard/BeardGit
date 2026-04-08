<!--
  GitConfigSettings.svelte — Git config viewer/editor for the Settings page.

  Displays a merged two-column table of local and global config entries
  with inline editing, known-key dropdowns, and add/unset support.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { listConfig, setConfig, unsetConfig, addConfig } from "$lib/api/tauri";
  import { isEnumKey, getEnumValues } from "$lib/utils/git-config-keys";
  import type { ConfigEntry, ConfigScope } from "$lib/types";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";

  let localEntries = $state<ConfigEntry[]>([]);
  let globalEntries = $state<ConfigEntry[]>([]);
  let systemEntries = $state<ConfigEntry[]>([]);
  let loading = $state(true);
  let filterText = $state("");

  // Inline editing state
  let editingKey = $state<string | null>(null);
  let editingScope = $state<ConfigScope | null>(null);
  let editingValue = $state("");

  // Unset confirmation
  let unsetTarget = $state<{ key: string; scope: ConfigScope } | null>(null);

  // Add entry state
  let showAddRow = $state(false);
  let addKey = $state("");
  let addValue = $state("");
  let addScope = $state<ConfigScope>("local");

  // Show/hide system config
  let showSystem = $state(false);
  let errorMessage = $state<string | null>(null);

  /** Merged key list from local + global, sorted alphabetically. */
  let mergedKeys = $derived.by(() => {
    const keys = new Set<string>();
    for (const e of localEntries) keys.add(e.key);
    for (const e of globalEntries) keys.add(e.key);
    const sorted = [...keys].sort();
    if (!filterText.trim()) return sorted;
    const lower = filterText.toLowerCase();
    return sorted.filter(k => k.toLowerCase().includes(lower));
  });

  function localValue(key: string): string | null {
    return localEntries.find(e => e.key === key)?.value ?? null;
  }

  function globalValue(key: string): string | null {
    return globalEntries.find(e => e.key === key)?.value ?? null;
  }

  async function loadAll() {
    loading = true;
    try {
      const [local, global, system] = await Promise.all([
        listConfig("local"),
        listConfig("global"),
        listConfig("system"),
      ]);
      localEntries = local;
      globalEntries = global;
      systemEntries = system;
    } catch {
      // Silently handle — some scopes may not exist
    }
    loading = false;
  }

  onMount(() => {
    loadAll();
  });

  function startEdit(key: string, scope: ConfigScope, currentValue: string) {
    editingKey = key;
    editingScope = scope;
    editingValue = currentValue;
  }

  function startAdd(key: string, scope: ConfigScope) {
    editingKey = key;
    editingScope = scope;
    editingValue = "";
  }

  async function saveEdit() {
    if (!editingKey || !editingScope) return;
    errorMessage = null;
    try {
      await setConfig(editingScope, editingKey, editingValue);
      editingKey = null;
      editingScope = null;
      await loadAll();
    } catch (err) {
      errorMessage = String(err);
    }
  }

  function cancelEdit() {
    editingKey = null;
    editingScope = null;
  }

  function handleEditKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") saveEdit();
    else if (e.key === "Escape") cancelEdit();
  }

  async function confirmUnset() {
    if (!unsetTarget) return;
    errorMessage = null;
    try {
      await unsetConfig(unsetTarget.scope, unsetTarget.key);
      unsetTarget = null;
      await loadAll();
    } catch (err) {
      errorMessage = String(err);
    }
  }

  async function handleAddEntry() {
    if (!addKey.trim()) return;
    errorMessage = null;
    try {
      await addConfig(addScope, addKey.trim(), addValue);
      addKey = "";
      addValue = "";
      showAddRow = false;
      await loadAll();
    } catch (err) {
      errorMessage = String(err);
    }
  }

  function handleAddKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") handleAddEntry();
    else if (e.key === "Escape") {
      showAddRow = false;
      addKey = "";
      addValue = "";
    }
  }
</script>

<div class="config-editor">
  <div class="config-toolbar">
    <input
      class="filter-input"
      type="text"
      placeholder={m.config_filter_placeholder()}
      bind:value={filterText}
    />
    <button class="add-btn" onclick={() => showAddRow = !showAddRow}>
      {m.config_add_entry()}
    </button>
  </div>

  {#if errorMessage}
    <div class="config-error">{errorMessage}</div>
  {/if}

  {#if showAddRow}
    <div class="add-row">
      <input
        class="add-input key-input"
        type="text"
        placeholder={m.config_add_key_placeholder()}
        bind:value={addKey}
        onkeydown={handleAddKeydown}
      />
      <input
        class="add-input value-input"
        type="text"
        placeholder={m.config_add_value_placeholder()}
        bind:value={addValue}
        onkeydown={handleAddKeydown}
      />
      <select class="scope-select" bind:value={addScope}>
        <option value="local">{m.config_add_scope_local()}</option>
        <option value="global">{m.config_add_scope_global()}</option>
      </select>
      <button class="save-btn" onclick={handleAddEntry} disabled={!addKey.trim()}>+</button>
    </div>
  {/if}

  {#if loading}
    <div class="empty-state">{m.config_loading()}</div>
  {:else}
    <div class="config-table">
      <div class="table-header">
        <span class="col-key">{m.config_column_key()}</span>
        <span class="col-local">{m.config_column_local()}</span>
        <span class="col-global">
          {m.config_column_global()}
          <span class="scope-badge">{m.config_global_badge()}</span>
        </span>
      </div>

      {#if mergedKeys.length === 0}
        <div class="empty-state">{m.config_empty()}</div>
      {/if}

      <div class="table-body">
        {#each mergedKeys as key}
          {@const lv = localValue(key)}
          {@const gv = globalValue(key)}
          {@const enumVals = getEnumValues(key)}
          <div class="table-row">
            <span class="col-key cell-key" title={key}>{key}</span>

            <!-- Local column -->
            <span class="col-local cell-value">
              {#if editingKey === key && editingScope === "local"}
                {#if enumVals}
                  <!-- svelte-ignore a11y_autofocus -->
                  <select
                    class="edit-select"
                    bind:value={editingValue}
                    onchange={saveEdit}
                    onblur={cancelEdit}
                    autofocus
                  >
                    <option value="">{m.config_no_value()}</option>
                    {#each enumVals as opt}
                      <option value={opt}>{opt}</option>
                    {/each}
                  </select>
                {:else}
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    class="edit-input"
                    type="text"
                    bind:value={editingValue}
                    onkeydown={handleEditKeydown}
                    onblur={saveEdit}
                    autofocus
                  />
                {/if}
              {:else if lv !== null}
                <button class="value-btn" onclick={() => startEdit(key, "local", lv)} title="Click to edit">
                  {lv}
                </button>
                <button class="unset-btn" onclick={() => unsetTarget = { key, scope: "local" }} title="Remove">
                  &#10005;
                </button>
              {:else}
                <button class="placeholder-btn" onclick={() => startAdd(key, "local")}>
                  {m.config_no_value()}
                </button>
              {/if}
            </span>

            <!-- Global column -->
            <span class="col-global cell-value">
              {#if editingKey === key && editingScope === "global"}
                {#if enumVals}
                  <!-- svelte-ignore a11y_autofocus -->
                  <select
                    class="edit-select"
                    bind:value={editingValue}
                    onchange={saveEdit}
                    onblur={cancelEdit}
                    autofocus
                  >
                    <option value="">{m.config_no_value()}</option>
                    {#each enumVals as opt}
                      <option value={opt}>{opt}</option>
                    {/each}
                  </select>
                {:else}
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    class="edit-input"
                    type="text"
                    bind:value={editingValue}
                    onkeydown={handleEditKeydown}
                    onblur={saveEdit}
                    autofocus
                  />
                {/if}
              {:else if gv !== null}
                <button class="value-btn" onclick={() => startEdit(key, "global", gv)} title="Click to edit">
                  {gv}
                </button>
                <button class="unset-btn" onclick={() => unsetTarget = { key, scope: "global" }} title="Remove">
                  &#10005;
                </button>
              {:else}
                <button class="placeholder-btn" onclick={() => startAdd(key, "global")}>
                  {m.config_no_value()}
                </button>
              {/if}
            </span>
          </div>
        {/each}
      </div>
    </div>

    <!-- System config (read-only, collapsible) -->
    {#if systemEntries.length > 0}
      <button class="system-toggle" onclick={() => showSystem = !showSystem}>
        {showSystem ? "\u25BC" : "\u25B6"} {m.config_system_section()}
      </button>
      {#if showSystem}
        <div class="system-table">
          {#each systemEntries as entry}
            <div class="system-row">
              <span class="col-key cell-key">{entry.key}</span>
              <span class="cell-value system-value">{entry.value}</span>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}
</div>

{#if unsetTarget}
  <ConfirmDialog
    title={m.config_unset_confirm_title()}
    message={m.config_unset_confirm_message({ key: unsetTarget.key, scope: unsetTarget.scope })}
    destructive={true}
    onConfirm={confirmUnset}
    onCancel={() => unsetTarget = null}
  />
{/if}

<style>
  .config-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .config-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .filter-input {
    flex: 1;
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
  }

  .filter-input:focus {
    border-color: var(--accent-blue);
  }

  .add-btn {
    padding: 6px 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--accent-blue);
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }

  .add-btn:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .config-error {
    padding: 6px 16px;
    font-size: 12px;
    color: var(--accent-red, #f85149);
    background: var(--overlay-accent-red);
    border-bottom: 1px solid var(--border);
    word-break: break-word;
  }

  .add-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    background: rgba(255, 255, 255, 0.02);
  }

  .add-input {
    padding: 4px 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
  }

  .key-input { width: 200px; font-family: var(--font-mono); }
  .value-input { flex: 1; }

  .scope-select {
    padding: 4px 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
  }

  .save-btn {
    padding: 4px 10px;
    background: var(--accent-blue);
    color: #fff;
    border: none;
    border-radius: 4px;
    font-size: 14px;
    cursor: pointer;
  }

  .save-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .config-table {
    flex: 1;
    overflow-y: auto;
  }

  .table-header {
    display: flex;
    align-items: center;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    position: sticky;
    top: 0;
    background: var(--bg-secondary);
    z-index: 1;
  }

  .table-body {
    display: flex;
    flex-direction: column;
  }

  .table-row {
    display: flex;
    align-items: center;
    padding: 4px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  .table-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .col-key {
    width: 40%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-local { width: 30%; min-width: 0; }
  .col-global { width: 30%; min-width: 0; }

  .cell-key {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
  }

  .cell-value {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
  }

  .value-btn {
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: calc(100% - 24px);
    text-align: left;
  }

  .value-btn:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .placeholder-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    padding: 2px 4px;
    opacity: 0.5;
  }

  .placeholder-btn:hover {
    opacity: 1;
    color: var(--accent-blue);
  }

  .unset-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 10px;
    cursor: pointer;
    padding: 2px 4px;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .table-row:hover .unset-btn {
    opacity: 0.6;
  }

  .unset-btn:hover {
    opacity: 1 !important;
    color: var(--accent-red, #f85149);
  }

  .edit-input, .edit-select {
    width: 100%;
    padding: 2px 4px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--accent-blue);
    border-radius: 3px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
    outline: none;
  }

  .scope-badge {
    font-size: 9px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    padding: 1px 5px;
    border-radius: 3px;
    margin-left: 6px;
    text-transform: none;
    letter-spacing: 0;
  }

  .system-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 10px 16px;
    background: none;
    border: none;
    border-top: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }

  .system-toggle:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .system-table {
    padding: 0 16px;
  }

  .system-row {
    display: flex;
    align-items: center;
    padding: 3px 0;
    font-size: 12px;
  }

  .system-value {
    font-family: var(--font-mono);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 120px;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
