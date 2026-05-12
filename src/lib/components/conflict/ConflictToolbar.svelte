<!--
  ConflictToolbar — Banner displayed during merge/rebase/cherry-pick/revert conflicts.

  Shows the current operation state, lists conflicted files, and provides
  Abort and Continue buttons. Continue is disabled until all conflicts are
  resolved. Automatically hidden when no operation is in progress.
-->
<script lang="ts">
  import {
    conflictStatus,
    isInConflict,
    abortOperation,
    continueOperation,
    refreshConflictStatus,
  } from "../../stores/conflict";
  import { getConflictFileContents, writeResolvedFile } from "../../api/tauri";
  import type { ConflictFileContents } from "../../types";
  import MergeEditor from "../editor/MergeEditor.svelte";
  import { activeTheme } from "../../stores/theme";
  import * as m from "$lib/paraglide/messages";
  import { Button } from "$lib/components/ui";

  /** The file path currently open in the merge editor. */
  let mergeFile = $state<string | null>(null);
  /** Conflict contents for the open file. */
  let mergeContents = $state<ConflictFileContents | null>(null);
  /** Whether the file list is expanded. */
  let showFileList = $state(false);

  /** Open a conflicted file in the merge editor. */
  async function openConflictFile(path: string) {
    try {
      const contents = await getConflictFileContents(path);
      mergeFile = path;
      mergeContents = contents;
    } catch {
      // File may have been resolved externally.
    }
  }

  /** Handle resolve from the merge editor: write content and refresh status. */
  async function handleResolve(content: string) {
    if (!mergeFile) return;
    try {
      await writeResolvedFile(mergeFile, content);
    } catch {
      // Write may fail if file was already resolved.
    }
    mergeFile = null;
    mergeContents = null;
    await refreshConflictStatus();
  }

  /** Close the merge editor without resolving. */
  function handleCancelMerge() {
    mergeFile = null;
    mergeContents = null;
  }

  function getAbortLabel(state: string): string {
    switch (state) {
      case "merging":
        return m.conflict_abort_merge();
      case "rebasing":
        return m.conflict_abort_rebase();
      case "cherry_picking":
        return m.conflict_abort_cherry_pick();
      case "reverting":
        return m.conflict_abort_revert();
      default:
        return "";
    }
  }

  function getContinueLabel(state: string): string {
    switch (state) {
      case "merging":
        return m.conflict_continue_merge();
      case "rebasing":
        return m.conflict_continue_rebase();
      case "cherry_picking":
        return m.conflict_continue_cherry_pick();
      case "reverting":
        return m.conflict_continue_revert();
      default:
        return "";
    }
  }

  function getStateLabel(state: string): string {
    switch (state) {
      case "merging":
        return m.conflict_state_merging();
      case "rebasing":
        return m.conflict_state_rebasing();
      case "cherry_picking":
        return m.conflict_state_cherry_picking();
      case "reverting":
        return m.conflict_state_reverting();
      default:
        return "";
    }
  }

  let fileCountText = $derived(
    $conflictStatus.conflicted_files.length === 0
      ? m.conflict_resolved_all()
      : $conflictStatus.conflicted_files.length === 1
        ? m.conflict_toolbar_files_one({ count: 1 })
        : m.conflict_toolbar_files({ count: $conflictStatus.conflicted_files.length }),
  );
</script>

{#if $isInConflict}
  <div class="conflict-toolbar">
    <div class="conflict-left">
      <span class="conflict-icon">{"\uF071"}</span>
      <span class="conflict-state">{getStateLabel($conflictStatus.state)}</span>
      <span class="conflict-separator">—</span>
      <button
        class="conflict-files-btn"
        class:resolved={$conflictStatus.can_continue}
        onclick={() => showFileList = !showFileList}
      >
        {fileCountText}
        <span class="chevron" class:open={showFileList}>{"\uF078"}</span>
      </button>
    </div>
    <div class="conflict-right">
      <Button variant="danger" onclick={abortOperation}>
        {getAbortLabel($conflictStatus.state)}
      </Button>
      <Button
        variant="primary"
        disabled={!$conflictStatus.can_continue}
        description={$conflictStatus.can_continue ? "" : m.conflict_continue_disabled_tooltip()}
        onclick={continueOperation}
      >
        {getContinueLabel($conflictStatus.state)}
      </Button>
    </div>
  </div>
  {#if showFileList && $conflictStatus.conflicted_files.length > 0}
    <div class="conflict-file-list">
      {#each $conflictStatus.conflicted_files as file}
        <button class="conflict-file-item" onclick={() => openConflictFile(file)}>
          <span class="file-icon">{"\uF15C"}</span>
          <span class="file-path">{file}</span>
        </button>
      {/each}
    </div>
  {/if}
{/if}

{#if mergeFile && mergeContents}
  <div class="merge-editor-overlay">
    <MergeEditor
      ours={mergeContents.ours}
      theirs={mergeContents.theirs}
      base={mergeContents.base}
      filename={mergeFile}
      editorTheme={$activeTheme?.editor}
      isDark={$activeTheme?.meta.mode !== 'light'}
      onResolve={handleResolve}
      onCancel={handleCancelMerge}
    />
  </div>
{/if}

<style>
  .conflict-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background: color-mix(in srgb, var(--accent-orange) 10%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--accent-orange) 30%, transparent);
    flex-shrink: 0;
  }

  .conflict-left {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .conflict-icon {
    font-family: var(--font-icons);
    color: var(--accent-orange);
    font-size: 14px;
  }

  .conflict-state {
    font-weight: 700;
    color: var(--accent-orange);
    font-size: 11px;
    letter-spacing: 0.5px;
  }

  .conflict-separator {
    color: var(--text-secondary);
  }

  .conflict-files-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    padding: 0;
  }

  .conflict-files-btn:hover {
    color: var(--accent-primary);
  }

  .conflict-files-btn.resolved {
    color: var(--accent-green);
  }

  .chevron {
    font-family: var(--font-icons);
    font-size: 9px;
    transition: transform 0.15s;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  .conflict-right {
    display: flex;
    gap: 8px;
  }

  .conflict-file-list {
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--accent-orange) 5%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--accent-orange) 20%, transparent);
  }

  .conflict-file-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 16px 4px 32px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }

  .conflict-file-item:hover {
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    color: var(--accent-primary);
  }

  .file-icon {
    font-family: var(--font-icons);
    font-size: 12px;
    color: var(--accent-orange);
    flex-shrink: 0;
  }

  .file-path {
    font-family: var(--font-mono);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .merge-editor-overlay {
    position: fixed;
    inset: 0;
    z-index: 900;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
  }
</style>
