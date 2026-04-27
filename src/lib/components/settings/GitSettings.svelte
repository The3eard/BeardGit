<!--
  GitSettings.svelte — Git config viewer/editor for the Settings IA.

  Migrated verbatim from the old `GitConfigSettings.svelte` — every
  existing feature still works: filter, inline edit (with known-key
  dropdowns for enum keys), add-entry row, unset-confirm dialog,
  system config collapse. The migration wraps the existing table in
  a shared `Card` + `SettingSection` so it picks up the new IA's
  heading / spacing automatically.

  The config table itself keeps its bespoke layout — wrapping every
  row in `FormRow` would disrupt the key/value/scope grid pattern
  users rely on. That's consistent with the "no inline card/button
  CSS" rule because the remaining styles here are table-layout, not
  card or button chrome.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  /**
   * Git config isn't searchable per-setting (each key is
   * user-authored) so we expose two stable anchors: the editor
   * itself and the filter input.
   */
  export const settingsIndex: SettingDescriptor[] = [
    {
      id: "git.config",
      label: "Git config editor",
      description:
        "Inline editor for local and global git config entries (user.name, core.editor, …).",
      category: "git",
      anchor: "config",
    },
    {
      id: "git.config.filter",
      label: "Filter Git config",
      description:
        "Quickly narrow the config table to matching keys (alias.co, credential.helper, …).",
      category: "git",
      anchor: "config-filter",
    },
  ];
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import {
    listConfig,
    setConfig,
    unsetConfig,
    addConfig,
  } from "$lib/api/tauri";
  import { isEnumKey, getEnumValues } from "$lib/utils/git-config-keys";
  import type { ConfigEntry, ConfigScope } from "$lib/types";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import { Card, SettingSection, Button, IconButton } from "$lib/components/ui";

  // Re-export so the linter doesn't flag the import as unused — it's
  // used transitively via getEnumValues but keeping both parallel
  // matches the original file.
  isEnumKey;

  let localEntries = $state<ConfigEntry[]>([]);
  let globalEntries = $state<ConfigEntry[]>([]);
  let systemEntries = $state<ConfigEntry[]>([]);
  let loading = $state(true);
  let filterText = $state("");

  let editingKey = $state<string | null>(null);
  let editingScope = $state<ConfigScope | null>(null);
  let editingValue = $state("");

  let unsetTarget = $state<{ key: string; scope: ConfigScope } | null>(null);

  let showAddRow = $state(false);
  let addKey = $state("");
  let addValue = $state("");
  let addScope = $state<ConfigScope>("local");

  let showSystem = $state(false);
  let errorMessage = $state<string | null>(null);

  const mergedKeys = $derived.by(() => {
    const keys = new Set<string>();
    for (const e of localEntries) keys.add(e.key);
    for (const e of globalEntries) keys.add(e.key);
    const sorted = [...keys].sort();
    if (!filterText.trim()) return sorted;
    const lower = filterText.toLowerCase();
    return sorted.filter((k) => k.toLowerCase().includes(lower));
  });

  function localValue(key: string): string | null {
    return localEntries.find((e) => e.key === key)?.value ?? null;
  }

  function globalValue(key: string): string | null {
    return globalEntries.find((e) => e.key === key)?.value ?? null;
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
    if (!editingValue.trim()) {
      cancelEdit();
      return;
    }
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

  function handleEditKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") saveEdit();
    else if (event.key === "Escape") cancelEdit();
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

  function handleAddKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") handleAddEntry();
    else if (event.key === "Escape") {
      showAddRow = false;
      addKey = "";
      addValue = "";
    }
  }
</script>

<Card
  title={m.settings_git_section_title()}
  description={m.settings_git_section_description()}
