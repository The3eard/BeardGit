<!--
  IssueDetail — detail panel for a selected issue. Shows summary, description,
  labels, assignees, milestone, comments, and action buttons.
-->
<script lang="ts">
  import {
    issueDetail,
    issueDetailLoading,
    closeIssue,
    reopenIssue,
    addIssueComment,
    addIssueLabels,
    removeIssueLabels,
    addIssueAssignees,
    removeIssueAssignees,
    setIssueMilestone,
    labelsCache,
    labelsCacheLoading,
    refreshLabelsCache,
  } from "../../stores/issues";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as m from "$lib/paraglide/messages";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import LabelPicker from "../common/LabelPicker.svelte";
  import AssigneePicker from "./AssigneePicker.svelte";
  import MilestonePicker from "./MilestonePicker.svelte";
  import Xrefs from "../common/Xrefs.svelte";
  import { renderMarkdown } from "../../utils/markdown";
  import { Button, IconButton } from "$lib/components/ui";

  let showCloseConfirm = $state(false);
  let actionError = $state("");
  let commentBody = $state("");
  let commentSubmitting = $state(false);

  let showLabelPicker = $state(false);
  let showAssigneePicker = $state(false);
  let showMilestonePicker = $state(false);

  async function handleClose() {
    const d = $issueDetail;
    if (!d) return;
    try {
      actionError = "";
      await closeIssue(d.summary.number);
    } catch (e) {
      actionError = m.issues_close_failed({ error: String(e) });
    }
    showCloseConfirm = false;
  }

  async function handleReopen() {
    const d = $issueDetail;
    if (!d) return;
    try {
      actionError = "";
      await reopenIssue(d.summary.number);
    } catch (e) {
      actionError = String(e);
    }
  }

  async function handleAddComment() {
    const d = $issueDetail;
    if (!d || !commentBody.trim()) return;
    commentSubmitting = true;
    try {
      actionError = "";
      await addIssueComment(d.summary.number, commentBody.trim());
      commentBody = "";
    } catch (e) {
      actionError = String(e);
    } finally {
      commentSubmitting = false;
    }
  }

  function openLabelPicker() {
    if ($labelsCache.length === 0) void refreshLabelsCache();
    showLabelPicker = true;
  }

  async function applyLabels(added: string[], removed: string[]) {
    const d = $issueDetail;
    if (!d) {
      showLabelPicker = false;
      return;
    }
    try {
      actionError = "";
      if (added.length) await addIssueLabels(d.summary.number, added);
      if (removed.length) await removeIssueLabels(d.summary.number, removed);
    } catch (e) {
      actionError = String(e);
    }
    showLabelPicker = false;
  }

  async function applyAssignees(added: string[], removed: string[]) {
    const d = $issueDetail;
    if (!d) {
      showAssigneePicker = false;
      return;
    }
    try {
      actionError = "";
      if (added.length) await addIssueAssignees(d.summary.number, added);
      if (removed.length) await removeIssueAssignees(d.summary.number, removed);
    } catch (e) {
      actionError = String(e);
    }
    showAssigneePicker = false;
  }

  async function applyMilestone(id: number | null) {
    const d = $issueDetail;
    if (!d) {
      showMilestonePicker = false;
      return;
    }
    try {
      actionError = "";
      await setIssueMilestone(d.summary.number, id);
    } catch (e) {
      actionError = String(e);
    }
    showMilestonePicker = false;
  }
</script>

