<!--
  PipelineList — list of CI runs with search/filter, trigger dialog, and
  per-row context menu for retry/cancel/open. Since the v2 migration the
  scaffolding (header, search bar, empty state, load-more, polling bar)
  lives in the shared <List> component; this file owns the pipeline-specific
  row template, polling lifecycle, context menu, and trigger dialog.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { ciRuns, loadCiRuns, loadMoreCiRuns, loadCiRunDetail, selectedCiRunId, startCiRunListPolling, stopCiRunListPolling, hasMoreCiRuns, hasActiveProvider, retryCiRun, cancelCiRun } from "../../stores/provider";
  import type { CiRun } from "../../types";
  import { repoInfo } from "../../stores/repo";
  import SearchBar from "../common/SearchBar.svelte";
  import List from "../common/List.svelte";
  import TwoLineRow from "../common/TwoLineRow.svelte";
  import { Button, IconButton } from "$lib/components/ui";
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
    if ($hasActiveProvider) initAndLoad();
    return () => stopCiRunListPolling();
  });

  $effect(() => {
    if ($hasActiveProvider && !initialized) initAndLoad();
  });

  async function initAndLoad() {
    if (initialized) return;
    initialized = true;
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

  function refresh() { fetchPipelines(); }

  function handleSearch(tags: SearchTag[]) {
    searchTags = tags;
    initialized = false;
    initialized = true;
    fetchPipelines();
  }

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
    try { await loadCiRunDetail(run.id); } catch (e) { error = String(e); }
  }

  function shortSha(sha: string): string { return sha.substring(0, 8); }

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

  function showDuration(run: CiRun): boolean {
    return run.status === "success" || run.status === "failed" || run.status === "timed_out" || run.status === "canceled";
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

  function isActiveStatus(s: string): boolean {
    return s === "running" || s === "pending" || s === "queued";
  }

  let selectedId = $derived($selectedCiRunId);
</script>

<List
  items={!$hasActiveProvider ? [] : filteredPipelines}
  loading={loading && $ciRuns.length === 0}
  refreshing={loading && $ciRuns.length > 0}
  title={m.pipeline_title()}
  selectedKey={selectedId !== null ? String(selectedId) : null}
  getKey={(r) => String(r.id)}
  onSelect={selectCiRun}
  onRefresh={refresh}
  onContextMenu={onRowContextMenu}
>
  {#snippet headerActions()}
    <Button
      variant="primary"
      size="sm"
      onclick={() => (triggerDialogOpen = true)}
      disabled={!$hasActiveProvider}
    >{m.pipeline_action_trigger()}</Button>
    <IconButton
      icon={"\uF021"}
      description={m.tooltip_refresh()}
      loading={loading}
      onclick={refresh}
    />
  {/snippet}

  {#snippet afterHeader()}
    <SearchBar
      filters={ciFilters}
      bind:tags={searchTags}
      placeholder={m.pipeline_search_placeholder()}
      onSearch={handleSearch}
    />
  {/snippet}

  {#snippet emptyState()}
    <div class="empty-state">
      {#if !$hasActiveProvider}
        <h3 class="empty-state-title">{m.pipeline_no_provider()}</h3>
        <p class="empty-state-description">{m.pipeline_no_provider_description()}</p>
      {:else if $ciRuns.length === 0}
        <h3 class="empty-state-title">{m.pipeline_no_runs()}</h3>
        <p class="empty-state-description">{m.pipeline_no_runs_description()}</p>
      {:else}
        <h3 class="empty-state-title">{m.pipeline_no_match()}</h3>
        <p class="empty-state-description">{m.pipeline_no_match_description()}</p>
      {/if}
    </div>
  {/snippet}

  {#snippet row({ item, selected })}
    <TwoLineRow {selected}>
      {#snippet leadIcon()}
        <span
          class="status-dot"
          class:status-dot--active={isActiveStatus(item.status)}
          style:background={ciStatusColor(item.status)}
        ></span>
      {/snippet}
      {#snippet keyLabel()}
        <span class="status-label" style:color={ciStatusColor(item.status)}>
          {ciStatusLabel(item.status)}
        </span>
      {/snippet}
      {#snippet title()}
        <span class="pipeline-title">
          {item.name ?? m.pipeline_run_title({ id: String(item.display_id) })}
        </span>
      {/snippet}
      {#snippet trailingDate()}
        <span class="row-time">{formatRelativeTime(item.created_at)}</span>
      {/snippet}
      {#snippet meta()}
        <span class="pipeline-id">#{item.display_id}</span>
        <span class="pipeline-ref" title={item.ref_name}>{item.ref_name}</span>
        <span class="pipeline-sha">{shortSha(item.sha)}</span>
        {#if item.source}
          <span class="source-badge {sourceBadgeClass(item.source)}">
            {sourceLabel(item.source)}
          </span>
        {/if}
        {#if item.actor}
          <span class="pipeline-actor" aria-label={m.pipeline_actor_aria()}>{item.actor}</span>
        {/if}
        {#if showDuration(item)}
          {@const dur = formatDuration(item.created_at, item.updated_at)}
          {#if dur}
            <span class="status-duration nf">{""} {dur}</span>
          {/if}
        {/if}
      {/snippet}
    </TwoLineRow>
  {/snippet}

  {#snippet footer()}
    {#if $hasMoreCiRuns}
      <Button variant="neutral" size="sm" onclick={handleLoadMore} disabled={loadingMore} loading={loadingMore}>
        {#if !loadingMore}{m.pipeline_load_more({ count: String($ciRuns.length) })}{/if}
      </Button>
    {/if}
  {/snippet}
</List>

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

{#if ctxError}<div class="list-error" role="alert">{ctxError}</div>{/if}
{#if error}<div class="list-error" role="alert">{error}</div>{/if}

<style>
  /* Scaffolding styles (.pipeline-list, .list-header, .list-loading-bar, .empty-state)
     now live in List.svelte / app.css. Keep only what's specific to the pipeline row. */

  .list-error { padding: 8px 12px; font-size: 12px; color: var(--accent-red); background: var(--overlay-accent-red); margin: 8px; border-radius: 4px; word-break: break-word; }

  .status-dot { width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
  .status-dot--active { animation: pulse 2s ease-in-out infinite; }
  .status-label { font-size: 11px; font-weight: 600; text-transform: capitalize; white-space: nowrap; }

  .pipeline-title { font-size: 12px; font-weight: 500; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .pipeline-id { font-family: var(--font-mono); font-size: 10px; flex-shrink: 0; }
  .pipeline-ref {
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 220px; /* decided in plan §"Resolved open questions" */
  }
  .pipeline-sha { font-family: var(--font-mono); font-size: 10px; flex-shrink: 0; }
  .pipeline-actor { font-size: 10px; color: var(--text-secondary); flex-shrink: 0; }
  .status-duration { font-size: 10px; color: var(--text-secondary); font-family: var(--font-mono); flex-shrink: 0; }

  .row-time { font-size: 11px; color: var(--text-secondary); white-space: nowrap; }

  .source-badge { font-size: 9px; font-weight: 600; padding: 1px 5px; border-radius: 3px; text-transform: uppercase; letter-spacing: 0.3px; white-space: nowrap; flex-shrink: 0; }
  .source-badge--branch { background: var(--overlay-accent-blue); color: var(--accent-blue); }
  .source-badge--mr { background: var(--overlay-accent-purple); color: var(--accent-purple); }
  .source-badge--schedule { background: var(--overlay-accent-orange); color: var(--accent-orange); }
  .source-badge--gray { background: var(--overlay-accent-muted); color: var(--text-secondary); }
  .source-badge--manual { background: var(--overlay-accent-purple); color: var(--accent-purple); }

  .ctx-overlay { position: fixed; inset: 0; z-index: 900; }
  .ctx-menu { position: fixed; z-index: 901; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 4px; padding: 4px 0; min-width: 180px; box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3); /* beardgit:allow-hex: shadow neutral always-dark */ }
  .ctx-menu button { display: block; width: 100%; text-align: left; background: none; border: none; color: var(--text-primary); padding: 6px 12px; font-size: 12px; cursor: pointer; }
  .ctx-menu button:hover { background: color-mix(in srgb, var(--text-primary) 6%, transparent); }
</style>
