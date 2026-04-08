<!--
  MrPrDetail — detail panel for a selected merge request / pull request.

  Shows summary, description, labels, changed files, comments, and
  action buttons for merge/close/approve/request-changes/comment.
-->
<script lang="ts">
  import {
    mrPrDetail,
    mrPrDetailLoading,
    mrPrDiffFiles,
    mergeMrPr,
    closeMrPr,
    approveMrPr,
    requestChangesMrPr,
    addMrPrComment,
  } from "../../stores/mr-pr";
  import { activeProvider } from "../../stores/provider";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as m from "$lib/paraglide/messages";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";

  let isGitHub = $derived($activeProvider?.kind === "github");
  let selectMessage = $derived(isGitHub ? m.mrpr_select_github() : m.mrpr_select());

  // Merge/close confirmation state
  let showMergeConfirm = $state(false);
  let mergeStrategy = $state("merge");
  let showCloseConfirm = $state(false);
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
</script>

{#if $mrPrDetailLoading}
  <div class="detail-empty">{m.mrpr_loading()}</div>
{:else if !$mrPrDetail}
  <div class="detail-empty">{selectMessage}</div>
{:else}
  {@const detail = $mrPrDetail}
  <div class="mrpr-detail">
    <div class="detail-header">
      <h3 class="detail-title">
        <span class="detail-number">#{detail.summary.number}</span>
        {detail.summary.title}
      </h3>
      <button class="open-browser-btn" onclick={() => openUrl(detail.summary.url)}>
        {m.mrpr_open_browser()}
      </button>
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

    <!-- Action buttons for open MR/PRs -->
    {#if detail.summary.state === "open"}
      <div class="detail-actions">
        <button class="approve-btn" onclick={handleApprove}>{m.mrpr_approve()}</button>
        <div class="merge-group">
          <select class="merge-select" bind:value={mergeStrategy}>
            <option value="merge">{m.mrpr_merge()}</option>
            <option value="squash">{m.mrpr_merge_squash()}</option>
            <option value="rebase">{m.mrpr_merge_rebase()}</option>
          </select>
          <button class="merge-btn" onclick={() => { showMergeConfirm = true; }}>
            {mergeStrategy === "squash" ? m.mrpr_merge_squash() : mergeStrategy === "rebase" ? m.mrpr_merge_rebase() : m.mrpr_merge()}
          </button>
        </div>
        <button class="close-btn" onclick={() => { showCloseConfirm = true; }}>{m.mrpr_close()}</button>
      </div>
    {/if}

    {#if actionError}
      <p class="error-msg">{actionError}</p>
    {/if}

    {#if detail.body}
      <div class="section">
        <h4 class="section-title">{m.mrpr_description()}</h4>
        <div class="description-body">{detail.body}</div>
      </div>
    {/if}

    {#if detail.summary.labels.length > 0}
      <div class="section">
        <h4 class="section-title">{m.mrpr_labels()}</h4>
        <div class="label-list">
          {#each detail.summary.labels as label}
            <span class="label-tag">{label}</span>
          {/each}
        </div>
      </div>
    {/if}

    <div class="section">
      <h4 class="section-title">{m.mrpr_changed_files({ count: $mrPrDiffFiles.length.toString() })}</h4>
      <div class="file-list">
        {#each $mrPrDiffFiles as file}
          <div class="file-row">
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
          </div>
        {/each}
      </div>
    </div>

    {#if detail.comments.length > 0}
      <div class="section">
        <h4 class="section-title">
          {m.mrpr_comments({ count: detail.comments.length.toString() })}
        </h4>
        <div class="comment-list">
          {#each detail.comments as comment}
            <div class="comment">
              <div class="comment-header">
                <span class="comment-author">{comment.author}</span>
                <span class="comment-date">{new Date(comment.created_at).toLocaleString()}</span>
                {#if comment.path}
                  <span class="comment-file">{comment.path}{comment.line ? `:${comment.line}` : ""}</span>
                {/if}
              </div>
              <div class="comment-body">{comment.body}</div>
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

<style>
  .detail-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    font-size: 13px;
  }

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

  .open-browser-btn {
    padding: 4px 10px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--accent-blue);
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
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
    background: rgba(63, 185, 80, 0.15);
    color: var(--accent-green);
  }
  .merge-status.not-mergeable {
    background: rgba(248, 81, 73, 0.15);
    color: var(--accent-red);
  }

  .review-badge {
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
  }

  .review-badge.approved {
    background: rgba(63, 185, 80, 0.15);
    color: var(--accent-green);
  }

  .review-badge.changes-requested {
    background: rgba(248, 81, 73, 0.15);
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

  .approve-btn {
    padding: 5px 12px;
    background: rgba(63, 185, 80, 0.15);
    color: var(--accent-green);
    border: 1px solid rgba(63, 185, 80, 0.3);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }

  .approve-btn:hover { background: rgba(63, 185, 80, 0.25); }

  .merge-group {
    display: flex;
    gap: 0;
  }

  .merge-select {
    padding: 5px 6px;
    background: var(--bg-primary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px 0 0 4px;
    font-size: 10px;
    cursor: pointer;
    appearance: none;
    max-width: 80px;
  }

  .merge-btn {
    padding: 5px 12px;
    background: var(--accent-blue);
    color: #fff;
    border: 1px solid var(--accent-blue);
    border-radius: 0 4px 4px 0;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
  }

  .merge-btn:hover { opacity: 0.9; }

  .close-btn {
    padding: 5px 12px;
    background: rgba(248, 81, 73, 0.1);
    color: var(--accent-red);
    border: 1px solid rgba(248, 81, 73, 0.3);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    margin-left: auto;
  }

  .close-btn:hover { background: rgba(248, 81, 73, 0.2); }

  .error-msg {
    margin: 0 0 12px;
    padding: 6px 10px;
    background: rgba(248, 81, 73, 0.1);
    border: 1px solid rgba(248, 81, 73, 0.3);
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
    white-space: pre-wrap;
  }

  .label-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .label-tag {
    padding: 2px 8px;
    border-radius: 12px;
    background: rgba(88, 166, 255, 0.15);
    color: var(--accent-blue);
    font-size: 11px;
  }

  .file-list {
    font-size: 12px;
  }

  .file-row {
    display: flex;
    gap: 6px;
    padding: 3px 0;
    align-items: center;
  }

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
    white-space: pre-wrap;
    line-height: 1.4;
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
    color: #fff;
    border: 1px solid var(--accent-blue);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn-comment:hover { opacity: 0.9; }
  .btn-comment:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-request-changes {
    padding: 5px 12px;
    background: rgba(248, 81, 73, 0.1);
    color: var(--accent-red);
    border: 1px solid rgba(248, 81, 73, 0.3);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn-request-changes:hover { background: rgba(248, 81, 73, 0.2); }
  .btn-request-changes:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
