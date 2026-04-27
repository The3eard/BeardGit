<!--
  MrPrDetail — detail panel for a selected merge request / pull request.

  Shows summary, description, labels, changed files, comments, and
  action buttons for merge/close/approve/request-changes/comment.
-->
<script lang="ts">
  import {
    mrPrDetail,
    mrPrDetailLoading,
    mrPrDetailError,
    mrPrDiffFiles,
    mrPrDiffLoading,
    mrPrDiffError,
    selectedMrPrNumber,
    loadMrPrDetail,
    mergeMrPr,
    closeMrPr,
    approveMrPr,
    requestChangesMrPr,
    addMrPrComment,
    addMrPrLabels,
    removeMrPrLabels,
    addMrPrReviewers,
    removeMrPrReviewers,
    markMrPrReady,
    markMrPrDraft,
    reopenMrPr,
    resolveDiscussion,
    unresolveDiscussion,
    checkoutMrPrLocally,
    repoLabels,
    repoLabelsLoading,
    loadRepoLabels,
    selectedPrFilePath,
  } from "../../stores/mr-pr";
  import ForgeDetailShell from "../common/ForgeDetailShell.svelte";
  import { Button, IconButton } from "$lib/components/ui";
  import { activeProvider } from "../../stores/provider";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import Xrefs from "../common/Xrefs.svelte";
  import { renderMarkdown } from "../../utils/markdown";
  import PillRow from "./PillRow.svelte";
  import LabelPicker from "../common/LabelPicker.svelte";
  import ReviewerPicker from "./ReviewerPicker.svelte";
  import PathTree from "../common/PathTree.svelte";
  import type { CheckoutResult } from "../../types";

  interface Props {
    /** Called when the user clicks a changed-file row. */
    onFileClick?: (path: string) => void;
  }
  let { onFileClick }: Props = $props();

  let isGitHub = $derived($activeProvider?.kind === "github");
  let isGitLab = $derived($activeProvider?.kind === "gitlab");
  let selectMessage = $derived(isGitHub ? m.mrpr_select_github() : m.mrpr_select());

  // Merge/close confirmation state
  let showMergeConfirm = $state(false);
  let mergeStrategy = $state("merge");
  let showMergeDropdown = $state(false);
  let showCloseConfirm = $state(false);

  // Phase 8.2 — enhancement state
  let showLabelPicker = $state(false);
  let showReviewerPicker = $state(false);
  let showReopenConfirm = $state(false);
  let showCheckoutConfirm = $state(false);
  let checkoutTaskId = $state<number | null>(null);
  let checkoutSuccess = $state<CheckoutResult | null>(null);
  let unlistenCheckout: (() => void) | null = null;

  // Load repo labels when the picker is opened for the first time.
  $effect(() => {
    if (showLabelPicker && $repoLabels.length === 0) {
      loadRepoLabels();
    }
  });

  onMount(async () => {
    unlistenCheckout = await listen<CheckoutResult>("mr-pr-checked-out", (event) => {
      checkoutSuccess = event.payload;
      checkoutTaskId = null;
      // Auto-dismiss after 4 seconds.
      setTimeout(() => {
        checkoutSuccess = null;
      }, 4000);
    });
  });

  onDestroy(() => {
    unlistenCheckout?.();
  });

  // Close merge dropdown on outside click
  function handleWindowClick(e: MouseEvent) {
    if (!showMergeDropdown) return;
    const target = e.target as HTMLElement;
    if (!target.closest(".merge-group")) {
      showMergeDropdown = false;
    }
  }
  let actionError = $state("");

  // Comment input state
  let commentBody = $state("");
  let commentSubmitting = $state(false);

  async function handleMerge() {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      await mergeMrPr(detail.summary.number, mergeStrategy);
    } catch (e) {
      actionError = m.mrpr_merge_failed({ error: String(e) });
    }
    showMergeConfirm = false;
  }

  async function handleClose() {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      await closeMrPr(detail.summary.number);
    } catch (e) {
      actionError = m.mrpr_close_failed({ error: String(e) });
    }
    showCloseConfirm = false;
  }

  async function handleApprove() {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      await approveMrPr(detail.summary.number);
    } catch (e) {
      actionError = String(e);
    }
  }

  async function handleRequestChanges() {
    const detail = $mrPrDetail;
    if (!detail || !commentBody.trim()) return;
    try {
      actionError = "";
      await requestChangesMrPr(detail.summary.number, commentBody.trim());
      commentBody = "";
    } catch (e) {
      actionError = String(e);
    }
  }

  async function handleAddComment() {
    const detail = $mrPrDetail;
    if (!detail || !commentBody.trim()) return;
    commentSubmitting = true;
    try {
      actionError = "";
      await addMrPrComment(detail.summary.number, commentBody.trim());
      commentBody = "";
    } catch (e) {
      actionError = String(e);
    } finally {
      commentSubmitting = false;
    }
  }

  async function handleLabelApply(added: string[], removed: string[]) {
    const detail = $mrPrDetail;
    if (!detail) {
      showLabelPicker = false;
      return;
    }
    try {
      actionError = "";
      if (added.length > 0) await addMrPrLabels(detail.summary.number, added);
      if (removed.length > 0) await removeMrPrLabels(detail.summary.number, removed);
    } catch (e) {
      actionError = String(e);
    }
    showLabelPicker = false;
  }

  async function handleRemoveLabel(label: string) {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      await removeMrPrLabels(detail.summary.number, [label]);
    } catch (e) {
      actionError = String(e);
    }
  }

  async function handleReviewerApply(added: string[]) {
    const detail = $mrPrDetail;
    if (!detail || added.length === 0) {
      showReviewerPicker = false;
      return;
    }
    try {
      actionError = "";
      await addMrPrReviewers(detail.summary.number, added);
    } catch (e) {
      actionError = String(e);
    }
    showReviewerPicker = false;
  }

  async function handleRemoveReviewer(reviewer: string) {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      await removeMrPrReviewers(detail.summary.number, [reviewer]);
    } catch (e) {
      actionError = String(e);
    }
  }

  async function handleDraftToggle() {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      if (detail.summary.draft) {
        await markMrPrReady(detail.summary.number);
      } else {
        await markMrPrDraft(detail.summary.number);
      }
    } catch (e) {
      actionError = String(e);
    }
  }

  async function handleReopen() {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      await reopenMrPr(detail.summary.number);
    } catch (e) {
      actionError = String(e);
    }
    showReopenConfirm = false;
  }

  async function handleToggleResolve(discussionId: string, resolved: boolean) {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      if (resolved) {
        await unresolveDiscussion(detail.summary.number, discussionId);
      } else {
        await resolveDiscussion(detail.summary.number, discussionId);
      }
    } catch (e) {
      actionError = String(e);
    }
  }

  async function handleCheckout() {
    const detail = $mrPrDetail;
    if (!detail) return;
    try {
      actionError = "";
      checkoutTaskId = await checkoutMrPrLocally(detail.summary.number);
    } catch (e) {
      actionError = String(e);
    }
    showCheckoutConfirm = false;
  }
