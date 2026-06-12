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
    releaseDetailError,
    selectedReleaseTag,
    selectRelease,
    doDeleteRelease,
    doPublishRelease,
    doUploadAsset,
    doDeleteAsset,
    refreshSelectedDetail,
    completeUpload,
  } from "../../stores/releases";
  import ForgeDetailShell from "../common/ForgeDetailShell.svelte";
  import { Button, IconButton } from "$lib/components/ui";
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
  // An "empty" release has no notes and no assets. We still render the
  // assets table + upload zone so the user can seed a blank release;
  // only the body section swaps to a neutral empty-state string instead
  // of a lonely em-dash.
  let isReleaseEmpty = $derived(
    !detail?.body?.trim() && (detail?.assets?.length ?? 0) === 0,
  );

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

<ForgeDetailShell
  loading={$releaseDetailLoading}
  error={$releaseDetailError}
  isEmpty={!detail && !$releaseDetailLoading && !$releaseDetailError}
  emptyMessage={m.release_detail_empty()}
  emptyIcon={"\uF135"}
  onRetry={() => {
    const tag = $selectedReleaseTag;
    if (tag) selectRelease(tag);
  }}
>
  {#snippet content()}
    {#if detail}
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
      {:else if isReleaseEmpty}
        <p class="empty-body">
          {m.release_empty_blank({ tag: detail.summary.tag })}
        </p>
      {:else}
        <p class="muted">—</p>
      {/if}
    </section>

    <section class="assets">
      <div class="assets-header">
        <h3>{m.release_assets_heading()}</h3>
        <Button variant="neutral" size="sm" onclick={pickAndUpload}>
          {m.release_upload_button()}
        </Button>
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
                  <IconButton icon={"\uF00D"} description={m.release_delete_asset()} tone="danger" onclick={() => handleDeleteAsset(asset.id, asset.name)} />
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </section>

    <footer class="actions">
      {#if canPublish}
        <Button variant="primary" onclick={handlePublish}>
          {m.release_publish_button()}
        </Button>
      {/if}
      <Button variant="danger" onclick={handleDelete}>
        {m.release_delete_button()}
      </Button>
    </footer>

    {#if dragOver}
      <div class="drop-hint">{m.release_drop_hint()}</div>
    {/if}
      </div>
    {/if}
  {/snippet}
</ForgeDetailShell>

<style>
  .detail {
    position: relative;
    padding: 12px 16px;
    overflow-y: auto;
    height: 100%;
  }
  .detail.drag-over {
    outline: 2px dashed var(--accent-primary);
    outline-offset: -8px;
  }
  .drop-hint {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--accent-primary) 8%, transparent);
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--accent-primary);
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
    font-size: var(--font-size-xl);
  }
  .tag {
    font-family: var(--font-mono);
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--bg-secondary);
    font-size: var(--font-size-sm);
  }
  .author {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }
  .date {
    color: var(--text-secondary);
    font-size: var(--font-size-xs);
  }
  .body {
    padding: 12px 0;
    font-size: var(--font-size-md);
    line-height: 1.5;
  }
  /*
   * Markdown-body rules. Content is injected via `{@html}` so every
   * descendant selector has to be `:global(...)` — Svelte's scoped
   * hashing doesn't reach nodes added at runtime.
   *
   * All colours/fonts go through theme tokens (`--bg-secondary`,
   * `--border`, `--font-mono`, `--accent-blue`); no hard-coded values.
   */
  .body :global(pre) {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px 10px;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }
  .body :global(code:not(pre code)) {
    background: var(--bg-secondary);
    border-radius: 3px;
    padding: 0 4px;
    font-family: var(--font-mono);
    font-size: 0.95em;
  }
  .body :global(table) {
    border-collapse: collapse;
    margin: 6px 0;
  }
  .body :global(th),
  .body :global(td) {
    border: 1px solid var(--border);
    padding: 4px 8px;
  }
  .body :global(a) {
    color: var(--accent-primary);
    text-decoration: none;
  }
  .body :global(a:hover) {
    text-decoration: underline;
  }
  .body :global(ul),
  .body :global(ol) {
    padding-left: 20px;
  }
  .muted {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }
  .empty-body {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-md);
    font-style: italic;
  }
  .assets-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin: 8px 0;
  }
  .assets-header h3 {
    margin: 0;
    font-size: var(--font-size-md);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }
  .empty-assets {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    padding: 8px 0;
  }
  .assets-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-sm);
  }
  .assets-table th,
  .assets-table td {
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    text-align: left;
  }
  .assets-table th {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .assets-table a {
    color: var(--accent-primary);
    text-decoration: none;
  }
  .assets-table a:hover {
    text-decoration: underline;
  }
  .actions {
    display: flex;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    margin-top: 12px;
  }
  /*
   * Legacy `.loading`, `.empty`, `.spinner` + `@keyframes spin`
   * lived here for the local loading/empty states. Those states are
   * now rendered by `ForgeDetailShell`, which owns its own spinner,
   * so the rules were removed to avoid dead CSS.
   */
  .badge {
    font-size: var(--font-size-2xs);
    padding: 1px 6px;
    border-radius: 3px;
    font-weight: 600;
    text-transform: uppercase;
  }
  .badge-draft {
    background: color-mix(in srgb, var(--accent-orange) 15%, transparent);
    color: var(--accent-orange);
  }
  .badge-prerelease {
    background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
    color: var(--accent-primary);
  }
  .badge-published {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }
  .error-msg {
    padding: 6px 10px;
    background: var(--overlay-accent-red);
    border: 1px solid color-mix(in srgb, var(--accent-red) 30%, transparent);
    border-radius: 4px;
    color: var(--accent-red);
    font-size: var(--font-size-sm);
    margin-bottom: 8px;
  }
</style>
