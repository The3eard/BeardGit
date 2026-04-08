<script lang="ts">
  import {
    mrPrList,
    mrPrListLoading,
    mrPrFilter,
    selectedMrPrNumber,
    refreshMrPrList,
    loadMrPrDetail,
  } from "../../stores/mr-pr";
  import { activeProvider } from "../../stores/provider";
  import * as m from "$lib/paraglide/messages";
  import type { MrPrState } from "../../types";

  $effect(() => {
    refreshMrPrList();
  });

  // Re-fetch when filter changes
  $effect(() => {
    void $mrPrFilter;
    refreshMrPrList();
  });

  let isGitHub = $derived($activeProvider?.kind === "github");
  let title = $derived(isGitHub ? m.mrpr_title_github() : m.mrpr_title());
  let emptyMessage = $derived(isGitHub ? m.mrpr_empty_github() : m.mrpr_empty());

  type FilterTab = MrPrState | "all";
  const filters: { label: () => string; value: FilterTab }[] = [
    { label: () => m.mrpr_filter_open(), value: "open" },
    { label: () => m.mrpr_filter_closed(), value: "closed" },
    { label: () => m.mrpr_filter_merged(), value: "merged" },
    { label: () => m.mrpr_filter_all(), value: "all" },
  ];

  function handleSelect(number: number) {
    loadMrPrDetail(number);
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
    <h2 class="list-title">{title}</h2>
  </div>

  <div class="filter-tabs">
    {#each filters as tab}
      <button
        class="filter-tab"
        class:active={$mrPrFilter === tab.value}
        onclick={() => mrPrFilter.set(tab.value)}
      >
        {tab.label()}
      </button>
    {/each}
  </div>

  {#if $mrPrListLoading}
    <div class="empty-state">{m.mrpr_loading()}</div>
  {:else if $mrPrList.length === 0}
    <div class="empty-state">{emptyMessage}</div>
  {:else}
    <div class="list-scroll">
      {#each $mrPrList as item}
        <button
          class="mrpr-row"
          class:selected={$selectedMrPrNumber === item.number}
          onclick={() => handleSelect(item.number)}
        >
          <div class="row-main">
            <span class="mrpr-number">#{item.number}</span>
            <span class="mrpr-title-text">{item.title}</span>
            {#if item.draft}
              <span class="draft-badge">{m.mrpr_draft()}</span>
            {/if}
          </div>
          <div class="row-meta">
            <span class="mrpr-branch">{item.source_branch}</span>
            <span class="mrpr-author">{item.author}</span>
            <span class="mrpr-date">{formatDate(item.created_at)}</span>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .mrpr-list-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .list-header {
    padding: 12px 16px 0;
  }

  .list-title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .filter-tabs {
    display: flex;
    gap: 0;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
  }

  .filter-tab {
    padding: 4px 12px;
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
  }

  .filter-tab:first-child {
    border-radius: 4px 0 0 4px;
  }
  .filter-tab:last-child {
    border-radius: 0 4px 4px 0;
  }
  .filter-tab:not(:first-child) {
    border-left: none;
  }

  .filter-tab.active {
    background: var(--accent-blue);
    color: #ffffff;
    border-color: var(--accent-blue);
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
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 8px 16px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
  }

  .mrpr-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }
  .mrpr-row.selected {
    background: rgba(88, 166, 255, 0.08);
  }

  .row-main {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .mrpr-number {
    font-size: 12px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }

  .mrpr-title-text {
    font-size: 13px;
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
  }

  .row-meta {
    display: flex;
    gap: 8px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .mrpr-branch {
    font-family: var(--font-mono);
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
