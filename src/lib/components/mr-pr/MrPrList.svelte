<!--
  MrPrList — list of merge requests / pull requests with search/filter
  bar and a "New MR/PR" create button.
-->
<script lang="ts">
  import {
    mrPrList,
    mrPrListLoading,
    mrPrListError,
    mrPrFilter,
    selectedMrPrNumber,
    refreshMrPrList,
    loadMrPrDetail,
  } from "../../stores/mr-pr";
  import { activeProvider, hasActiveProvider } from "../../stores/provider";
  import * as m from "$lib/paraglide/messages";
  import type { MrPr, MrPrState } from "../../types";
  import CreateMrPrDialog from "./CreateMrPrDialog.svelte";
  import SearchBar from "../common/SearchBar.svelte";
  import List from "../common/List.svelte";
  import { Button, IconButton } from "$lib/components/ui";
  import type { SearchTag } from "../../search/types";
  import { mrFilters, filterMrPrLocal } from "../../search/mr-provider";

  let showCreateDialog = $state(false);
  let loading = $state(false);
  let initialized = false;

  let searchTags = $state<SearchTag[]>([{
    id: `init-${Date.now()}`,
    type: "state",
    value: "open",
    display: "state:open",
  }]);

  let filteredList = $derived(filterMrPrLocal($mrPrList, searchTags));

  function syncStateFilter(tags: SearchTag[]) {
    const stateTag = tags.find(t => t.type === "state");
    const stateValue = stateTag?.value.toLowerCase();
    if (stateValue === "open" || stateValue === "closed" || stateValue === "merged") {
      mrPrFilter.set(stateValue as MrPrState);
    } else {
      mrPrFilter.set("all");
    }
  }

  function handleSearch(tags: SearchTag[]) {
    searchTags = tags;
    syncStateFilter(tags);
    fetchList();
  }

  $effect(() => {
    if ($hasActiveProvider && !initialized) {
      initialized = true;
      fetchList();
    }
  });

  $effect(() => {
    void $mrPrFilter;
    if (initialized) {
      fetchList();
    }
  });

  async function fetchList() {
    if (!$hasActiveProvider) return;
    loading = true;
    await refreshMrPrList();
    loading = false;
  }

  let isGitHub = $derived($activeProvider?.kind === "github");
  let title = $derived(isGitHub ? m.mrpr_title_github() : m.mrpr_title());
  let emptyMessage = $derived(isGitHub ? m.mrpr_empty_github() : m.mrpr_empty());

  function getKey(item: MrPr): string {
    return String(item.number);
  }

  let selectedKey = $derived(
    $selectedMrPrNumber !== null ? String($selectedMrPrNumber) : null,
  );

  function handleSelect(item: MrPr) {
    loadMrPrDetail(item.number);
  }

  function formatDate(iso: string): string {
    if (!iso) return "";
    const d = new Date(iso);
    const now = new Date();
    const diff = now.getTime() - d.getTime();
    const days = Math.floor(diff / 86400000);
    if (days === 0) return "today";
    if (days === 1) return "yesterday";
    if (days < 30) return `${days}d ago`;
    return d.toLocaleDateString();
  }
</script>

<List
  items={!$hasActiveProvider ? [] : filteredList}
  loading={$mrPrListLoading}
  {title}
  {selectedKey}
  {getKey}
  {emptyMessage}
  onSelect={handleSelect}
  onRefresh={fetchList}
>
  {#snippet headerActions()}
    <Button variant="primary" size="sm" onclick={() => { showCreateDialog = true; }}>
      {isGitHub ? m.mrpr_create_github() : m.mrpr_create()}
    </Button>
    <IconButton
      icon={"\uF021"}
      description={m.tooltip_refresh()}
      loading={loading}
      onclick={fetchList}
    />
  {/snippet}

  {#snippet afterHeader()}
    <SearchBar
      filters={mrFilters}
      bind:tags={searchTags}
      placeholder={m.mrpr_search_placeholder()}
      onSearch={handleSearch}
    />
  {/snippet}

  {#snippet emptyState()}
    {#if !$hasActiveProvider}
      <div class="empty-state">{m.mrpr_no_provider()}</div>
    {:else if $mrPrListError}
      <div class="empty-state empty-state--error">
        <div class="error-title">{m.mrpr_error_title()}</div>
        <pre class="error-message">{$mrPrListError}</pre>
        <button class="retry-btn" onclick={fetchList}>{m.mrpr_error_retry()}</button>
      </div>
    {:else if $mrPrList.length > 0 && filteredList.length === 0}
      <div class="empty-state">{m.mrpr_no_filter_results()}</div>
    {:else}
      <div class="empty-state">{emptyMessage}</div>
    {/if}
  {/snippet}

  {#snippet row({ item })}
    <div class="row-status">
      {#if item.state === "merged"}
        <span class="state-icon state-icon--merged nf">&#xF126;</span>
      {:else if item.state === "closed"}
        <span class="state-icon state-icon--closed nf">&#xF00D;</span>
      {:else}
        <span class="state-icon state-icon--open nf">&#xF41E;</span>
      {/if}
    </div>
    <div class="row-center">
      <div class="row-title">
        <span class="mrpr-number">#{item.number}</span>
        <span class="mrpr-title-text">{item.title}</span>
        {#if item.draft}
          <span class="draft-badge">{m.mrpr_draft()}</span>
        {/if}
      </div>
      <div class="row-meta">
        <span class="mrpr-branch">{item.source_branch}</span>
        <span class="mrpr-author">{item.author}</span>
      </div>
    </div>
    <div class="row-time">
      {formatDate(item.created_at)}
    </div>
  {/snippet}
</List>

{#if showCreateDialog}
  <CreateMrPrDialog onClose={() => { showCreateDialog = false; }} />
{/if}

<style>
  .empty-state {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .empty-state--error {
    padding: 24px 16px;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .error-title {
    color: var(--accent-red);
    font-weight: 600;
    font-size: 13px;
  }

  .error-message {
    margin: 0;
    padding: 10px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-mono, monospace);
    font-size: 11.5px;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 180px;
    overflow: auto;
  }

  .retry-btn {
    align-self: flex-start;
    padding: 5px 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }

  .retry-btn:hover {
    background: var(--bg-hover);
  }

  .row-status {
    display: flex;
    align-items: flex-start;
    min-width: 28px;
    flex-shrink: 0;
    padding-top: 1px;
  }

  .state-icon {
    font-size: 14px;
    font-family: var(--font-icons);
  }

  .state-icon--open { color: var(--accent-green); }
  .state-icon--merged { color: var(--accent-purple); }
  .state-icon--closed { color: var(--accent-red); }

  .row-center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .row-title {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
  }

  .mrpr-number {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .mrpr-title-text {
    font-size: 12px;
    font-weight: 500;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .draft-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--text-primary) 10%, transparent);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .row-meta {
    display: flex;
    gap: 8px;
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
  }

  .mrpr-branch {
    font-family: var(--font-mono);
    font-size: 10px;
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .mrpr-author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-time {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
    margin-top: 2px;
  }
</style>
