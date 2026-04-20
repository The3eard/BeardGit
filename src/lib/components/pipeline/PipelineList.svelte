<script lang="ts">
  import { onMount } from "svelte";
  import { ciRuns, loadCiRuns, loadMoreCiRuns, loadCiRunDetail, selectedCiRunId, startCiRunListPolling, stopCiRunListPolling, hasMoreCiRuns, hasActiveProvider, retryCiRun, cancelCiRun } from "../../stores/provider";
  import type { CiRun } from "../../types";
  import { repoInfo } from "../../stores/repo";
  import SearchBar from "../common/SearchBar.svelte";
  import type { SearchTag } from "../../search/types";
  import { ciFilters } from "../../search/ci-provider";
  import * as m from "$lib/paraglide/messages";
  import { formatRelativeTime } from "../../utils/time";
  import { ciStatusColor, ciStatusLabel } from "../../utils/status";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import TriggerWorkflowDialog from "./TriggerWorkflowDialog.svelte";

  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);
  let searchTags = $state<SearchTag[]>([]);
  let initialized = false;
  let triggerDialogOpen = $state(false);
  let ctxMenu = $state<{ x: number; y: number; run: CiRun } | null>(null);
  let ctxError = $state<string | null>(null);

  function onRowContextMenu(e: MouseEvent, run: CiRun) {
    e.preventDefault();
    ctxMenu = { x: e.clientX, y: e.clientY, run };
  }

  function closeCtxMenu() { ctxMenu = null; }

  async function retryFromMenu(run: CiRun) {
    closeCtxMenu();
    try { await retryCiRun(run.id); await fetchPipelines(); }
    catch (e) { ctxError = m.pipeline_retry_error({ error: String(e) }); }
  }
  async function cancelFromMenu(run: CiRun) {
    closeCtxMenu();
    try { await cancelCiRun(run.id); await fetchPipelines(); }
    catch (e) { ctxError = m.pipeline_cancel_error({ error: String(e) }); }
  }
  async function openInBrowser(run: CiRun) {
    closeCtxMenu();
    try { await openUrl(run.web_url); } catch { /* ignore */ }
  }

  onMount(() => {
    if ($hasActiveProvider) {
      initAndLoad();
    }
    return () => stopCiRunListPolling();
  });

  // React to provider connection after mount (e.g. auto-reconnect)
  $effect(() => {
    if ($hasActiveProvider && !initialized) {
      initAndLoad();
    }
  });

  async function initAndLoad() {
    if (initialized) return;
    initialized = true;

    // Set default branch filter
    if ($repoInfo?.head_branch) {
      searchTags = [{
        id: `init-${Date.now()}`,
        type: "branch",
        value: $repoInfo.head_branch,
        display: `branch:${$repoInfo.head_branch}`,
      }];
    }

    await fetchPipelines();
  }

  function extractApiParams(tags: SearchTag[]) {
    let branch: string | undefined;
    let status: string | undefined;
    let source: string | undefined;
    for (const tag of tags) {
      if (tag.type === "branch") branch = tag.value;
      if (tag.type === "status") status = tag.value.toLowerCase();
      if (tag.type === "source") source = tag.value.toLowerCase();
    }
    return { branch, status, source };
  }

  async function fetchPipelines() {
    if (loading) return;
    loading = true;
    error = null;
    try {
      const { branch, status, source } = extractApiParams(searchTags);
      await loadCiRuns(branch, source, status);
      startCiRunListPolling(async () => {
        try {
          const p = extractApiParams(searchTags);
          await loadCiRuns(p.branch, p.source, p.status);
        } catch { /* ignore */ }
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function refresh() {
    fetchPipelines();
  }

  function handleSearch(tags: SearchTag[]) {
    searchTags = tags;
    // Re-fetch from API with new filters
    initialized = false;
    initialized = true;
    fetchPipelines();
  }

  // No client-side filtering — all filtering is server-side via the API
  let filteredPipelines = $derived($ciRuns);

  async function handleLoadMore() {
    loadingMore = true;
    try {
      const { branch, status, source } = extractApiParams(searchTags);
      await loadMoreCiRuns(branch, source, status);
    } catch (e) {
      error = String(e);
    } finally {
      loadingMore = false;
    }
  }

  async function selectCiRun(run: CiRun) {
    try {
      await loadCiRunDetail(run.id);
    } catch (e) {
      error = String(e);
    }
  }

  function shortSha(sha: string): string {
    return sha.substring(0, 8);
  }

  function formatDuration(created: string | null, updated: string | null): string {
    if (!created || !updated) return "";
    const start = new Date(created).getTime();
    const end = new Date(updated).getTime();
    const diffSec = Math.max(0, Math.floor((end - start) / 1000));
    if (diffSec === 0) return "";
    const hours = Math.floor(diffSec / 3600);
    const minutes = Math.floor((diffSec % 3600) / 60);
    const seconds = diffSec % 60;
    const mm = String(minutes).padStart(2, "0");
    const ss = String(seconds).padStart(2, "0");
    if (hours > 0) return `${String(hours).padStart(2, "0")}:${mm}:${ss}`;
    return `${mm}:${ss}`;
  }

  function sourceLabel(source: string | null): string {
    switch (source) {
      case "push": return m.pipeline_source_push();
      case "merge_request_event": return m.pipeline_source_mr();
      case "schedule": return m.pipeline_source_schedule();
      case "web": return m.pipeline_source_web();
      case "api": return m.pipeline_source_api();
      case "trigger": return m.pipeline_source_trigger();
      case "merge_train": return m.pipeline_source_merge_train();
      case "parent_pipeline":
      case "pipeline": return m.pipeline_source_child();
      case "pull_request": return m.pipeline_source_pr();
      case "pull_request_target": return m.pipeline_source_pr_target();
      case "workflow_dispatch": return m.pipeline_source_manual();
      case "repository_dispatch": return m.pipeline_source_dispatch();
      case "release": return m.pipeline_source_release();
      case "workflow_call": return m.pipeline_source_reusable();
      default: return source ?? "";
    }
  }

  function sourceBadgeClass(source: string | null): string {
    switch (source) {
      case "push": return "source-badge--branch";
      case "merge_request_event":
      case "merge_train":
      case "pull_request":
      case "pull_request_target": return "source-badge--mr";
      case "schedule": return "source-badge--schedule";
      case "workflow_dispatch": return "source-badge--manual";
      default: return "source-badge--gray";
    }
  }

  let selectedId = $derived($selectedCiRunId);
</script>

<div class="pipeline-list">
  <div class="list-header">
    <span class="list-title">{m.pipeline_title()}</span>
    <div class="header-actions">
      <button
        class="header-btn"
        onclick={() => (triggerDialogOpen = true)}
        disabled={!$hasActiveProvider}
      >
        {m.pipeline_action_trigger()}
      </button>
      <button class="refresh-btn nf" onclick={refresh} disabled={loading} title="Refresh">
        {loading ? "\uF110" : "\uF021"}
      </button>
    </div>
  </div>

  <SearchBar
    filters={ciFilters}
    bind:tags={searchTags}
    placeholder={m.pipeline_search_placeholder()}
    onSearch={handleSearch}
  />

  {#if loading && $ciRuns.length > 0}
    <div class="list-loading-bar">
      <div class="loading-bar-track"><div class="loading-bar-fill"></div></div>
    </div>
  {/if}

  {#if loading && $ciRuns.length === 0}
    <div class="list-loading">
      <div class="spinner"></div>
      <span>{m.pipeline_loading()}</span>
    </div>
  {:else if !$hasActiveProvider}
    <div class="empty-state">
      <h3 class="empty-state-title">{m.pipeline_no_provider()}</h3>
      <p class="empty-state-description">{m.pipeline_no_provider_description()}</p>
    </div>
  {:else if filteredPipelines.length === 0}
    <div class="empty-state">
      {#if $ciRuns.length === 0}
        <h3 class="empty-state-title">{m.pipeline_no_runs()}</h3>
        <p class="empty-state-description">{m.pipeline_no_runs_description()}</p>
      {:else}
        <h3 class="empty-state-title">{m.pipeline_no_match()}</h3>
        <p class="empty-state-description">{m.pipeline_no_match_description()}</p>
      {/if}
    </div>
  {:else}
    <div class="list-items">
      {#each filteredPipelines as run (run.id)}
        <button
          class="pipeline-row"
          class:selected={selectedId === run.id}
          onclick={() => selectCiRun(run)}
          oncontextmenu={(e) => onRowContextMenu(e, run)}
        >
          <div class="row-status">
            <span
              class="status-dot"
              class:status-dot--active={run.status === 'running' || run.status === 'pending' || run.status === 'queued'}
              style="background: {ciStatusColor(run.status)}"
            ></span>
            <div class="status-text">
              <span class="status-label" style="color: {ciStatusColor(run.status)}">
                {ciStatusLabel(run.status)}
              </span>
              {#if run.status === 'success' || run.status === 'failed'}
                {@const dur = formatDuration(run.created_at, run.updated_at)}
                {#if dur}
                  <span class="status-duration">{dur}</span>
                {/if}
              {/if}
            </div>
          </div>

          <div class="row-center">
            <div class="pipeline-title">{run.name || m.pipeline_run_title({ id: String(run.display_id) })}</div>
            <div class="pipeline-meta">
              <span class="pipeline-id">#{run.display_id}</span>
              <span class="pipeline-ref">{run.ref_name}</span>
              <span class="pipeline-sha">{shortSha(run.sha)}</span>
              {#if run.source}
                <span class="source-badge {sourceBadgeClass(run.source)}">
                  {sourceLabel(run.source)}
                </span>
              {/if}
            </div>
          </div>

          <div class="row-time">
            {formatRelativeTime(run.created_at)}
          </div>
        </button>
      {/each}

      {#if $hasMoreCiRuns}
        <button class="load-more-btn" onclick={handleLoadMore} disabled={loadingMore}>
          {#if loadingMore}
            <div class="spinner"></div>
            {m.pipeline_loading()}
          {:else}
            {m.pipeline_load_more({ count: String($ciRuns.length) })}
          {/if}
        </button>
      {/if}
    </div>
  {/if}
</div>

<TriggerWorkflowDialog
  open={triggerDialogOpen}
  onClose={() => { triggerDialogOpen = false; fetchPipelines(); }}
/>

{#if ctxMenu}
  <div
    class="ctx-overlay"
    role="presentation"
    onclick={closeCtxMenu}
    onkeydown={(e) => e.key === "Escape" && closeCtxMenu()}
  ></div>
  <div class="ctx-menu" style="top: {ctxMenu.y}px; left: {ctxMenu.x}px" role="menu">
    {#if ctxMenu.run.status === "failed" || ctxMenu.run.status === "canceled" || ctxMenu.run.status === "timed_out"}
      <button role="menuitem" onclick={() => retryFromMenu(ctxMenu!.run)}>
        {m.pipeline_action_retry()}
      </button>
    {/if}
    {#if ctxMenu.run.status === "running" || ctxMenu.run.status === "pending" || ctxMenu.run.status === "queued"}
      <button role="menuitem" onclick={() => cancelFromMenu(ctxMenu!.run)}>
        {m.pipeline_action_cancel()}
      </button>
    {/if}
    <button role="menuitem" onclick={() => openInBrowser(ctxMenu!.run)}>
      {m.pipeline_action_view_in_browser()}
    </button>
  </div>
{/if}

{#if ctxError}
  <div class="list-error" role="alert">{ctxError}</div>
{/if}
{#if error}
  <div class="list-error" role="alert">{error}</div>
{/if}

<style>
  .pipeline-list { display: flex; flex-direction: column; height: 100%; overflow: hidden; }
  .list-title { font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-secondary); }

  .list-error { padding: 8px 12px; font-size: 12px; color: var(--accent-red); background: var(--overlay-accent-red); margin: 8px; border-radius: 4px; word-break: break-word; }
  .list-loading-bar { padding: 0; }
  .loading-bar-track { height: 2px; background: var(--overlay-hover); overflow: hidden; }
  .loading-bar-fill { height: 100%; width: 40%; background: var(--accent-blue); border-radius: 1px; animation: loading-slide 1s ease-in-out infinite; }
  @keyframes loading-slide { 0% { transform: translateX(-100%); } 100% { transform: translateX(350%); } }

  .pipeline-row { display: flex; align-items: flex-start; gap: 12px; width: 100%; padding: 10px 12px; background: none; border: none; border-bottom: 1px solid var(--border); color: var(--text-primary); cursor: pointer; text-align: left; }
  .pipeline-row:hover { background: color-mix(in srgb, var(--text-primary) 3%, transparent); }
  .pipeline-row.selected { background: color-mix(in srgb, var(--accent-blue) 8%, transparent); }

  .row-status { display: flex; align-items: flex-start; gap: 8px; min-width: 90px; flex-shrink: 0; }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
  .status-dot { width: 10px; height: 10px; min-width: 10px; border-radius: 50%; margin-top: 3px; flex-shrink: 0; }
  .status-dot--active { animation: pulse 2s ease-in-out infinite; }
  .status-text { display: flex; flex-direction: column; gap: 2px; }
  .status-label { font-size: 12px; font-weight: 600; text-transform: capitalize; white-space: nowrap; }
  .status-duration { font-size: 10px; color: var(--text-secondary); font-family: "SF Mono", monospace; white-space: nowrap; }

  .row-center { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .pipeline-title { font-size: 12px; font-weight: 500; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 100%; }
  .pipeline-meta { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--text-secondary); overflow: hidden; }
  .pipeline-id { font-family: "SF Mono", monospace; font-size: 10px; flex-shrink: 0; }
  .pipeline-ref { font-size: 11px; flex-shrink: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 120px; }
  .pipeline-sha { font-family: "SF Mono", monospace; font-size: 10px; flex-shrink: 0; }

  .row-time { font-size: 11px; color: var(--text-secondary); white-space: nowrap; flex-shrink: 0; margin-top: 2px; }

  .source-badge { font-size: 9px; font-weight: 600; padding: 1px 5px; border-radius: 3px; text-transform: uppercase; letter-spacing: 0.3px; white-space: nowrap; flex-shrink: 0; }
  .source-badge--branch { background: var(--overlay-accent-blue); color: var(--accent-blue); }
  .source-badge--mr { background: var(--overlay-accent-purple); color: var(--accent-purple); }
  .source-badge--schedule { background: var(--overlay-accent-orange); color: var(--accent-orange); }
  .source-badge--gray { background: var(--overlay-accent-muted); color: var(--text-secondary); }
  .source-badge--manual { background: var(--overlay-accent-purple); color: var(--accent-purple); }

  .load-more-btn {
    display: flex; align-items: center; justify-content: center; gap: 8px;
    width: 100%; padding: 12px; background: none; border: none;
    border-top: 1px solid var(--border); color: var(--accent-blue);
    font-size: 12px; cursor: pointer;
  }
  .load-more-btn:hover:not(:disabled) { background: color-mix(in srgb, var(--accent-blue) 5%, transparent); }
  .load-more-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .header-actions { display: flex; gap: 6px; align-items: center; }
  .header-btn {
    background: var(--bg-secondary); color: var(--text-primary);
    border: 1px solid var(--border); border-radius: 4px;
    padding: 4px 10px; font-size: 11px; cursor: pointer;
  }
  .header-btn:hover:not(:disabled) { border-color: var(--accent-blue); color: var(--accent-blue); }
  .header-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .ctx-overlay { position: fixed; inset: 0; z-index: 900; }
  .ctx-menu {
    position: fixed; z-index: 901;
    background: var(--bg-primary); border: 1px solid var(--border);
    border-radius: 4px; padding: 4px 0; min-width: 180px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }
  .ctx-menu button {
    display: block; width: 100%; text-align: left;
    background: none; border: none; color: var(--text-primary);
    padding: 6px 12px; font-size: 12px; cursor: pointer;
  }
  .ctx-menu button:hover { background: color-mix(in srgb, var(--text-primary) 6%, transparent); }
</style>
