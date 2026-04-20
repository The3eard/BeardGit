<!--
  ReleaseDetail — header, markdown-rendered notes body, assets table, and
  drag-drop zone for asset uploads. Actions: Publish (GitHub draft only),
  Delete. Uploads are fire-and-forget tasks; progress rows show while the
  task is running and disappear on completion.
-->
<script lang="ts">
  import {
    releaseDetail,
    releaseDetailLoading,
    selectedReleaseTag,
    doDeleteRelease,
    doPublishRelease,
    doUploadAsset,
    doDeleteAsset,
    refreshSelectedDetail,
    completeUpload,
  } from "../../stores/releases";
  import { activeProvider } from "../../stores/provider";
  import { renderMarkdown } from "../../utils/markdown";
  import { formatRelativeTime } from "../../utils/time";
  import * as m from "$lib/paraglide/messages";
  import AssetUploadProgress from "./AssetUploadProgress.svelte";
  import Xrefs from "../common/Xrefs.svelte";
  import type { TaskId } from "../../types";
  import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let dragOver = $state(false);
  let uploadRows = $state<{ taskId: TaskId; fileName: string }[]>([]);
  let errorMsg = $state("");

  let detail = $derived($releaseDetail);
  let isGitHub = $derived($activeProvider?.kind === "github");
  let canPublish = $derived(isGitHub && detail?.summary.state === "draft");

  function formatSize(bytes: number): string {
    if (!bytes) return "—";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) {
      return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    }
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function onDragOver(e: DragEvent): void {
    e.preventDefault();
    dragOver = true;
  }

  function onDragLeave(): void {
    dragOver = false;
  }

  async function onDrop(e: DragEvent): Promise<void> {
    e.preventDefault();
    dragOver = false;
    if (!detail) return;
    // Browser File objects in Tauri v2 webview don't expose absolute paths on
    // drop, so fall back to the native picker when the user drops files.
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (files.length > 0) {
      await pickAndUpload();
    }
  }

  async function pickAndUpload(): Promise<void> {
    const tag = $selectedReleaseTag;
    if (!tag) return;
    const picked = await dialogOpen({ multiple: true, directory: false });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    try {
      for (const p of paths) {
        const fileName = p.split(/[\\/]/).pop() ?? p;
        const taskId = await doUploadAsset(tag, p);
        uploadRows = [...uploadRows, { taskId, fileName }];
      }
      errorMsg = "";
    } catch (e) {
      errorMsg = String(e);
    }
  }

  function onUploadComplete(taskId: TaskId): void {
    const tag = $selectedReleaseTag;
    if (tag) completeUpload(tag, taskId);
    uploadRows = uploadRows.filter((r) => r.taskId !== taskId);
    void refreshSelectedDetail();
  }

  async function handleDelete(): Promise<void> {
    if (!detail) return;
    // eslint-disable-next-line no-alert
    if (!confirm(m.release_delete_confirm({ tag: detail.summary.tag }))) return;
    try {
      errorMsg = "";
      await doDeleteRelease(detail.summary.tag);
    } catch (e) {
      errorMsg = String(e);
    }
  }

  async function handlePublish(): Promise<void> {
    if (!detail) return;
    try {
      errorMsg = "";
      await doPublishRelease(detail.summary.tag);
    } catch (e) {
      errorMsg = String(e);
    }
  }

  async function handleDeleteAsset(id: number, name: string): Promise<void> {
    if (!detail) return;
    // eslint-disable-next-line no-alert
    if (!confirm(m.release_delete_asset_confirm({ name }))) return;
    try {
      errorMsg = "";
      await doDeleteAsset(detail.summary.tag, id);
    } catch (e) {
      errorMsg = String(e);
    }
  }

  function stateLabel(s: string): string {
    if (s === "draft") return m.release_state_draft();
    if (s === "prerelease") return m.release_state_prerelease();
    return m.release_state_published();
  }

  function onAssetClick(e: MouseEvent, url: string): void {
    e.preventDefault();
    void openUrl(url);
  }
</script>

