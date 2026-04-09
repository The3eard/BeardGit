<!--
  MrPrList — list of merge requests / pull requests with search/filter
  bar and a "New MR/PR" create button.
-->
<script lang="ts">
  import {
    mrPrList,
    mrPrListLoading,
    mrPrFilter,
    selectedMrPrNumber,
    refreshMrPrList,
    loadMrPrDetail,
  } from "../../stores/mr-pr";
  import { activeProvider, hasActiveProvider } from "../../stores/provider";
  import * as m from "$lib/paraglide/messages";
  import type { MrPrState } from "../../types";
  import CreateMrPrDialog from "./CreateMrPrDialog.svelte";
  import SearchBar from "../common/SearchBar.svelte";
  import type { SearchTag } from "../../search/types";
  import { mrFilters, filterMrPrLocal } from "../../search/mr-provider";

  let showCreateDialog = $state(false);
  let loading = $state(false);
  let initialized = false;

  // Initialize with default "state:open" tag
  let searchTags = $state<SearchTag[]>([{
    id: `init-${Date.now()}`,
    type: "state",
    value: "open",
    display: "state:open",
  }]);

  let filteredList = $derived(filterMrPrLocal($mrPrList, searchTags));

  /** Extract state filter from tags and sync with mrPrFilter store. */
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

  // Wait for provider to be ready, then fetch
  $effect(() => {
    if ($hasActiveProvider && !initialized) {
      initialized = true;
      fetchList();
    }
  });

  // Re-fetch when filter changes (skip initial — handled by provider effect)
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

  function handleSelect(number: number) {
    loadMrPrDetail(number);
  }

  function refresh() {
    fetchList();
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

<div class="mrpr-list-container">
  <div class="list-header">
    <span class="list-title">{title}</span>
    <div class="header-actions">
      <button class="action-btn" onclick={() => { showCreateDialog = true; }}>
        {isGitHub ? m.mrpr_create_github() : m.mrpr_create()}
      </button>
      <button class="refresh-btn nf" onclick={refresh} disabled={loading} title="Refresh">
        {loading ? "\uF110" : "\uF021"}
      </button>
    </div>
  </div>

  <SearchBar
    filters={mrFilters}
    bind:tags={searchTags}
    placeholder={m.mrpr_search_placeholder()}
    onSearch={handleSearch}
  />

  {#if $mrPrListLoading}
    <div class="empty-state">{m.mrpr_loading()}</div>
  {:else if !$hasActiveProvider}
    <div class="empty-state">{m.mrpr_no_provider()}</div>
  {:else if $mrPrList.length === 0}
    <div class="empty-state">{emptyMessage}</div>
  {:else if filteredList.length === 0}
    <div class="empty-state">{m.mrpr_no_filter_results()}</div>
  {:else}
    <div class="list-scroll">
      {#each filteredList as item}
        <button
          class="mrpr-row"
          class:selected={$selectedMrPrNumber === item.number}
          onclick={() => handleSelect(item.number)}
        >
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
        </button>
      {/each}
    </div>
  {/if}
</div>

{#if showCreateDialog}
  <CreateMrPrDialog onClose={() => { showCreateDialog = false; }} />
{/if}

<style>
  .mrpr-list-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
  }

  .list-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .action-btn {
    padding: 4px 10px;
    background: var(--accent-blue);
    color: #ffffff;
    border: none;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
  }

  .action-btn:hover {
    opacity: 0.9;
  }



  .empty-state {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .list-scroll {
    flex: 1;
    overflow-y: auto;
  }

  .mrpr-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
  }

  .mrpr-row:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .mrpr-row.selected {
    background: color-mix(in srgb, var(--accent-blue) 8%, transparent);
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

  .state-icon--open {
    color: var(--accent-green);
  }

  .state-icon--merged {
    color: var(--accent-purple);
  }

  .state-icon--closed {
    color: var(--accent-red);
  }

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
    background: rgba(255, 255, 255, 0.1);
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
