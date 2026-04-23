<!--
  AiConfigEditor.svelte — split-panel AI config editor view.

  Left panel: file tree (project + user config files).
  Right panel: CodeMirror editor for the selected file, with toolbar
  showing filename, dirty state, scope/language badges, and save button.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import AiConfigFileTree from "./AiConfigFileTree.svelte";
  import CreateConfigDialog from "./CreateConfigDialog.svelte";
  import CodeEditor from "../editor/CodeEditor.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import {
    configFiles,
    activeFilePath,
    activeFileContent,
    activeFileDirty,
    configLoading,
    configFileChangedOnDisk,
    loadConfigFiles,
    openFile,
    saveFile,
    markDirty,
    startConfigWatcher,
    stopConfigWatcher,
    reloadActiveFile,
    dismissDiskChange,
  } from "../../stores/aiConfig";
  import { activeTheme } from "../../stores/theme";
  import * as m from "$lib/paraglide/messages";
  import type { AiConfigFile } from "../../types";

  // ─── Local state ───

  /** Current editor content (may differ from saved). */
  let editorContent = $state("");

  /** Whether the create-file dialog is open. */
  let showCreateDialog = $state(false);

  /** Default scope to pass to the create dialog. */
  let createDialogScope = $state("project");

  /** Whether the discard-changes confirm dialog is open. */
  let showDiscardDialog = $state(false);

  /** Path the user wants to switch to (while dirty). */
  let pendingFilePath = $state<string | null>(null);

  // ─── Derived ───

  /** The AiConfigFile entry for the currently open file. */
  let activeFile = $derived.by<AiConfigFile | null>(() => {
    const path = $activeFilePath;
    if (!path) return null;
    return $configFiles.find((f) => f.path === path) ?? null;
  });

  /** Display filename (last segment of path). */
  let displayName = $derived.by(() => {
    const path = $activeFilePath;
    if (!path) return "";
    const i = path.lastIndexOf("/");
    return i >= 0 ? path.substring(i + 1) : path;
  });

  /** Language badge based on file extension. */
  let languageBadge = $derived.by(() => {
    if (!displayName) return "";
    if (displayName.endsWith(".json")) return "json";
    if (displayName.endsWith(".md")) return "markdown";
    if (displayName.endsWith(".toml")) return "toml";
    if (displayName.endsWith(".yaml") || displayName.endsWith(".yml")) return "yaml";
    return "";
  });

  // ─── Lifecycle ───

  onMount(() => {
    loadConfigFiles();
    startConfigWatcher();
  });

  onDestroy(() => {
    stopConfigWatcher();
  });

  // ─── Sync editor content when active file changes ───

  $effect(() => {
    const content = $activeFileContent;
    if (content !== null) {
      editorContent = content;
    }
  });

  // ─── Keyboard shortcut: Cmd+S / Ctrl+S ───

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      if ($activeFileDirty && $activeFilePath) {
        saveFile(editorContent);
      }
    }
  }

  // ─── Handlers ───

  function handleSelectFile(path: string) {
    if (path === $activeFilePath) return;
    if ($activeFileDirty) {
      pendingFilePath = path;
      showDiscardDialog = true;
    } else {
      openFile(path);
    }
  }

  function handleDiscardConfirm() {
    showDiscardDialog = false;
    if (pendingFilePath) {
      openFile(pendingFilePath);
      pendingFilePath = null;
    }
  }

  function handleDiscardCancel() {
    showDiscardDialog = false;
    pendingFilePath = null;
  }

  function handleCreateFile(scope: string) {
    createDialogScope = scope;
    showCreateDialog = true;
  }

  function handleEditorChange(content: string) {
    editorContent = content;
    if (content !== get(activeFileContent)) {
      markDirty();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="ai-config-editor">
  <!-- Left panel: file tree -->
  <div class="file-tree-panel">
    <AiConfigFileTree
      onSelectFile={handleSelectFile}
      onCreateFile={handleCreateFile}
    />
  </div>

  <!-- Right panel: editor or empty state -->
  <div class="editor-panel">
    {#if $activeFilePath && $activeFileContent !== null}
      <!-- Toolbar -->
      <div class="toolbar">
        <span class="toolbar-filename">{displayName}</span>
        {#if $activeFileDirty}
          <span class="dirty-dot" title={m.ai_config_unsaved()}></span>
        {/if}
        {#if activeFile}
          <span class="badge badge-scope">{activeFile.scope}</span>
        {/if}
        {#if languageBadge}
          <span class="badge badge-lang">{languageBadge}</span>
        {/if}
        <div class="toolbar-spacer"></div>
        <button
          class="save-btn"
          class:highlight={$activeFileDirty}
          disabled={!$activeFileDirty}
          onclick={() => saveFile(editorContent)}
        >
          {m.ai_config_save()} <kbd class="save-kbd">{navigator.platform.includes("Mac") ? "\u2318S" : "Ctrl+S"}</kbd>
        </button>
        {#if $configFileChangedOnDisk}
          <div class="disk-change-notice">
            <span>{m.ai_config_changed_on_disk()}</span>
            <button class="notice-btn" onclick={() => reloadActiveFile()}>
              {m.ai_config_reload()}
            </button>
            <button class="notice-btn dismiss" onclick={() => dismissDiskChange()}>
              {m.ai_config_dismiss()}
            </button>
          </div>
        {/if}
      </div>

      <!-- CodeMirror editor -->
      <div class="editor-area">
        <CodeEditor
          content={editorContent}
          filename={displayName}
          editorTheme={$activeTheme?.editor}
          isDark={$activeTheme?.meta.mode !== "light"}
          readonly={false}
          onChange={handleEditorChange}
        />
      </div>
    {:else if $configLoading}
      <div class="empty-state">
        <div class="spinner"></div>
      </div>
    {:else}
      <div class="empty-state">
        <span class="empty-icon nf">{"\uF15C"}</span>
        <span class="empty-text">{m.ai_config_select_file()}</span>
      </div>
    {/if}
  </div>
</div>

<!-- Create dialog -->
{#if showCreateDialog}
  <CreateConfigDialog
    defaultScope={createDialogScope}
    onClose={() => { showCreateDialog = false; }}
  />
{/if}

<!-- Discard changes dialog -->
{#if showDiscardDialog}
  <ConfirmDialog
    title={m.ai_config_discard()}
    message={m.ai_config_discard_confirm()}
    confirmLabel={m.ai_config_discard_btn()}
    destructive={true}
    onConfirm={handleDiscardConfirm}
    onCancel={handleDiscardCancel}
  />
{/if}

<style>
  .ai-config-editor {
    display: flex;
    flex: 1;
    overflow: hidden;
    height: 100%;
  }

  /* ─── Left panel ─── */

  .file-tree-panel {
    width: 240px;
    flex-shrink: 0;
    overflow-y: auto;
    overflow-x: hidden;
    border-right: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  /* ─── Right panel ─── */

  .editor-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  /* ─── Toolbar ─── */

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .toolbar-filename {
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dirty-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-yellow);
    flex-shrink: 0;
  }

  .badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    flex-shrink: 0;
    font-weight: 500;
  }

  .badge-scope {
    background: color-mix(in srgb, var(--accent-blue) 12%, transparent);
    color: var(--accent-blue);
    border: 1px solid color-mix(in srgb, var(--accent-blue) 20%, transparent);
  }

  .badge-lang {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .toolbar-spacer {
    flex: 1;
  }

  .save-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    border: 1px solid var(--border);
    border-radius: 5px;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
    flex-shrink: 0;
  }

  .save-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .save-btn.highlight {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    color: var(--text-primary);
    opacity: 1;
  }

  .save-btn.highlight:hover {
    opacity: 0.9;
  }

  .save-kbd {
    font-family: var(--font-mono);
    font-size: 10px;
    opacity: 0.7;
    background: none;
    border: none;
    padding: 0;
  }

  /* ─── Disk change notice ─── */

  .disk-change-notice {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: color-mix(in srgb, var(--accent-orange) 15%, transparent);
    border-radius: 4px;
    font-size: 11px;
    color: var(--accent-orange);
  }

  .notice-btn {
    background: none;
    border: 1px solid var(--accent-orange);
    color: var(--accent-orange);
    font-size: 10px;
    padding: 2px 8px;
    border-radius: 3px;
    cursor: pointer;
  }

  .notice-btn:hover {
    background: color-mix(in srgb, var(--accent-orange) 10%, transparent);
  }

  .notice-btn.dismiss {
    border-color: var(--border);
    color: var(--text-secondary);
  }

  .notice-btn.dismiss:hover {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }

  /* ─── Editor area ─── */

  .editor-area {
    flex: 1;
    overflow: hidden;
  }

  /* ─── Empty state ─── */

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-secondary);
  }

  .empty-icon {
    font-size: 32px;
    opacity: 0.3;
  }

  .empty-text {
    font-size: 13px;
    font-style: italic;
    opacity: 0.5;
  }
</style>
