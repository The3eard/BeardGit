<!--
  BlameView.svelte — Full blame view with gutter annotations and code.

  Displays per-line blame info alongside syntax-highlighted file content.
  Includes a tab toggle between "Blame" and "History" sub-views.
-->
<script lang="ts">
  import CodeEditor from '$lib/components/editor/CodeEditor.svelte';
  import BlameGutter from './BlameGutter.svelte';
  import FileHistoryPanel from './FileHistoryPanel.svelte';
  import {
    blamePath,
    blameOid,
    blameLines,
    blameLoading,
    blameError,
    blameActiveTab,
    blamePreviousView,
    fileHistoryEntries,
    fileHistoryLoading,
    loadBlame,
    loadFileHistory,
  } from '$lib/stores/blame';
  import { activeTheme } from '$lib/stores/theme';
  import EmptyState from '$lib/components/common/EmptyState.svelte';
  import { IconButton } from '$lib/components/ui';
  import { shortOid } from '$lib/utils/git';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    onNavigateBack?: (view: string) => void;
  }

  let { onNavigateBack }: Props = $props();

  /** Reconstruct file content from blame lines for the code editor. */
  let fileContent = $derived(
    $blameLines.map((l) => l.content).join('\n')
  );

  /** Synchronize scroll between gutter and code editor. */
  let gutterEl: HTMLDivElement | undefined = $state();
  let editorEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!gutterEl || !editorEl) return;

    const scroller = editorEl.querySelector('.cm-scroller');
    if (!scroller) return;

    function syncGutter() {
      if (gutterEl && scroller) {
        gutterEl.scrollTop = scroller.scrollTop;
      }
    }

    scroller.addEventListener('scroll', syncGutter);
    return () => scroller.removeEventListener('scroll', syncGutter);
  });

  function handleClose() {
    onNavigateBack?.($blamePreviousView);
  }

  function handleOidClick(oid: string) {
    if ($blamePath) {
      loadBlame($blamePath, oid);
    }
  }

  function handleHistoryCommitClick(oid: string) {
    if ($blamePath) {
      blameActiveTab.set('blame');
      loadBlame($blamePath, oid);
    }
  }

  function setTab(tab: 'blame' | 'history') {
    blameActiveTab.set(tab);
  }
</script>

<div class="blame-view">
  <!-- Header -->
  <div class="blame-header">
    <div class="header-left">
      <span class="file-path">{$blamePath ?? ''}</span>
      {#if $blameOid}
        <span class="at-commit">
          {m.blame_at_commit({ sha: shortOid($blameOid) })}
        </span>
      {/if}
    </div>
    <div class="header-right">
      <div class="tab-toggle">
        <button
          class="tab-btn"
          class:active={$blameActiveTab === 'blame'}
          onclick={() => setTab('blame')}
        >
          {m.blame_title()}
        </button>
        <button
          class="tab-btn"
          class:active={$blameActiveTab === 'history'}
          onclick={() => setTab('history')}
        >
          {m.file_history_title()}
        </button>
      </div>
      <IconButton icon={"\uF00D"} description={m.tooltip_close()} onclick={handleClose} />
    </div>
  </div>

  <!-- Content -->
  <div class="blame-content">
    {#if $blameActiveTab === 'blame'}
      {#if $blameLoading}
        <div class="blame-placeholder">
          <div class="spinner"></div>
          <span>{m.blame_loading()}</span>
        </div>
      {:else if $blameError}
        <div class="blame-placeholder">
          <span class="error-text">{$blameError}</span>
        </div>
      {:else if !$blamePath || $blameLines.length === 0}
        <EmptyState fill icon={"\uF15C"} title={m.blame_no_file()} />
      {:else}
        <div class="blame-split">
          <div class="gutter-scroll" bind:this={gutterEl}>
            <BlameGutter
              lines={$blameLines}
              onOidClick={handleOidClick}
            />
          </div>
          <div class="editor-area" bind:this={editorEl}>
            <CodeEditor
              content={fileContent}
              filename={$blamePath ?? ''}
              editorTheme={$activeTheme?.editor}
              isDark={$activeTheme?.meta.mode !== 'light'}
              readonly={true}
            />
          </div>
        </div>
      {/if}
    {:else}
      <FileHistoryPanel
        entries={$fileHistoryEntries}
        loading={$fileHistoryLoading}
        onCommitClick={handleHistoryCommitClick}
      />
    {/if}
  </div>
</div>

<style>
  .blame-view {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .blame-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-toolbar);
    flex-shrink: 0;
    gap: 12px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .file-path {
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: var(--font-size-md);
    color: var(--accent-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .at-commit {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    background: var(--bg-secondary);
    padding: 2px 6px;
    border-radius: 4px;
    flex-shrink: 0;
    font-family: 'Fira Code', var(--font-mono), monospace;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .tab-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .tab-btn {
    padding: 4px 12px;
    font-size: var(--font-size-sm);
    font-weight: 500;
    border: none;
    background: var(--bg-secondary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .tab-btn:first-child {
    border-right: 1px solid var(--border);
  }

  .tab-btn.active {
    background: var(--accent-primary);
    color: var(--text-primary);
  }

  .tab-btn:not(.active):hover {
    background: var(--bg-primary);
  }

  .blame-content {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .blame-placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: var(--font-size-md);
  }

  .error-text {
    color: var(--accent-orange);
    max-width: 400px;
    text-align: center;
    word-break: break-word;
  }

  .blame-split {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .gutter-scroll {
    overflow-y: hidden;
    flex-shrink: 0;
  }

  .editor-area {
    flex: 1;
    overflow: hidden;
    min-width: 0;
  }
</style>
