<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import FileChangeList from "../common/FileChangeList.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import SideBySideDiff from "../diff/SideBySideDiff.svelte";
  import CommitDetail from "../detail/CommitDetail.svelte";
  import { formatRelativeTime } from "../../utils/time";
  import { getCommitFileDiff, getCommitDetail, getCommitFiles } from "../../api/tauri";
  import { navigateToCommit } from "../../stores/graph";
  import type { FileDiff, CommitInfo, CommitFileChange } from "../../types";
  import {
    selectedTagName,
    selectedTagInfo,
    selectedCommitInfo,
    selectedCommitStats,
    selectedCommitFiles,
    loadingDetail,
    doDeleteTag,
    doPushTag,
  } from "../../stores/tags";

  let confirmDelete = $state(false);
  let fileDiff = $state<FileDiff | null>(null);
  let parentCommit = $state<CommitInfo | null>(null);
  let parentFiles = $state<CommitFileChange[]>([]);

  // Clear transient state when selected tag changes
  $effect(() => {
    $selectedTagName;
    fileDiff = null;
    parentCommit = null;
    parentFiles = [];
  });

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }

  let statsBarWidth = $derived.by(() => {
    if (!$selectedCommitStats) return { add: 50, del: 50 };
    const total =
      $selectedCommitStats.insertions + $selectedCommitStats.deletions;
    if (total === 0) return { add: 50, del: 50 };
    return {
      add: Math.round(($selectedCommitStats.insertions / total) * 100),
      del: Math.round(($selectedCommitStats.deletions / total) * 100),
    };
  });

  async function handleFileClick(path: string) {
    if (!$selectedCommitInfo) return;
    const diffs = await getCommitFileDiff($selectedCommitInfo.oid, path);
    fileDiff = diffs.length > 0 ? diffs[0] : null;
  }

  async function handleParentClick(oid: string) {
    parentCommit = null;
    parentFiles = [];
    const [commit, files] = await Promise.all([
      getCommitDetail(oid),
      getCommitFiles(oid).catch(() => [] as CommitFileChange[]),
    ]);
    parentCommit = commit;
    parentFiles = files;
  }
</script>