{#if $issueDetailLoading}
  <div class="detail-empty">{m.issues_loading()}</div>
{:else if !$issueDetail}
  <div class="detail-empty">{m.issues_select()}</div>
{:else}
  {@const detail = $issueDetail}
  <div class="issue-detail">
    <div class="detail-header">
      <h3 class="detail-title">
        <span class="detail-number">#{detail.summary.number}</span>
        {detail.summary.title}
      </h3>
      <IconButton
        tone="default"
        icon={""}
        description={m.issues_open_browser()}
        onclick={() => openUrl(detail.summary.url)}
      />
    </div>

    <div class="detail-meta">
      <span class="state-badge" class:closed={detail.summary.state === "closed"}>
        {detail.summary.state === "open" ? m.issues_state_open() : m.issues_state_closed()}
      </span>
      <span class="author">{detail.summary.author}</span>
      <span class="created">{new Date(detail.summary.created_at).toLocaleDateString()}</span>
    </div>

    <div class="detail-actions">
      {#if detail.summary.state === "open"}
        <Button variant="neutral" size="sm" onclick={() => { showCloseConfirm = true; }}>{m.issues_close()}</Button>
      {:else}
        <Button variant="primary" size="sm" onclick={handleReopen}>{m.issues_reopen()}</Button>
      {/if}
    </div>

    {#if actionError}<p class="error-msg">{actionError}</p>{/if}

    {#if detail.body}
      <div class="section">
        <h4 class="section-title">{m.issues_description()}</h4>
        <div class="description-body">
          <Xrefs text={detail.body} render={(t) => renderMarkdown(t)} />
        </div>
      </div>
    {/if}

    <div class="section">
      <div class="section-head">
        <h4 class="section-title">{m.issues_labels()}</h4>
        <IconButton tone="default" icon={""} description={m.issues_edit()} onclick={openLabelPicker} />
      </div>
      <div class="label-list">
        {#each detail.summary.labels as label}
          <span
            class="label-tag"
            style:background={label.color ? `#${label.color}20` : "color-mix(in srgb, var(--text-primary) 10%, transparent)"} /* beardgit:allow-hex: dynamic GitHub API label color */
            style:color={label.color ? `#${label.color}` : "var(--text-secondary)"} /* beardgit:allow-hex: dynamic GitHub API label color */
          >{label.name}</span>
        {/each}
        {#if detail.summary.labels.length === 0}
          <span class="empty-inline">{m.issues_no_labels()}</span>
        {/if}
      </div>
    </div>

    <div class="section">
      <div class="section-head">
        <h4 class="section-title">{m.issues_assignees()}</h4>
        <IconButton tone="default" icon={""} description={m.issues_edit()} onclick={() => showAssigneePicker = true} />
      </div>
      <div class="assignee-list">
        {#each detail.summary.assignees as a}
          <span class="assignee-tag">{a}</span>
        {/each}
        {#if detail.summary.assignees.length === 0}
          <span class="empty-inline">{m.issues_no_assignees()}</span>
        {/if}
      </div>
    </div>

    <div class="section">
      <div class="section-head">
        <h4 class="section-title">{m.issues_milestone()}</h4>
        <IconButton tone="default" icon={""} description={m.issues_edit()} onclick={() => showMilestonePicker = true} />
      </div>
      <div class="milestone-display">
        {detail.summary.milestone?.title ?? m.issues_no_milestone()}
      </div>
    </div>

    {#if detail.comments.length > 0}
      <div class="section">
        <h4 class="section-title">
          {m.issues_comments({ count: detail.comments.length.toString() })}
        </h4>
        <div class="comment-list">
          {#each detail.comments as comment (comment.id)}
            <div class="comment">
              <div class="comment-header">
                <span class="comment-author">{comment.author}</span>
                <span class="comment-date">{new Date(comment.created_at).toLocaleString()}</span>
              </div>
              <div class="comment-body">
                <Xrefs text={comment.body} render={(t) => renderMarkdown(t)} />
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    {#if detail.summary.state === "open"}
      <div class="section comment-input-section">
        <textarea
          class="comment-textarea"
          placeholder={m.issues_comment_placeholder()}
          bind:value={commentBody}
          rows="3"
        ></textarea>
        <div class="comment-actions">
          <button
            class="btn-comment"
            disabled={!commentBody.trim() || commentSubmitting}
            onclick={handleAddComment}
          >
            {m.issues_add_comment()}
          </button>
        </div>
      </div>
    {/if}
  </div>
{/if}

{#if showCloseConfirm && $issueDetail}
  <ConfirmDialog
    title={m.issues_close()}
    message={m.issues_close_confirm()}
    confirmLabel={m.issues_close()}
    destructive
    onConfirm={handleClose}
    onCancel={() => { showCloseConfirm = false; }}
  />
{/if}

{#if showLabelPicker && $issueDetail}
  <LabelPicker
    labels={$labelsCache}
    loading={$labelsCacheLoading}
    current={$issueDetail.summary.labels.map((l) => l.name)}
    onApply={applyLabels}
    onCancel={() => showLabelPicker = false}
  />
{/if}

{#if showAssigneePicker && $issueDetail}
  <AssigneePicker
    current={$issueDetail.summary.assignees}
    onApply={applyAssignees}
    onCancel={() => showAssigneePicker = false}
  />
{/if}

{#if showMilestonePicker && $issueDetail}
  <MilestonePicker
    current={$issueDetail.summary.milestone?.id ?? null}
    onConfirm={applyMilestone}
    onCancel={() => showMilestonePicker = false}
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
  .issue-detail {
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
    gap: 8px;
    align-items: center;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }
  .state-badge {
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 600;
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }
  .state-badge.closed {
    background: color-mix(in srgb, var(--accent-purple) 15%, transparent);
    color: var(--accent-purple);
  }
  .detail-actions {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
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
  .section { margin-bottom: 16px; }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
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
  /*
   * Markdown-body rules — content comes in via `{@html}` so every
   * descendant selector is `:global(...)`. Theme tokens only; rules
   * duplicated (intentionally) in `.comment-body` below because
   * scoped Svelte styles cannot be shared across sibling wrappers
   * without pushing the stylesheet to the global layer.
   */
  .description-body :global(pre) {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px 10px;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .description-body :global(code:not(pre code)) {
    padding: 1px 4px;
    background: var(--bg-secondary);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 12px;
  }
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
    pointer-events: none;
  }
  .description-body :global(a) {
    color: var(--accent-blue);
    text-decoration: none;
  }
  .description-body :global(a:hover) {
    text-decoration: underline;
  }
  .description-body :global(ul),
  .description-body :global(ol) {
    padding-left: 20px;
  }
  .label-list, .assignee-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .label-tag {
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 11px;
  }
  .assignee-tag {
    padding: 2px 8px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent-blue) 15%, transparent);
    color: var(--accent-blue);
    font-size: 11px;
  }
  .empty-inline {
    color: var(--text-secondary);
    font-size: 12px;
    font-style: italic;
  }
  .milestone-display {
    font-size: 13px;
    color: var(--text-primary);
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
  }
  .comment-author {
    font-weight: 600;
    color: var(--text-primary);
  }
  .comment-date { color: var(--text-secondary); }
  .comment-body {
    font-size: 12px;
    color: var(--text-primary);
    line-height: 1.4;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }
  .comment-body :global(pre) {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px 10px;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .comment-body :global(code:not(pre code)) {
    padding: 1px 4px;
    background: var(--bg-secondary);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 11px;
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
  .comment-body :global(a) {
    color: var(--accent-blue);
    text-decoration: none;
  }
  .comment-body :global(a:hover) {
    text-decoration: underline;
  }
  .comment-body :global(ul),
  .comment-body :global(ol) {
    padding-left: 20px;
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
  .btn-comment:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
