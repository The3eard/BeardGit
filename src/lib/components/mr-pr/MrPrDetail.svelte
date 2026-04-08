<script lang="ts">
  import {
    mrPrDetail,
    mrPrDetailLoading,
    mrPrDiffFiles,
  } from "../../stores/mr-pr";
  import { activeProvider } from "../../stores/provider";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as m from "$lib/paraglide/messages";

  let isGitHub = $derived($activeProvider?.kind === "github");
  let selectMessage = $derived(isGitHub ? m.mrpr_select_github() : m.mrpr_select());
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

      <span class="review-badge">
        {#if detail.review_status === "approved"}
          {m.mrpr_status_approved()}
        {:else if detail.review_status === "changes_requested"}
          {m.mrpr_status_changes_requested()}
        {:else}
          {m.mrpr_status_pending()}
        {/if}
      </span>
    </div>

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
                <span class="comment-date"
                  >{new Date(comment.created_at).toLocaleString()}</span
                >
              </div>
              <div class="comment-body">{comment.body}</div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
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
    margin-bottom: 16px;
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
  }
  .comment-author {
    font-weight: 600;
    color: var(--text-primary);
  }
  .comment-date {
    color: var(--text-secondary);
  }
  .comment-body {
    font-size: 12px;
    color: var(--text-primary);
    white-space: pre-wrap;
    line-height: 1.4;
  }
</style>
