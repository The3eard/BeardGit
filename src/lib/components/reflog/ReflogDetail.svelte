<script lang="ts">
  import type { CommitInfo, CommitFileChange, ReflogEntry } from "../../types";
  import CommitDetail from "../detail/CommitDetail.svelte";
  import * as m from "$lib/paraglide/messages";
  import * as api from "../../api/tauri";
  import { runMutation } from "../../api/runMutation";
  import { shortOid } from "../../utils/git";
  import { loadReflog } from "../../stores/reflog";
  import { openCreateBranchDialog } from "../../stores/createBranchDialog";

  let {
    entry,
    onNavigateToGraph,
    onNavigate,
    onFileClick,
  }: {
    entry: ReflogEntry;
    onNavigateToGraph?: (oid: string) => void;
    onNavigate?: (view: string) => void;
    onFileClick?: (path: string) => void;
  } = $props();

  let commit = $state<CommitInfo | null>(null);
  let files = $state<CommitFileChange[]>([]);
  let loadError = $state<string | null>(null);
  let showResetMenu = $state(false);

  $effect(() => {
    if (entry) {
      loadCommitDetail(entry.oid);
      showResetMenu = false;
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

  async function handleCheckout() {
    try {
      await runMutation({
        kind: "checkout_detached",
        invoke: () => api.checkoutDetached(entry.oid),
        successToast: () => `Checked out ${shortOid(entry.oid)} (detached)`,
        failureToastPrefix: "Checkout failed",
      });
      await loadReflog();
    } catch {
      // runMutation already surfaced the toast.
    }
  }

  function handleCreateBranch() {
    openCreateBranchDialog({ kind: "commit", oid: entry.oid });
  }

  async function handleReset(mode: string) {
    showResetMenu = false;
    const labels: Record<string, string> = { soft: "Soft", mixed: "Mixed", hard: "Hard" };
    if (!confirm(`${labels[mode]} reset to ${shortOid(entry.oid)}?`)) return;
    try {
      await runMutation({
        kind: `reset_${mode}`,
        invoke: () => api.resetToCommit(entry.oid, mode),
        successToast: () =>
          `Reset (${mode}) to ${shortOid(entry.oid)}`,
        failureToastPrefix: "Reset failed",
      });
      await loadReflog();
    } catch {
      // runMutation already surfaced the toast.
    }
  }

  function handleCopySha() {
    navigator.clipboard.writeText(entry.oid);
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
      <button class="action-btn" onclick={handleCheckout}>
        <span class="nf">{"\uE725"}</span>
        {m.reflog_checkout()}
      </button>
      <button class="action-btn" onclick={handleCreateBranch}>
        <span class="nf">{"\uE727"}</span>
        {m.reflog_create_branch()}
      </button>
      <div class="reset-wrapper">
        <button class="action-btn action-btn-danger" onclick={() => { showResetMenu = !showResetMenu; }}>
          <span class="nf">{"\uF0E2"}</span>
          {m.reflog_reset()}
        </button>
        {#if showResetMenu}
          <div class="reset-menu">
            <button class="reset-option" onclick={() => handleReset("soft")}>{m.graph_reset_soft()}</button>
            <button class="reset-option" onclick={() => handleReset("mixed")}>{m.graph_reset_mixed()}</button>
            <button class="reset-option reset-option-danger" onclick={() => handleReset("hard")}>{m.graph_reset_hard()}</button>
          </div>
        {/if}
      </div>
      <button class="action-btn action-btn-subtle" onclick={handleCopySha} title={entry.oid}>
        <span class="nf">{"\uF0C5"}</span>
        {m.reflog_copy_sha()}
      </button>
    </div>
    <CommitDetail
      {commit}
      {files}
      showNavigateToGraph={true}
      onNavigateToGraph={onNavigateToGraph}
      onNavigate={onNavigate}
      {onFileClick}
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
    gap: 6px;
    flex-shrink: 0;
    flex-wrap: wrap;
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
    background: var(--overlay-accent-blue);
  }

  .action-btn .nf {
    font-family: var(--font-icons);
    font-size: 12px;
  }

  .action-btn-danger {
    color: var(--accent-orange);
  }

  .action-btn-danger:hover {
    background: color-mix(in srgb, var(--accent-orange) 10%, transparent);
  }

  .action-btn-subtle {
    color: var(--text-secondary);
  }

  .action-btn-subtle:hover {
    color: var(--text-primary);
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }

  .reset-wrapper {
    position: relative;
  }

  .reset-menu {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 0;
    z-index: 10;
    min-width: 200px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3); /* beardgit:allow-hex: shadow neutral always-dark */
  }

  .reset-option {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
    transition: background 0.1s;
  }

  .reset-option:hover {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }

  .reset-option-danger {
    color: var(--accent-red);
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
