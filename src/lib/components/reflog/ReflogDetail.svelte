<script lang="ts">
  import type { CommitInfo, CommitFileChange, ReflogEntry } from "../../types";
  import CommitDetail from "../detail/CommitDetail.svelte";
  import * as m from "$lib/paraglide/messages";
  import * as api from "../../api/tauri";

  let {
    entry,
    onNavigateToGraph,
    onNavigate,
  }: {
    entry: ReflogEntry;
    onNavigateToGraph?: (oid: string) => void;
    onNavigate?: (view: string) => void;
  } = $props();

  let commit = $state<CommitInfo | null>(null);
  let files = $state<CommitFileChange[]>([]);
  let loadError = $state<string | null>(null);

  $effect(() => {
    if (entry) {
      loadCommitDetail(entry.oid);
    }
  });

  async function loadCommitDetail(oid: string) {
    loadError = null;
    try {
      const [c, f] = await Promise.all([
        api.getCommitDetail(oid),
        api.getCommitFiles(oid),
      ]);
      commit = c;
      files = f;
    } catch (e) {
      loadError = String(e);
      commit = null;
      files = [];
    }
  }

  function handleShowInGraph() {
    onNavigateToGraph?.(entry.oid);
  }
</script>

<div class="reflog-detail">
  {#if loadError}
    <div class="detail-error">
      <p>{loadError}</p>
    </div>
  {:else if commit}
    <div class="detail-actions">
      <button class="action-btn" onclick={handleShowInGraph}>
        <span class="nf">{"\uE728"}</span>
        {m.reflog_show_in_graph()}
      </button>
    </div>
    <CommitDetail
      {commit}
      {files}
      showNavigateToGraph={true}
      onNavigateToGraph={onNavigateToGraph}
      onNavigate={onNavigate}
    />
  {:else}
    <div class="detail-loading">
      <div class="spinner"></div>
    </div>
  {/if}
</div>

<style>
  .reflog-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .detail-actions {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    font-size: 11px;
    background: none;
    border: 1px solid var(--border);
    color: var(--accent-blue);
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.1s;
  }

  .action-btn:hover {
    background: rgba(88, 166, 255, 0.1);
  }

  .action-btn .nf {
    font-family: var(--font-icons);
    font-size: 12px;
  }

  .detail-error {
    padding: 24px;
    text-align: center;
    color: var(--accent-orange);
    font-size: 13px;
  }

  .detail-loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
