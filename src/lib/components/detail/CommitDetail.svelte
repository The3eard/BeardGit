<script lang="ts">
  import type { CommitInfo, CommitFileChange } from "../../types";
  import * as m from "$lib/paraglide/messages";
  import FileChangeList from "../common/FileChangeList.svelte";

  let {
    commit,
    files = [],
    showNavigateToGraph = false,
    onNavigateToGraph,
    onClose,
    onFileClick,
  }: {
    commit: CommitInfo;
    files?: CommitFileChange[];
    showNavigateToGraph?: boolean;
    onNavigateToGraph?: (oid: string) => void;
    onClose?: () => void;
    onFileClick?: (path: string) => void;
  } = $props();

  function handleFileSelect(path: string) {
    onFileClick?.(path);
  }

  function formatDate(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    return date.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function formatRef(ref: string): string {
    if (ref.startsWith("refs/heads/")) return ref.replace("refs/heads/", "");
    if (ref.startsWith("refs/remotes/")) return ref.replace("refs/remotes/", "");
    if (ref.startsWith("refs/tags/")) return ref.replace("refs/tags/", "");
    if (ref === "HEAD") return "HEAD";
    return ref;
  }

  function refClass(ref: string): string {
    if (ref.startsWith("refs/tags/")) return "ref-tag";
    if (ref.startsWith("refs/remotes/")) return "ref-remote";
    if (ref === "HEAD") return "ref-head";
    return "ref-branch";
  }

</script>

<aside class="commit-detail">
  <div class="detail-header">
    <h3 class="detail-title">{m.commit_detail_title()}</h3>
    <div class="detail-header-actions">
      {#if showNavigateToGraph && onNavigateToGraph}
        <button class="header-btn navigate-btn" onclick={() => onNavigateToGraph!(commit.oid)} title="Show in Graph">
          ↗ Graph
        </button>
      {/if}
      {#if onClose}
        <button class="close-btn" onclick={() => onClose!()}>
          {"\uEA76"}
        </button>
      {/if}
    </div>
  </div>

  <div class="detail-body">
    <div class="detail-section">
      <div class="commit-summary">{commit.summary}</div>
      {#if commit.body}
        <div class="commit-body">{commit.body}</div>
      {/if}
    </div>

    <div class="detail-section">
      <div class="detail-row">
        <span class="detail-label">{m.commit_detail_author()}</span>
        <span class="detail-value">{commit.author}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">{m.commit_detail_email()}</span>
        <span class="detail-value email">{commit.email}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">{m.commit_detail_date()}</span>
        <span class="detail-value">{formatDate(commit.timestamp)}</span>
      </div>
    </div>

    <div class="detail-section">
      <div class="detail-row">
        <span class="detail-label">{m.commit_detail_sha()}</span>
        <span class="detail-value sha">{commit.oid}</span>
      </div>
    </div>

    {#if commit.parents.length > 0}
      <div class="detail-section">
        <div class="detail-label">{m.commit_detail_parents()}</div>
        {#each commit.parents as parent}
          {#if onNavigateToGraph}
            <button class="parent-oid clickable" onclick={() => onNavigateToGraph!(parent)}>
              {parent.substring(0, 12)}
            </button>
          {:else}
            <span class="parent-oid">{parent.substring(0, 12)}</span>
          {/if}
        {/each}
      </div>
    {/if}

    {#if commit.refs.length > 0}
      <div class="detail-section">
        <div class="detail-label">{m.commit_detail_refs()}</div>
        <div class="ref-list">
          {#each commit.refs as ref}
            <span class="ref-badge {refClass(ref)}">{formatRef(ref)}</span>
          {/each}
        </div>
      </div>
    {/if}

    {#if files.length > 0}
      <div class="detail-section">
        <div class="detail-label">{m.commit_detail_files({ count: String(files.length) })}</div>
        <FileChangeList files={files} onSelect={handleFileSelect} />
      </div>
    {/if}
  </div>
</aside>

<style>
  .commit-detail {
    min-width: 0;
    flex: 1;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .detail-header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .detail-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    font-family: var(--font-icons);
    padding: 2px 4px;
    border-radius: 3px;
  }

  .close-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }

  .header-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    font-family: var(--font-icons);
    padding: 2px 4px;
    border-radius: 3px;
  }

  .header-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }

  .navigate-btn {
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    font-size: 11px;
    letter-spacing: 0.3px;
  }

  .detail-body {
    padding: 0;
  }

  .detail-section {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .detail-section:last-child {
    border-bottom: none;
  }

  .commit-summary {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.4;
    word-break: break-word;
  }

  .commit-body {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .detail-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 4px;
  }

  .detail-row:last-child {
    margin-bottom: 0;
  }

  .detail-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    min-width: 48px;
    flex-shrink: 0;
  }

  .detail-value {
    font-size: 12px;
    color: var(--text-primary);
    word-break: break-all;
  }

  .detail-value.email {
    color: var(--accent-blue);
  }

  .detail-value.sha {
    font-family: "SF Mono", "Fira Code", "Consolas", monospace;
    font-size: 11px;
    color: var(--accent-orange);
    word-break: break-all;
  }

  .parent-oid {
    font-family: "SF Mono", "Fira Code", "Consolas", monospace;
    font-size: 11px;
    color: var(--accent-blue);
    margin-top: 4px;
    cursor: default;
  }

  .parent-oid.clickable {
    background: none;
    border: none;
    padding: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent-blue);
    cursor: pointer;
  }

  .parent-oid.clickable:hover {
    text-decoration: underline;
  }

  .ref-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }

  .ref-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 500;
    background: none;
    cursor: default;
  }

  .ref-branch {
    background: rgba(88, 166, 255, 0.12);
    color: var(--accent-blue);
    border: 1px solid rgba(88, 166, 255, 0.3);
  }

  .ref-remote {
    background: rgba(187, 128, 255, 0.12);
    color: var(--accent-purple);
    border: 1px solid rgba(187, 128, 255, 0.3);
  }

  .ref-tag {
    background: rgba(240, 136, 62, 0.12);
    color: var(--accent-orange);
    border: 1px solid rgba(240, 136, 62, 0.3);
  }

  .ref-head {
    background: rgba(247, 120, 186, 0.12);
    color: #f778ba;
    border: 1px solid rgba(247, 120, 186, 0.3);
  }

</style>