{#if $releaseDetailLoading && !detail}
  <div class="loading"><div class="spinner"></div></div>
{:else if !detail}
  <div class="empty">{m.release_detail_empty()}</div>
{:else}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="detail"
    class:drag-over={dragOver}
    ondragover={onDragOver}
    ondragleave={onDragLeave}
    ondrop={onDrop}
  >
    <header class="header">
      <span class="badge badge-{detail.summary.state}">
        {stateLabel(detail.summary.state)}
      </span>
      <h2 class="title">{detail.summary.name || detail.summary.tag}</h2>
      <code class="tag">{detail.summary.tag}</code>
      <span class="author">{detail.summary.author}</span>
      {#if detail.summary.published_at}
        <span class="date">
          {formatRelativeTime(detail.summary.published_at)}
        </span>
      {/if}
    </header>

    {#if errorMsg}
      <p class="error-msg">{errorMsg}</p>
    {/if}

    <section class="body">
      {#if detail.body}
        <Xrefs text={detail.body} render={renderMarkdown} />
      {:else}
        <p class="muted">—</p>
      {/if}
    </section>

    <section class="assets">
      <div class="assets-header">
        <h3>{m.release_assets_heading()}</h3>
        <button class="btn-small" onclick={pickAndUpload}>
          {m.release_upload_button()}
        </button>
      </div>

      {#if uploadRows.length > 0}
        <div class="uploads">
          {#each uploadRows as row (row.taskId)}
            <AssetUploadProgress
              taskId={row.taskId}
              fileName={row.fileName}
              onComplete={() => onUploadComplete(row.taskId)}
            />
          {/each}
        </div>
      {/if}

      {#if detail.assets.length === 0}
        <p class="empty-assets">{m.release_assets_empty()}</p>
      {:else}
        <table class="assets-table">
          <thead>
            <tr>
              <th>{m.release_asset_name()}</th>
              <th>{m.release_asset_size()}</th>
              <th>{m.release_asset_downloads()}</th>
              <th aria-label="actions"></th>
            </tr>
          </thead>
          <tbody>
            {#each detail.assets as asset (asset.id)}
              <tr>
                <td>
                  <a
                    href={asset.url}
                    onclick={(e) => onAssetClick(e, asset.url)}
                  >{asset.name}</a>
                </td>
                <td>{formatSize(asset.size)}</td>
                <td>{asset.download_count}</td>
                <td>
                  <button
                    class="btn-icon"
                    onclick={() => handleDeleteAsset(asset.id, asset.name)}
                    title={m.release_delete_asset()}
                    aria-label={m.release_delete_asset()}
                  >
                    <span class="nf">{"\uF00D"}</span>
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </section>

    <footer class="actions">
      {#if canPublish}
        <button class="btn btn-confirm" onclick={handlePublish}>
          {m.release_publish_button()}
        </button>
      {/if}
      <button class="btn btn-danger" onclick={handleDelete}>
        {m.release_delete_button()}
      </button>
    </footer>

    {#if dragOver}
      <div class="drop-hint">{m.release_drop_hint()}</div>
    {/if}
  </div>
{/if}

<style>
  .detail {
    position: relative;
    padding: 12px 16px;
    overflow-y: auto;
    height: 100%;
  }
  .detail.drag-over {
    outline: 2px dashed var(--accent-blue);
    outline-offset: -8px;
  }
  .drop-hint {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(88, 166, 255, 0.08);
    font-size: 14px;
    font-weight: 600;
    color: var(--accent-blue);
    pointer-events: none;
  }
  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }
  .title {
    margin: 0;
    font-size: 16px;
  }
  .tag {
    font-family: var(--font-mono);
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--bg-secondary);
    font-size: 12px;
  }
  .author {
    color: var(--text-secondary);
    font-size: 12px;
  }
  .date {
    color: var(--text-secondary);
    font-size: 11px;
  }
  .body {
    padding: 12px 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .muted {
    color: var(--text-secondary);
    font-size: 12px;
  }
  .assets-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin: 8px 0;
  }
  .assets-header h3 {
    margin: 0;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }
  .btn-small {
    padding: 4px 10px;
    background: var(--accent-blue);
    color: #fff;
    border: none;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }
  .empty-assets {
    color: var(--text-secondary);
    font-size: 12px;
    padding: 8px 0;
  }
  .assets-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  .assets-table th,
  .assets-table td {
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    text-align: left;
  }
  .assets-table th {
    font-size: 11px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .assets-table a {
    color: var(--accent-blue);
    text-decoration: none;
  }
  .assets-table a:hover {
    text-decoration: underline;
  }
  .btn-icon {
    color: var(--accent-red);
  }
  .btn-icon:hover:not(:disabled) {
    color: var(--accent-red);
    background: rgba(248, 81, 73, 0.12);
  }
  .btn-icon .nf {
    font-family: var(--font-icons);
  }
  .actions {
    display: flex;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    margin-top: 12px;
  }
  .btn {
    padding: 6px 14px;
    border-radius: 4px;
    border: 1px solid var(--border);
    font-size: 12px;
    cursor: pointer;
    background: var(--bg-secondary);
    color: var(--text-primary);
  }
  .btn-confirm {
    background: var(--accent-green);
    color: #fff;
    border-color: transparent;
  }
  .btn-danger {
    background: transparent;
    color: var(--accent-red);
    border-color: var(--accent-red);
  }
  .loading,
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border);
    border-top-color: var(--accent-blue);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 3px;
    font-weight: 600;
    text-transform: uppercase;
  }
  .badge-draft {
    background: rgba(255, 193, 7, 0.15);
    color: var(--accent-amber, #ffc107);
  }
  .badge-prerelease {
    background: rgba(33, 150, 243, 0.15);
    color: var(--accent-blue);
  }
  .badge-published {
    background: rgba(63, 185, 80, 0.15);
    color: var(--accent-green);
  }
  .error-msg {
    padding: 6px 10px;
    background: rgba(248, 81, 73, 0.1);
    border: 1px solid rgba(248, 81, 73, 0.3);
    border-radius: 4px;
    color: var(--accent-red);
    font-size: 12px;
    margin-bottom: 8px;
  }
</style>
