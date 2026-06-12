<script lang="ts">
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import { Button } from "$lib/components/ui";
  import { formatRelativeTimeUnix } from "../../utils/time";
  import { shortOid } from "../../utils/git";
  import {
    selectedBranchName,
    selectedBranchInfo,
    selectedBranchCommits,
    loadingDetail,
    doCheckout,
    doDeleteBranch,
    doMergeBranch,
    branchSelectedCommit,
    selectBranchCommit,
    closeBranchCommitDetail,
  } from "../../stores/branches";

  let confirmDelete = $state(false);
  let forceDelete = $state(false);

  $effect(() => {
    $selectedBranchName;
    closeBranchCommitDetail();
  });
</script>

<div class="branch-detail">
  {#if $loadingDetail}
    <div class="detail-loading">
      <div class="spinner"></div>
      <span>Loading commits…</span>
    </div>
  {:else if $selectedBranchInfo}
    <!-- Header -->
    <div class="detail-header">
      <div class="detail-title-row">
        <span class="detail-title">{$selectedBranchInfo.name}</span>
        {#if $selectedBranchInfo.is_head}
          <span class="badge badge-head">HEAD</span>
        {/if}
        {#if $selectedBranchInfo.is_remote}
          <span class="badge badge-remote">remote</span>
        {/if}
      </div>
      <div class="detail-meta">
        <span class="meta-oid">{$selectedBranchInfo.oid.slice(0, 8)}</span>
      </div>
    </div>

    <!-- Commit list -->
    <div class="branch-commits-panel">
      {#if $selectedBranchCommits.length > 0}
        <div class="detail-section">
          <div class="section-label">RECENT COMMITS</div>
          <div class="commits-list">
            {#each $selectedBranchCommits as commit (commit.oid)}
              <div
                class="commit-row"
                class:selected={$branchSelectedCommit?.oid === commit.oid}
                onclick={() => selectBranchCommit(commit.oid)}
                role="button"
                tabindex="0"
                onkeydown={(e) => { if (e.key === "Enter") selectBranchCommit(commit.oid); }}
              >
                <div class="commit-summary">{commit.summary}</div>
                <div class="commit-meta">
                  <span class="commit-author">{commit.author}</span>
                  <span class="commit-sep">·</span>
                  <span class="commit-time">{formatRelativeTimeUnix(commit.timestamp)}</span>
                  <span class="commit-oid">{shortOid(commit.oid)}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="no-commits">No commits found</div>
      {/if}
    </div>

    <!-- Actions footer -->
    {#if !$selectedBranchInfo.is_remote}
      <div class="detail-actions">
        {#if !$selectedBranchInfo.is_head}
          <Button
            variant="primary"
            size="sm"
            onclick={() => doCheckout($selectedBranchInfo!.name)}
          >
            Checkout
          </Button>
          <Button
            variant="success"
            size="sm"
            onclick={() => doMergeBranch($selectedBranchInfo!.name)}
          >
            Merge into current
          </Button>
          <Button variant="danger" size="sm" onclick={() => { forceDelete = false; confirmDelete = true; }}>
            Delete
          </Button>
        {/if}
      </div>
    {/if}
  {:else if !$selectedBranchName}
    <EmptyState fill icon={"\uE725"} title="Select a branch to view details" />
  {/if}

  {#if confirmDelete && $selectedBranchInfo}
    <ConfirmDialog
      title="Delete Branch"
      detail={`${$selectedBranchInfo.name}\n${$selectedBranchInfo.oid.slice(0, 8)}`}
      message={`Are you sure you want to delete branch "${$selectedBranchInfo.name}"? This action cannot be undone.`}
      confirmLabel={forceDelete ? "Force Delete" : "Delete"}
      destructive={true}
      checkboxLabel="Force delete (allow unmerged commits)"
      bind:checkboxChecked={forceDelete}
      onConfirm={async () => {
        const target = $selectedBranchInfo!.name;
        const force = forceDelete;
        confirmDelete = false;
        try {
          await doDeleteBranch(target, force);
        } catch {
          // `runMutation` already surfaced a sticky failure toast.
        }
      }}
      onCancel={() => (confirmDelete = false)}
    />
  {/if}
</div>

<style>
  .branch-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .detail-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .detail-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }

  .detail-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    word-break: break-all;
  }

  .badge {
    font-size: 9px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    flex-shrink: 0;
  }

  .badge-head {
    background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
    color: var(--accent-primary);
  }

  .badge-remote {
    background: color-mix(in srgb, var(--accent-purple) 15%, transparent);
    color: var(--accent-purple);
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
    color: var(--accent-primary);
  }

  .branch-commits-panel {
    flex: 1;
    min-width: 0;
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

  .commits-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .commit-row {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .commit-row:last-child {
    border-bottom: none;
  }

  .commit-row:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
    cursor: pointer;
  }

  .commit-row.selected {
    background: var(--selection);
  }

  .commit-summary {
    font-size: 12px;
    color: var(--text-primary);
    margin-bottom: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .commit-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .commit-author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 160px;
  }

  .commit-sep {
    opacity: 0.5;
  }

  .commit-time {
    flex-shrink: 0;
  }

  .commit-oid {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--accent-primary);
    margin-left: auto;
    flex-shrink: 0;
  }

  .no-commits {
    padding: 24px;
    text-align: center;
    font-size: 12px;
    color: var(--text-secondary);
    font-style: italic;
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

</style>
