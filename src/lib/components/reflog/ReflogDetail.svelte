<script lang="ts">
  import type { CommitInfo, CommitFileChange, ReflogEntry } from "../../types";
  import CommitDetail from "../detail/CommitDetail.svelte";
  import * as m from "$lib/paraglide/messages";
  import * as api from "../../api/tauri";
  import { runMutation } from "../../api/runMutation";
  import { shortOid } from "../../utils/git";
  import { loadReflog } from "../../stores/reflog";
  import { openCreateBranchDialog } from "../../stores/createBranchDialog";
  import { Button, Skeleton } from "$lib/components/ui";

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
      <Button variant="neutral" size="sm" icon={"\uE728"} onclick={handleShowInGraph}>
        {m.reflog_show_in_graph()}
      </Button>
      <Button variant="primary" size="sm" icon={"\uE725"} onclick={handleCheckout}>
        {m.reflog_checkout()}
      </Button>
      <Button variant="primary" size="sm" icon={"\uE727"} onclick={handleCreateBranch}>
        {m.reflog_create_branch()}
      </Button>
      <div class="reset-wrapper">
        <Button variant="danger" size="sm" icon={"\uF0E2"} onclick={() => { showResetMenu = !showResetMenu; }}>
          {m.reflog_reset()}
        </Button>
        {#if showResetMenu}
          <div class="reset-menu">
            <button class="reset-option" onclick={() => handleReset("soft")}>{m.graph_reset_soft()}</button>
            <button class="reset-option" onclick={() => handleReset("mixed")}>{m.graph_reset_mixed()}</button>
            <button class="reset-option reset-option-danger" onclick={() => handleReset("hard")}>{m.graph_reset_hard()}</button>
          </div>
        {/if}
      </div>
      <Button variant="neutral" size="sm" icon={"\uF0C5"} description={entry.oid} onclick={handleCopySha}>
        {m.reflog_copy_sha()}
      </Button>
    </div>
    <CommitDetail
      {commit}
      {files}
      showNavigateToGraph={true}
      showOpenInEditor={true}
      onNavigateToGraph={onNavigateToGraph}
      onNavigate={onNavigate}
      {onFileClick}
    />
  {:else}
    <Skeleton variant="detail" rows={6} />
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
    box-shadow: var(--shadow-overlay);
  }

  .reset-option {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 6px 12px;
    font-size: var(--font-size-sm);
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
    font-size: var(--font-size-md);
  }

</style>