<div class="tag-detail">
  {#if $loadingDetail}
    <div class="detail-loading">
      <div class="spinner"></div>
      <span>{m.tags_loading_detail()}</span>
    </div>
  {:else if $selectedTagInfo && $selectedCommitInfo}
    <!-- Header -->
    <div class="detail-header">
      <div class="detail-title-row">
        <span class="detail-title">{$selectedTagInfo.name}</span>
        {#if $selectedTagInfo.annotated}
          <span class="detail-badge-annotated">{m.tags_badge_annotated()}</span>
        {:else}
          <span class="detail-badge-lightweight">lightweight</span>
        {/if}
      </div>
      <div class="detail-meta">
        <span class="meta-oid">{$selectedTagInfo.commit_oid.slice(0, 8)}</span>
        {#if $selectedTagInfo.annotated && $selectedTagInfo.tagger_name}
          <span>{m.tags_detail_tagged_by({ author: $selectedTagInfo.tagger_name })}</span>
          <span>{formatRelativeTime($selectedTagInfo.date)}</span>
        {:else}
          <span>{$selectedCommitInfo.author}</span>
          <span>{formatDate($selectedCommitInfo.timestamp)}</span>
        {/if}
      </div>
    </div>

    <!-- Scrollable body -->
    <div class="detail-body">
      <!-- Tag message (annotated only) -->
      {#if $selectedTagInfo.annotated && $selectedTagInfo.message}
        <div class="detail-section">
          <div class="section-label">{m.tags_detail_message()}</div>
          <div class="section-card message-card">
            {$selectedTagInfo.message}
          </div>
        </div>
      {/if}

      <!-- Commit -->
      <div class="detail-section">
        <div class="section-label">{m.tags_detail_commit()}</div>
        <div class="section-card">
          <div class="commit-summary">{$selectedCommitInfo.summary}</div>
          {#if $selectedCommitInfo.body}
            <div class="commit-body">{$selectedCommitInfo.body}</div>
          {/if}
          <div class="commit-footer">
            <span class="commit-author">{$selectedCommitInfo.author}</span>
            <span class="commit-sha">{$selectedCommitInfo.oid.slice(0, 8)}</span>
            <span class="commit-date">{formatDate($selectedCommitInfo.timestamp)}</span>
          </div>
        </div>
      </div>

      <!-- Changes -->
      {#if $selectedCommitStats}
        <div class="detail-section">
          <div class="section-label">{m.tags_detail_changes()}</div>
          <div class="section-card stats-card">
            <div class="stats-row">
              <div class="stat-item">
                <span class="stat-label">{m.tags_detail_files_changed()}:</span>
                <span class="stat-value">{$selectedCommitStats.files_changed}</span>
              </div>
              <span class="stat-additions">+{$selectedCommitStats.insertions}</span>
              <span class="stat-deletions">-{$selectedCommitStats.deletions}</span>
            </div>
            {#if $selectedCommitStats.insertions + $selectedCommitStats.deletions > 0}
              <div class="stats-bar">
                <div
                  class="stats-bar-add"
                  style="width: {statsBarWidth.add}%"
                ></div>
                <div
                  class="stats-bar-del"
                  style="width: {statsBarWidth.del}%"
                ></div>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Changed files -->
      {#if $selectedCommitFiles && $selectedCommitFiles.length > 0}
        <div class="detail-section">
          <div class="section-label">{m.commit_detail_files({ count: String($selectedCommitFiles.length) })}</div>
          <div class="section-card files-card">
            <FileChangeList files={$selectedCommitFiles} onSelect={handleFileClick} />
          </div>
        </div>
      {/if}

      <!-- File diff preview (B10) -->
      {#if fileDiff}
        <div class="tag-diff-preview">
          <SideBySideDiff diff={fileDiff} onClose={() => { fileDiff = null; }} />
        </div>
      {/if}

      <!-- Parents -->
      {#if $selectedCommitInfo.parents.length > 0}
        <div class="detail-section">
          <div class="section-label">{m.tags_detail_parents()}</div>
          <div class="parents-row">
            {#each $selectedCommitInfo.parents as parent}
              <button class="parent-oid clickable" onclick={() => handleParentClick(parent)}>
                {parent.slice(0, 8)}
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Parent commit detail panel (B11) -->
      {#if parentCommit}
        <div class="parent-detail-panel">
          <CommitDetail
            commit={parentCommit}
            files={parentFiles}
            showNavigateToGraph={true}
            onNavigateToGraph={navigateToCommit}
            onClose={() => { parentCommit = null; parentFiles = []; }}
          />
        </div>
      {/if}
    </div>

    <!-- Actions footer -->
    <div class="detail-actions">
      <button class="action-btn action-push" onclick={() => doPushTag($selectedTagInfo!.name, "origin")}>
        {m.tags_action_push()}
      </button>
      <button class="action-btn action-delete" onclick={() => (confirmDelete = true)}>
        {m.tags_action_delete()}
      </button>
    </div>
  {:else if !$selectedTagName}
    <div class="detail-empty">
      <span>{m.tags_select_preview()}</span>
    </div>
  {/if}

  {#if confirmDelete && $selectedTagInfo}
    <ConfirmDialog
      title={m.tags_delete_dialog_title()}
      detail={`${$selectedTagInfo.name}\n${$selectedTagInfo.commit_oid.slice(0, 8)}`}
      message={m.tags_delete_body({ name: $selectedTagInfo.name })}
      confirmLabel={m.tags_delete_confirm()}
      destructive={true}
      onConfirm={() => {
        doDeleteTag($selectedTagInfo!.name);
        confirmDelete = false;
      }}
      onCancel={() => (confirmDelete = false)}
    />
  {/if}
</div>

<style>
  .tag-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .detail-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .detail-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .detail-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .detail-badge-annotated {
    font-size: 9px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    background: rgba(240, 136, 62, 0.15);
    color: #f0883e;
    text-transform: uppercase;
  }

  .detail-badge-lightweight {
    font-size: 9px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    background: rgba(139, 148, 158, 0.15);
    color: #8b949e;
    text-transform: uppercase;
  }

  .detail-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .meta-oid {
    font-family: var(--font-mono);
    color: #f0883e;
  }

  .detail-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .detail-section {
    margin-bottom: 20px;
  }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .section-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
  }

  .message-card {
    font-size: 12px;
    color: var(--text-primary);
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .commit-summary {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .commit-body {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.5;
    white-space: pre-wrap;
    margin-bottom: 8px;
  }

  .commit-footer {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 11px;
    color: var(--text-secondary);
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }

  .commit-author {
    color: var(--text-primary);
  }

  .commit-sha {
    font-family: var(--font-mono);
    color: var(--accent-blue);
  }

  .stats-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .stats-row {
    display: flex;
    align-items: center;
    gap: 16px;
    font-size: 12px;
  }

  .stat-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .stat-label {
    color: var(--text-secondary);
  }

  .stat-value {
    color: var(--text-primary);
    font-weight: 600;
  }

  .stat-additions {
    color: #3fb950;
  }

  .stat-deletions {
    color: #f85149;
  }

  .stats-bar {
    display: flex;
    height: 8px;
    border-radius: 4px;
    overflow: hidden;
    background: var(--border);
  }

  .stats-bar-add {
    background: #3fb950;
  }

  .stats-bar-del {
    background: #f85149;
  }

  .parents-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .parent-oid {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--accent-blue);
    background: rgba(88, 166, 255, 0.08);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .parent-oid.clickable {
    border: none;
    cursor: pointer;
  }

  .parent-oid.clickable:hover {
    text-decoration: underline;
    background: rgba(88, 166, 255, 0.15);
  }

  .tag-diff-preview {
    border-top: 1px solid var(--border);
    max-height: 300px;
    overflow: auto;
    margin-bottom: 20px;
  }

  .parent-detail-panel {
    border-top: 1px solid var(--border);
    margin-top: 8px;
    margin-bottom: 20px;
  }

  .files-card {
    padding: 4px 0;
    max-height: 300px;
    overflow-y: auto;
  }

  .detail-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .action-btn {
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }

  .action-push:hover {
    background: rgba(63, 185, 80, 0.15);
    border-color: #3fb950;
    color: #3fb950;
  }

  .action-delete:hover {
    background: rgba(248, 81, 73, 0.15);
    border-color: #f85149;
    color: #f85149;
  }
</style>