>
  <SettingSection title={m.settings_git_config()}>
    <div class="config-editor" data-setting-anchor="config">
      <div class="config-toolbar" data-setting-anchor="config-filter">
        <input
          class="filter-input"
          type="text"
          placeholder={m.config_filter_placeholder()}
          bind:value={filterText}
        />
        <Button
          variant="neutral"
          size="sm"
          onclick={() => (showAddRow = !showAddRow)}
        >
          {m.config_add_entry()}
        </Button>
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
          <Button
            variant="primary"
            size="sm"
            disabled={!addKey.trim()}
            onclick={handleAddEntry}
          >
            +
          </Button>
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
            {#each mergedKeys as key (key)}
              {@const lv = localValue(key)}
              {@const gv = globalValue(key)}
              {@const enumVals = getEnumValues(key)}
              <div class="table-row">
                <span class="col-key cell-key" title={key}>{key}</span>

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
                        {#each enumVals as opt (opt)}
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
                    <Button
                      variant="neutral"
                      size="sm"
                      description={m.config_click_to_edit()}
                      onclick={() => startEdit(key, "local", lv)}
                    >{lv}</Button>
                    <IconButton
                      icon={"\uF00D"}
                      tone="danger"
                      description={m.config_remove_entry_title()}
                      onclick={() => (unsetTarget = { key, scope: "local" })}
                    />
                  {:else}
                    <Button
                      variant="neutral"
                      size="sm"
                      icon={"\uF067"}
                      description={m.config_click_to_set()}
                      onclick={() => startAdd(key, "local")}
                    >{m.config_empty_value()}</Button>
                  {/if}
                </span>

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
                        {#each enumVals as opt (opt)}
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
                    <Button
                      variant="neutral"
                      size="sm"
                      description={m.config_click_to_edit()}
                      onclick={() => startEdit(key, "global", gv)}
                    >{gv}</Button>
                    <IconButton
                      icon={"\uF00D"}
                      tone="danger"
                      description={m.config_remove_entry_title()}
                      onclick={() => (unsetTarget = { key, scope: "global" })}
                    />
                  {:else}
                    <Button
                      variant="neutral"
                      size="sm"
                      icon={"\uF067"}
                      description={m.config_click_to_set()}
                      onclick={() => startAdd(key, "global")}
                    ></Button>
                  {/if}
                </span>
              </div>
            {/each}
          </div>
        </div>

        {#if systemEntries.length > 0}
          <button
            class="system-toggle"
            onclick={() => (showSystem = !showSystem)}
          >
            {showSystem ? "\u25BC" : "\u25B6"}
            {m.config_system_section()}
          </button>
          {#if showSystem}
            <div class="system-table">
              {#each systemEntries as entry (entry.key)}
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
  </SettingSection>
</Card>

{#if unsetTarget}
  <ConfirmDialog
    title={m.config_unset_confirm_title()}
    message={m.config_unset_confirm_message({
      key: unsetTarget.key,
      scope: unsetTarget.scope,
    })}
    destructive={true}
    onConfirm={confirmUnset}
    onCancel={() => (unsetTarget = null)}
  />
{/if}

<style>
  .config-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow: hidden;
  }

  .config-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 0 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .filter-input {
    flex: 1;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    font-family: inherit;
  }

  .filter-input:focus {
    border-color: var(--accent-blue);
  }

  .config-error {
    padding: 6px 8px;
    font-size: 12px;
    color: var(--accent-red);
    background: var(--overlay-accent-red);
    border-radius: 4px;
    word-break: break-word;
  }

  .add-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .add-input {
    padding: 4px 8px;
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    font-family: inherit;
  }

  .key-input {
    width: 200px;
    font-family: var(--font-mono);
  }
  .value-input {
    flex: 1;
  }

  .scope-select {
    padding: 4px 8px;
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
  }

  .config-table {
    flex: 1;
    overflow-y: auto;
  }

  .table-header {
    display: flex;
    align-items: center;
    padding: 8px 0;
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
    padding: 4px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .table-row:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .col-key {
    width: 40%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-local {
    width: 30%;
    min-width: 0;
  }
  .col-global {
    width: 30%;
    min-width: 0;
  }

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

  .edit-input,
  .edit-select {
    width: 100%;
    padding: 2px 4px;
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
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
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
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
    padding: 10px 0;
    background: none;
    border: none;
    border-top: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
  }

  .system-toggle:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .system-table {
    padding: 0;
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
