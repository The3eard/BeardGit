<!--
  BranchCleanupDialog.svelte — bulk cleanup of stale local branches (spec 11).

  Two grouped sections: "Gone" (upstream deleted; pre-checked) and "Merged
  into <target>" (unchecked by default). Each row shows the last-commit date
  and, as a second-thought signal, how many commits it holds beyond the
  target. Gone branches that aren't fully merged need a force delete, gated
  behind an explicit acknowledgment. Deletes route through `doDeleteBranches`
  (one command → one refresh); partial failures surface in-dialog.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";
  import { Checkbox } from "$lib/components/ui";
  import { listBranchCleanupCandidates } from "$lib/api/tauri";
  import { doDeleteBranches } from "$lib/stores/branches";
  import { formatRelativeTimeUnix } from "$lib/utils/time";
  import { shortOid } from "$lib/utils/git";
  import type { BranchCleanupCandidate, BranchCleanupList, BatchDeleteResult } from "$lib/types";
  import {
    needsForce,
    initialSelection,
    selectedForceNames,
    canDelete,
  } from "./branch-cleanup";

  let { onClose }: { onClose: () => void } = $props();

  let list = $state<BranchCleanupList | null>(null);
  let selected = $state<Set<string>>(new Set());
  let loading = $state(true);
  let deleting = $state(false);
  let forceAck = $state(false);
  let result = $state<BatchDeleteResult | null>(null);

  let forceNeeded = $derived(
    list ? selectedForceNames(list, selected).length > 0 : false,
  );
  let deletable = $derived(list ? canDelete(list, selected, forceAck) : false);
  let isEmpty = $derived(!!list && list.gone.length === 0 && list.merged.length === 0);

  async function load() {
    loading = true;
    try {
      const next = await listBranchCleanupCandidates();
      list = next;
      selected = initialSelection(next);
    } catch {
      list = { target: "", gone: [], merged: [] };
      selected = new Set();
    }
    loading = false;
  }

  onMount(load);

  function toggle(name: string) {
    const next = new Set(selected);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    selected = next;
  }

  async function handleDelete() {
    if (!list || !deletable || deleting) return;
    deleting = true;
    const names = [...selected];
    const force = selectedForceNames(list, selected);
    try {
      const r = await doDeleteBranches(names, force);
      result = r;
      if (r.failed.length === 0) {
        onClose();
        return;
      }
      // Partial failure: reload so deleted rows drop, keep the dialog open
      // with the failure panel visible.
      forceAck = false;
      await load();
    } catch {
      // doDeleteBranches → runMutation already surfaced a failure toast.
    } finally {
      deleting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div
  class="backdrop"
  onclick={onClose}
  onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
  role="button"
  tabindex="-1"
></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={m.branch_cleanup_title()} data-testid="branch-cleanup-dialog">
  <h3 class="dialog-title">{m.branch_cleanup_title()}</h3>

  <div class="candidate-container">
    {#if loading}
      <div class="empty-state">{m.branch_cleanup_loading()}</div>
    {:else if isEmpty}
      <div class="empty-state">{m.branch_cleanup_empty()}</div>
    {:else if list}
      {#snippet row(c: BranchCleanupCandidate)}
        <div class="candidate" class:needs-force={needsForce(c)}>
          <Checkbox
            id="cleanup-{c.name}"
            testid="cleanup-row-{c.name}"
            checked={selected.has(c.name)}
            onchange={() => toggle(c.name)}
          />
          <label class="candidate-name" for="cleanup-{c.name}">{c.name}</label>
          {#if needsForce(c)}
            <span class="not-merged-pill" title={m.branch_cleanup_not_merged_tooltip()}>
              {m.branch_cleanup_not_merged()}
            </span>
          {/if}
          {#if c.ahead > 0}
            <span class="ahead-hint" title={m.branch_cleanup_ahead_tooltip()}>
              {m.branch_cleanup_ahead({ count: String(c.ahead) })}
            </span>
          {/if}
          <span class="candidate-date">{formatRelativeTimeUnix(c.last_commit_time)}</span>
          <span class="candidate-oid">{shortOid(c.tip_oid)}</span>
        </div>
      {/snippet}

      {#if list.gone.length > 0}
        <div class="section-label">
          {m.branch_cleanup_gone_section()} <span class="section-count">{list.gone.length}</span>
        </div>
        {#each list.gone as c (c.name)}{@render row(c)}{/each}
      {/if}

      {#if list.merged.length > 0}
        <div class="section-label">
          {m.branch_cleanup_merged_section({ target: list.target })}
          <span class="section-count">{list.merged.length}</span>
        </div>
        {#each list.merged as c (c.name)}{@render row(c)}{/each}
      {/if}
    {/if}
  </div>

  {#if result && result.failed.length > 0}
    <div class="failure-panel" data-testid="cleanup-failures">
      <div class="failure-heading">{m.branch_cleanup_failed_heading()}</div>
      {#each result.failed as f (f.name)}
        <div class="failure-row"><span class="failure-name">{f.name}</span> — {f.reason}</div>
      {/each}
    </div>
  {/if}

  {#if forceNeeded}
    <span class="force-ack">
      <Checkbox
        id="cleanup-force-ack"
        testid="cleanup-force-ack"
        checked={forceAck}
        onchange={(e) => { forceAck = (e.target as HTMLInputElement).checked; }}
      />
      <label for="cleanup-force-ack">{m.branch_cleanup_force_ack()}</label>
    </span>
  {/if}

  <div class="reflog-note">{m.branch_cleanup_reflog_note()}</div>

  <div class="dialog-actions">
    <Button variant="neutral" onclick={onClose}>{m.confirm_cancel()}</Button>
    <Button
      variant="danger"
      testid="cleanup-delete-btn"
      disabled={!deletable || deleting}
      onclick={handleDelete}
    >
      {m.branch_cleanup_delete({ count: String(selected.size) })}
    </Button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions */

  .dialog {
    min-width: 440px;
    max-width: 560px;
    max-height: 72vh;
    display: flex;
    flex-direction: column;
  }

  .dialog-title {
    margin-bottom: 12px;
  }

  .candidate-container {
    flex: 1;
    overflow-y: auto;
    min-height: 120px;
    max-height: 340px;
  }

  .section-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--font-size-2xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    padding: 10px 0 4px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }

  .section-count {
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    padding: 1px 6px;
    border-radius: 10px;
    text-transform: none;
  }

  .candidate {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 4px;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    border-radius: 4px;
  }

  .candidate:hover {
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
  }

  .candidate-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
    min-width: 0;
  }

  .not-merged-pill {
    font-size: var(--font-size-2xs);
    font-weight: 500;
    padding: 1px 6px;
    border-radius: 8px;
    line-height: 1.4;
    flex-shrink: 0;
    background: var(--overlay-accent-red);
    color: var(--accent-red);
  }

  .ahead-hint {
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .candidate-date {
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .candidate-oid {
    font-size: var(--font-size-2xs);
    font-family: var(--font-mono);
    color: var(--text-secondary);
    opacity: 0.7;
    flex-shrink: 0;
  }

  .failure-panel {
    margin-top: 8px;
    padding: 8px 10px;
    background: var(--overlay-accent-red);
    border-radius: 6px;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    max-height: 120px;
    overflow-y: auto;
  }

  .failure-heading {
    font-weight: 600;
    color: var(--accent-red);
    margin-bottom: 4px;
  }

  .failure-row {
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
    line-height: 1.5;
    word-break: break-word;
  }

  .failure-name {
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .force-ack {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    font-size: var(--font-size-sm);
    color: var(--accent-red);
    user-select: none;
  }

  .force-ack label {
    cursor: pointer;
  }

  .reflog-note {
    padding: 10px 0 4px;
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .dialog-actions {
    padding-top: 8px;
  }
</style>
