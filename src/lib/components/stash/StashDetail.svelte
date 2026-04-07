<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import DiffEditor from "../editor/DiffEditor.svelte";
  import { stashes, selectedStashIndex, selectedStashDiff, doStashApplyFile } from "../../stores/stashes";
  import { activeTheme } from "../../stores/theme";
  import { fileDiffToContents } from "../../utils/diff";
  import { formatDateTime } from "../../utils/time";
  import type { StashEntry } from "../../types";

  let selectedEntry = $derived<StashEntry | undefined>(
    $stashes.find((e) => e.index === $selectedStashIndex)
  );
</script>

{#if selectedEntry && $selectedStashDiff}
  <div class="stash-detail">
    <div class="stash-detail-header">
      <div class="stash-detail-title">{selectedEntry.message || `stash@{${selectedEntry.index}}`}</div>
      <div class="stash-detail-meta">
        <span>{m.stash_on_branch({ branch: selectedEntry.branch })}</span>
        <span>{formatDateTime(selectedEntry.timestamp)}</span>
        <span class="stash-oid">{selectedEntry.oid.slice(0, 8)}</span>
      </div>
    </div>
    <div class="stash-detail-diffs">
      {#each $selectedStashDiff as diff (diff.path)}
        {@const contents = fileDiffToContents(diff)}
        <div class="stash-diff-item">
          <div class="stash-diff-header">
            <span class="stash-diff-path">{diff.path}</span>
            <div class="stash-diff-actions">
              <button
                class="apply-file-btn"
                onclick={() => {
                  if ($selectedStashIndex !== null) doStashApplyFile($selectedStashIndex, diff.path);
                }}
              >Apply</button>
              <span class="stash-diff-stats">
                <span class="stat-add">+{diff.additions}</span>
                <span class="stat-del">-{diff.deletions}</span>
              </span>
            </div>
          </div>
          <div class="stash-diff-editor">
            <DiffEditor
              oldContent={contents.oldContent}
              newContent={contents.newContent}
              filename={diff.path}
              editorTheme={$activeTheme?.editor}
              isDark={$activeTheme?.meta.mode !== 'light'}
            />
          </div>
        </div>
      {/each}
    </div>
  </div>
{:else}
  <div class="detail-empty">
    <p>{m.stash_select_preview()}</p>
  </div>
{/if}

<style>
  .stash-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .stash-detail-header {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .stash-detail-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .stash-detail-meta {
    display: flex;
    gap: 12px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .stash-oid {
    font-family: var(--font-mono);
    color: var(--accent-blue);
  }

  .stash-detail-diffs {
    flex: 1;
    overflow-y: auto;
  }

  .stash-diff-item {
    border-bottom: 1px solid var(--border);
  }

  .stash-diff-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .stash-diff-path {
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .stash-diff-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .apply-file-btn {
    padding: 2px 8px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    font-size: 10px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .apply-file-btn:hover {
    background: rgba(63, 185, 80, 0.15);
    border-color: var(--accent-green);
    color: var(--accent-green);
  }

  .stash-diff-stats {
    font-size: 11px;
  }

  .stat-add {
    color: var(--accent-green);
    margin-right: 6px;
  }

  .stat-del {
    color: #f85149;
  }

  .stash-diff-editor {
    height: 300px;
  }

</style>
