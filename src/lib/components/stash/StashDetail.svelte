<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import DiffViewer from "../diff/DiffViewer.svelte";
  import { stashes, selectedStashIndex, selectedStashDiff, doStashApplyFile } from "../../stores/stashes";
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
        <span>{new Date(selectedEntry.timestamp * 1000).toLocaleString()}</span>
        <span class="stash-oid">{selectedEntry.oid.slice(0, 8)}</span>
      </div>
    </div>
    <div class="stash-detail-diffs">
      {#each $selectedStashDiff as diff (diff.path)}
        <DiffViewer {diff} onApplyFile={(path) => {
          if ($selectedStashIndex !== null) doStashApplyFile($selectedStashIndex, path);
        }} />
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

</style>