</script>

<svelte:window onclick={handleWindowClick} />

<ForgeDetailShell
  loading={$mrPrDetailLoading}
  error={$mrPrDetailError}
  isEmpty={!$mrPrDetail && !$mrPrDetailLoading && !$mrPrDetailError}
  emptyMessage={selectMessage}
  onRetry={() => {
    const n = $selectedMrPrNumber;
    if (n !== null) void loadMrPrDetail(n);
  }}
>
  {#snippet content()}
    {#if $mrPrDetail}
      {@const detail = $mrPrDetail}
      <div class="mrpr-detail">
    <div class="detail-header">
      <h3 class="detail-title">
        <span class="detail-number">#{detail.summary.number}</span>
        {detail.summary.title}
      </h3>
      <IconButton
        tone="default"
        icon={""}
        description={m.mrpr_open_browser()}
        onclick={() => openUrl(detail.summary.url)}
      />
    </div>

    <div class="detail-meta">
      <span class="branch-info">
        {m.mrpr_branch_arrow({
          source: detail.summary.source_branch,
          target: detail.summary.target_branch,
        })}
      </span>
      <span class="author">{detail.summary.author}</span>

      {#if detail.mergeable === true}
        <span class="merge-status mergeable">{m.mrpr_mergeable()}</span>
      {:else if detail.mergeable === false}
        <span class="merge-status not-mergeable">{m.mrpr_not_mergeable()}</span>
      {/if}

      <span class="review-badge" class:approved={detail.review_status === "approved"} class:changes-requested={detail.review_status === "changes_requested"}>
        {#if detail.review_status === "approved"}
          {m.mrpr_status_approved()}
        {:else if detail.review_status === "changes_requested"}
          {m.mrpr_status_changes_requested()}
        {:else}
          {m.mrpr_status_pending()}
        {/if}
      </span>
    </div>

    <!-- Action buttons for open, closed, merged MR/PRs -->
    {#if detail.summary.state === "open"}
      <div class="detail-actions">
        <Button variant="success" size="sm" onclick={handleApprove}>{m.mrpr_approve()}</Button>
        <div class="merge-group">
          <Button variant="success" onclick={() => { showMergeConfirm = true; }}>
            {mergeStrategy === "squash" ? m.mrpr_merge_squash() : mergeStrategy === "rebase" ? m.mrpr_merge_rebase() : m.mrpr_merge()}
          </Button>
          <button class="merge-dropdown-trigger" onclick={() => showMergeDropdown = !showMergeDropdown}>{"\uF078"}</button>
          {#if showMergeDropdown}
            <div class="merge-dropdown-menu">
              <button class:active={mergeStrategy === "merge"} onclick={() => { mergeStrategy = "merge"; showMergeDropdown = false; }}>{m.mrpr_merge()}</button>
              <button class:active={mergeStrategy === "squash"} onclick={() => { mergeStrategy = "squash"; showMergeDropdown = false; }}>{m.mrpr_merge_squash()}</button>
              <button class:active={mergeStrategy === "rebase"} onclick={() => { mergeStrategy = "rebase"; showMergeDropdown = false; }}>{m.mrpr_merge_rebase()}</button>
            </div>
          {/if}
        </div>
        <button class="draft-toggle-btn" onclick={handleDraftToggle}>
          {detail.summary.draft ? m.mrpr_mark_ready() : m.mrpr_convert_to_draft()}
        </button>
        <Button variant="primary" size="sm" onclick={() => { showCheckoutConfirm = true; }} disabled={checkoutTaskId !== null}>
          {checkoutTaskId !== null ? m.mrpr_checkout_running() : m.mrpr_checkout_locally()}
        </Button>
        <div class="push-right">
          <Button variant="neutral" size="sm" onclick={() => { showCloseConfirm = true; }}>{m.mrpr_close()}</Button>
        </div>
      </div>
    {:else if detail.summary.state === "closed"}
      <div class="detail-actions">
        <Button variant="primary" size="sm" onclick={() => { showCheckoutConfirm = true; }} disabled={checkoutTaskId !== null}>
          {checkoutTaskId !== null ? m.mrpr_checkout_running() : m.mrpr_checkout_locally()}
        </Button>
        <Button variant="primary" size="sm" onclick={() => { showReopenConfirm = true; }}>{m.mrpr_reopen()}</Button>
      </div>
    {:else if detail.summary.state === "merged"}
      <div class="detail-actions">
        <Button variant="primary" size="sm" onclick={() => { showCheckoutConfirm = true; }} disabled={checkoutTaskId !== null}>
          {checkoutTaskId !== null ? m.mrpr_checkout_running() : m.mrpr_checkout_locally()}
        </Button>
      </div>
    {/if}

    {#if actionError}
      <p class="error-msg">{actionError}</p>
    {/if}

    {#if detail.body}
      <div class="section">
        <h4 class="section-title">{m.mrpr_description()}</h4>
        <div class="description-body">
          <Xrefs text={detail.body} render={(t) => renderMarkdown(t)} />
        </div>
      </div>
    {/if}

    <div class="section">
      <h4 class="section-title">{m.mrpr_labels()}</h4>
      <PillRow
        items={detail.summary.labels}
        onRemove={handleRemoveLabel}
        onAddClick={() => { showLabelPicker = true; }}
        emptyLabel={m.mrpr_no_labels()}
        pillClass="label-pill"
        removeAriaLabel={(item) => m.mrpr_remove_label_aria({ item })}
        addAriaLabel={m.mrpr_add_aria()}
      />
    </div>

    <div class="section">
      <h4 class="section-title">{m.mrpr_reviewers()}</h4>
      <PillRow
        items={detail.summary.reviewers}
        onRemove={handleRemoveReviewer}
        onAddClick={() => { showReviewerPicker = true; }}
        emptyLabel={m.mrpr_no_reviewers()}
        pillClass="reviewer-pill"
        removeAriaLabel={(item) => m.mrpr_remove_reviewer_aria({ item })}
        addAriaLabel={m.mrpr_add_aria()}
      />
    </div>

    <div class="section">
      <h4 class="section-title">{m.mrpr_changed_files({ count: $mrPrDiffFiles.length.toString() })}</h4>
      {#if $mrPrDiffLoading}
        <p class="empty-section" data-testid="mrpr-diff-loading">{m.mrpr_loading()}</p>
      {:else if $mrPrDiffError}
        <p class="diff-error" role="alert" data-testid="mrpr-diff-error">
          {m.mrpr_diff_failed({ error: $mrPrDiffError })}
        </p>
      {:else if $mrPrDiffFiles.length === 0}
        <p class="empty-section">{m.mrpr_empty_no_changes()}</p>
      {:else}
        {#if $mrPrDiffFiles.length > 20}
          <PathTree
            items={$mrPrDiffFiles.map((f) => ({ path: f.path, meta: f }))}
            autoFlattenThreshold={20}
            selectedPath={$selectedPrFilePath}
            onSelect={(p) => onFileClick?.(p)}
            aggregateLabel={(descs) => {
              const add = descs.reduce((a, d) => a + (d.meta?.additions ?? 0), 0);
              const del = descs.reduce((a, d) => a + (d.meta?.deletions ?? 0), 0);
              return `+${add} −${del} · ${descs.length}`;
            }}
          />
        {:else}
          <div class="file-list">
            {#each $mrPrDiffFiles as file (file.path)}
              <button
                class="file-row"
                class:selected={$selectedPrFilePath === file.path}
                onclick={() => onFileClick?.(file.path)}
                aria-label={file.path}
              >
                <span
                  class="file-status"
                  class:added={file.status === "added"}
                  class:deleted={file.status === "deleted"}
                >
                  {file.status === "added"
                    ? "A"
                    : file.status === "deleted"
                      ? "D"
                      : file.status === "renamed"
                        ? "R"
                        : "M"}
                </span>
                <span class="file-path">{file.path}</span>
                <span class="file-adds">+{file.additions}</span>
                <span class="file-dels">-{file.deletions}</span>
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    </div>

    {#if detail.comments.length > 0}
      <div class="section">
        <h4 class="section-title">
          {m.mrpr_comments({ count: detail.comments.length.toString() })}
        </h4>
        <div class="comment-list">
          {#each detail.comments as comment, i (comment.id !== 0 ? `id-${comment.id}` : `idx-${i}`)}
            <div class="comment" class:resolved={comment.resolved === true}>
              <div class="comment-header">
                <span class="comment-author">{comment.author}</span>
                <span class="comment-date">{new Date(comment.created_at).toLocaleString()}</span>
                {#if comment.path}
                  <span class="comment-file">{comment.path}{comment.line ? `:${comment.line}` : ""}</span>
                {/if}
                {#if isGitLab && comment.resolvable && comment.discussion_id}
                  <button
                    class="resolve-btn"
                    class:is-resolved={comment.resolved === true}
                    onclick={() => handleToggleResolve(comment.discussion_id!, comment.resolved === true)}
                  >
                    {comment.resolved ? m.mrpr_unresolve() : m.mrpr_resolve()}
                  </button>
                {/if}
              </div>
              <div class="comment-body">
                <Xrefs text={comment.body} render={(t) => renderMarkdown(t)} />
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Comment input area -->
    {#if detail.summary.state === "open"}
      <div class="section comment-input-section">
        <textarea
          class="comment-textarea"
          placeholder={m.mrpr_comment_placeholder()}
          bind:value={commentBody}
          rows="3"
        ></textarea>
        <div class="comment-actions">
          <button
            class="btn-comment"
            disabled={!commentBody.trim() || commentSubmitting}
            onclick={handleAddComment}
          >
            {m.mrpr_add_comment()}
          </button>
          <button
            class="btn-request-changes"
            disabled={!commentBody.trim()}
            onclick={handleRequestChanges}
          >
            {m.mrpr_request_changes()}
          </button>
        </div>
      </div>
    {/if}
  </div>
    {/if}
  {/snippet}
</ForgeDetailShell>

{#if showMergeConfirm && $mrPrDetail}
  <ConfirmDialog
    title={m.mrpr_merge()}
    message={m.mrpr_merge_confirm({ target: $mrPrDetail.summary.target_branch })}
    confirmLabel={m.mrpr_merge()}
    onConfirm={handleMerge}
    onCancel={() => { showMergeConfirm = false; }}
  />
{/if}

{#if showCloseConfirm && $mrPrDetail}
  <ConfirmDialog
    title={m.mrpr_close()}
    message={isGitHub ? m.mrpr_close_confirm_github() : m.mrpr_close_confirm()}
    confirmLabel={m.mrpr_close()}
    destructive
    onConfirm={handleClose}
    onCancel={() => { showCloseConfirm = false; }}
  />
{/if}

{#if showReopenConfirm && $mrPrDetail}
  <ConfirmDialog
    title={m.mrpr_reopen()}
    message={m.mrpr_reopen_confirm()}
    confirmLabel={m.mrpr_reopen()}
    onConfirm={handleReopen}
    onCancel={() => { showReopenConfirm = false; }}
  />
{/if}

{#if showCheckoutConfirm && $mrPrDetail}
  <ConfirmDialog
    title={m.mrpr_checkout_locally()}
    message={m.mrpr_checkout_confirm({ branch: $mrPrDetail.summary.source_branch })}
    confirmLabel={m.mrpr_checkout_locally()}
    onConfirm={handleCheckout}
    onCancel={() => { showCheckoutConfirm = false; }}
  />
{/if}

{#if showLabelPicker && $mrPrDetail}
  <LabelPicker
    labels={$repoLabels}
    loading={$repoLabelsLoading}
    current={$mrPrDetail.summary.labels.map((l) => l.name)}
    onApply={handleLabelApply}
    onCancel={() => { showLabelPicker = false; }}
  />
{/if}

{#if showReviewerPicker && $mrPrDetail}
  <ReviewerPicker
    current={$mrPrDetail.summary.reviewers}
    onApply={handleReviewerApply}
    onCancel={() => { showReviewerPicker = false; }}
  />
{/if}

{#if checkoutSuccess}
  <div class="checkout-toast">
    {m.mrpr_checkout_success({ branch: checkoutSuccess.branch_name })}
    {#if checkoutSuccess.remote_added}
      <br />
      <span class="toast-sub">{m.mrpr_checkout_remote_added({ remote: checkoutSuccess.remote_added })}</span>
    {/if}
  </div>
{/if}

<style>
  /*
   * `.detail-empty` used to live here for the legacy loading/empty
   * states — those are now owned by `ForgeDetailShell`, so the class
   * was removed to avoid "unused selector" noise.
   */
  .mrpr-detail {
    padding: 16px;
    overflow-y: auto;
    height: 100%;
  }

  .detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .detail-title {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .detail-number {
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }


  .detail-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }

  .branch-info {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .merge-status {
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
  }

  .merge-status.mergeable {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }
  .merge-status.not-mergeable {
    background: color-mix(in srgb, var(--accent-red) 15%, transparent);
    color: var(--accent-red);
  }

  .review-badge {
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
    color: var(--text-secondary);
  }

  .review-badge.approved {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }

  .review-badge.changes-requested {
    background: color-mix(in srgb, var(--accent-red) 15%, transparent);
    color: var(--accent-red);
  }

  .detail-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }

  .detail-actions .push-right {
    margin-left: auto;
  }

  .merge-group {
    display: flex;
    gap: 0;
    position: relative;
  }


  .merge-dropdown-trigger {
    padding: 5px 8px;
    background: var(--accent-blue);
    color: var(--text-primary);
    border: 1px solid var(--accent-blue);
    border-left: 1px solid color-mix(in srgb, var(--text-primary) 20%, transparent);
    border-radius: 0 4px 4px 0;
    font-family: var(--font-icons);
    font-size: 9px;
    cursor: pointer;
  }

  .merge-dropdown-trigger:hover {
    opacity: 0.9;
  }

  .merge-dropdown-menu {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px;
    min-width: 160px;
    z-index: 10;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3); /* beardgit:allow-hex: drop shadow neutral */
  }

  .merge-dropdown-menu button {
    display: block;
    width: 100%;
    padding: 6px 10px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    border-radius: 4px;
  }

  .merge-dropdown-menu button:hover {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }

  .merge-dropdown-menu button.active {
    color: var(--accent-blue);
    font-weight: 600;
  }

  .error-msg {
    margin: 0 0 12px;
    padding: 6px 10px;
    background: var(--overlay-accent-red);
    border: 1px solid color-mix(in srgb, var(--accent-red) 30%, transparent);
    border-radius: 4px;
    color: var(--accent-red);
    font-size: 12px;
  }

  .section {
    margin-bottom: 16px;
  }

  .section-title {
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .description-body {
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.5;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }

  .description-body :global(h1),
  .description-body :global(h2),
  .description-body :global(h3) {
    margin: 8px 0 4px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .description-body :global(h1) { font-size: 16px; }
  .description-body :global(h2) { font-size: 14px; }
  .description-body :global(h3) { font-size: 13px; }

  .description-body :global(code) {
    padding: 1px 4px;
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .description-body :global(pre) {
    padding: 8px 12px;
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    margin: 8px 0;
  }

  .description-body :global(pre code) {
    padding: 0;
    background: none;
  }

  .description-body :global(a) {
    color: var(--accent-blue);
    text-decoration: none;
  }

  .description-body :global(a:hover) {
    text-decoration: underline;
  }

  .description-body :global(blockquote) {
    margin: 8px 0;
    padding: 4px 12px;
    border-left: 3px solid var(--border);
    color: var(--text-secondary);
  }

  .description-body :global(ul),
  .description-body :global(ol) {
    margin: 4px 0;
    padding-left: 20px;
  }

  .description-body :global(li) {
    margin: 2px 0;
  }

  .description-body :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 12px 0;
  }

  .description-body :global(img) {
    max-width: 100%;
    border-radius: 4px;
  }

  /*
   * GFM additions — tables + task-list checkboxes. The pre/code/a/list
   * rules above already cover the rest of the spec's markdown-body
   * ruleset; only the net-new GFM output tags need new rules.
   * Theme tokens only: `--border` for grid lines, no hard-coded colour.
   */
  .description-body :global(table) {
    border-collapse: collapse;
    margin: 6px 0;
  }
  .description-body :global(th),
  .description-body :global(td) {
    border: 1px solid var(--border);
    padding: 4px 8px;
  }
  .description-body :global(input[type="checkbox"]) {
    margin-right: 4px;
    /* Task-list checkboxes are display-only. */
    pointer-events: none;
  }

  .file-list {
    font-size: 12px;
  }

  .empty-section {
    margin: 0;
    color: var(--text-secondary);
    font-size: 12px;
    font-style: italic;
  }

  .diff-error {
    margin: 0;
    padding: 6px 10px;
    background: var(--overlay-accent-red);
    color: var(--accent-red);
    font-size: 12px;
    border-radius: 4px;
  }

  .file-row {
    width: 100%;
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 4px 10px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    border-radius: 3px;
  }
  .file-row:hover { background: color-mix(in srgb, var(--text-primary) 4%, transparent); }
  .file-row.selected { background: var(--overlay-accent-blue); }

  .file-status {
    width: 14px;
    text-align: center;
    font-weight: 700;
    font-size: 10px;
  }
  .file-status.added {
    color: var(--accent-green);
  }
  .file-status.deleted {
    color: var(--accent-red);
  }
  .file-path {
    flex: 1;
    font-family: var(--font-mono);
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-adds {
    color: var(--accent-green);
  }
  .file-dels {
    color: var(--accent-red);
  }

  .comment {
    margin-bottom: 12px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .comment-header {
    display: flex;
    gap: 8px;
    margin-bottom: 4px;
    font-size: 11px;
    flex-wrap: wrap;
  }
  .comment-author {
    font-weight: 600;
    color: var(--text-primary);
  }
  .comment-date {
    color: var(--text-secondary);
  }
  .comment-file {
    font-family: var(--font-mono);
    color: var(--accent-blue);
    font-size: 10px;
  }
  .comment-body {
    font-size: 12px;
    color: var(--text-primary);
    word-wrap: break-word;
    overflow-wrap: break-word;
    line-height: 1.4;
  }

  .comment-body :global(code) {
    padding: 1px 4px;
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .comment-body :global(a) {
    color: var(--accent-blue);
    text-decoration: none;
  }

  /* GFM additions for inline comments — same rationale as
   * `.description-body`. pre/ul/ol inherit from the comment-body
   * text defaults; only net-new GFM output needs explicit rules. */
  .comment-body :global(pre) {
    padding: 8px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.4;
    margin: 6px 0;
  }
  .comment-body :global(pre code) {
    padding: 0;
    background: none;
  }
  .comment-body :global(table) {
    border-collapse: collapse;
    margin: 6px 0;
  }
  .comment-body :global(th),
  .comment-body :global(td) {
    border: 1px solid var(--border);
    padding: 4px 8px;
  }
  .comment-body :global(input[type="checkbox"]) {
    margin-right: 4px;
    pointer-events: none;
  }
  .comment-body :global(ul),
  .comment-body :global(ol) {
    padding-left: 20px;
    margin: 4px 0;
  }

  .comment-input-section {
    border-top: 1px solid var(--border);
    padding-top: 12px;
  }

  .comment-textarea {
    width: 100%;
    padding: 8px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    resize: vertical;
    min-height: 50px;
    box-sizing: border-box;
    margin-bottom: 8px;
  }

  .comment-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .btn-comment {
    padding: 5px 12px;
    background: var(--accent-blue);
    color: var(--text-primary);
    border: 1px solid var(--accent-blue);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn-comment:hover { opacity: 0.9; }
  .btn-comment:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-request-changes {
    padding: 5px 12px;
    background: var(--overlay-accent-red);
    color: var(--accent-red);
    border: 1px solid color-mix(in srgb, var(--accent-red) 30%, transparent);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn-request-changes:hover { background: color-mix(in srgb, var(--accent-red) 20%, transparent); }
  .btn-request-changes:disabled { opacity: 0.5; cursor: not-allowed; }

  .draft-toggle-btn {
    padding: 5px 12px;
    background: none;
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }
  .draft-toggle-btn:hover { background: color-mix(in srgb, var(--text-primary) 5%, transparent); }


  .comment.resolved { opacity: 0.6; }

  .resolve-btn {
    margin-left: auto;
    padding: 2px 8px;
    background: none;
    color: var(--accent-green);
    border: 1px solid var(--border);
    border-radius: 10px;
    font-size: 10px;
    cursor: pointer;
  }
  .resolve-btn.is-resolved { color: var(--text-secondary); }
  .resolve-btn:hover { border-color: var(--accent-green); }

  .checkout-toast {
    position: fixed;
    bottom: 24px;
    right: 24px;
    padding: 12px 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--accent-green);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4); /* beardgit:allow-hex: drop shadow neutral */
    z-index: 100;
  }
  .toast-sub { color: var(--text-secondary); font-size: 11px; }
</style>
