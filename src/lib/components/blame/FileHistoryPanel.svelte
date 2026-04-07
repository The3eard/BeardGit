<!--
  FileHistoryPanel.svelte — Commit history for a specific file.

  Shows a scrollable list of commits that touched the file, with
  diff stats, rename detection, and click-to-blame navigation.
-->
<script lang="ts">
  import type { FileHistoryEntry } from '$lib/types';
  import { formatRelativeTime } from '$lib/utils/time';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    entries: FileHistoryEntry[];
    loading: boolean;
    onCommitClick?: (oid: string) => void;
  }

  let { entries, loading, onCommitClick }: Props = $props();

  function shortOid(oid: string): string {
    return oid.slice(0, 7);
  }

  function truncateMessage(msg: string, max = 60): string {
    const first = msg.split('\n')[0];
    return first.length > max ? first.slice(0, max - 1) + '\u2026' : first;
  }
</script>

<div class="file-history-panel">
  {#if loading}
    <div class="history-loading">
      <div class="spinner"></div>
      <span>{m.blame_loading()}</span>
    </div>
  {:else if entries.length === 0}
    <div class="history-empty">
      <p>{m.file_history_empty()}</p>
    </div>
  {:else}
    <div class="history-list">
      {#each entries as entry}
        <button
          class="history-row"
          onclick={() => onCommitClick?.(entry.oid)}
        >
          <div class="row-top">
            <span class="oid">{shortOid(entry.oid)}</span>
            <span class="message">{truncateMessage(entry.message)}</span>
          </div>
          <div class="row-bottom">
            <span class="author">{entry.author}</span>
            <span class="date">{formatRelativeTime(entry.date)}</span>
            <span class="stats">
              <span class="additions">+{entry.additions}</span>
              <span class="deletions">-{entry.deletions}</span>
            </span>
          </div>
          {#if entry.old_path}
            <div class="rename-badge">
              {m.file_history_renamed_from({ path: entry.old_path })}
            </div>
          {/if}
        </button>
      {/each}
    </div>
    <div class="history-hint">
      <p>{m.file_history_click_to_blame()}</p>
    </div>
  {/if}
</div>

<style>
  .file-history-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .history-loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .history-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .history-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .history-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    font-size: 12px;
    transition: background 0.1s;
  }

  .history-row:hover {
    background: var(--bg-secondary);
  }

  .row-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .oid {
    font-family: 'Fira Code', var(--font-mono), monospace;
    color: var(--accent-blue);
    flex-shrink: 0;
    font-size: 11px;
  }

  .message {
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  .row-bottom {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .author {
    flex-shrink: 0;
  }

  .date {
    opacity: 0.7;
  }

  .stats {
    margin-left: auto;
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .additions {
    color: var(--accent-green);
  }

  .deletions {
    color: var(--accent-red);
  }

  .rename-badge {
    font-size: 10px;
    color: var(--accent-orange);
    font-style: italic;
    padding-top: 2px;
  }

  .history-hint {
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    opacity: 0.7;
    text-align: center;
    flex-shrink: 0;
  }

  .history-hint p {
    margin: 0;
  }
</style>
