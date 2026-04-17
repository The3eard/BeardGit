<!--
  IssueList — list of issues with search/filter bar and create button.
-->
<script lang="ts">
  import {
    issueList,
    issueListLoading,
    issueStateFilter,
    selectedIssueNumber,
    refreshIssueList,
    loadIssueDetail,
  } from "../../stores/issues";
  import { hasActiveProvider } from "../../stores/provider";
  import * as m from "$lib/paraglide/messages";
  import type { Issue, IssueState } from "../../types";
  import CreateIssueDialog from "./CreateIssueDialog.svelte";
  import SearchBar from "../common/SearchBar.svelte";
  import List from "../common/List.svelte";
  import type { SearchTag } from "../../search/types";
  import { issueFilters, filterIssuesLocal } from "../../search/issue-provider";

  let showCreateDialog = $state(false);
  let loading = $state(false);
  let initialized = false;

  let searchTags = $state<SearchTag[]>([
    {
      id: `init-${Date.now()}`,
      type: "state",
      value: "open",
      display: "state:open",
    },
  ]);

  let filteredList = $derived(filterIssuesLocal($issueList, searchTags));

  function syncStateFilter(tags: SearchTag[]) {
    const stateTag = tags.find((t) => t.type === "state");
    const v = stateTag?.value.toLowerCase();
    if (v === "open" || v === "closed") {
      issueStateFilter.set(v as IssueState);
    } else {
      issueStateFilter.set("all");
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
    void $issueStateFilter;
    if (initialized) fetchList();
  });

  async function fetchList() {
    if (!$hasActiveProvider) return;
    loading = true;
    await refreshIssueList();
    loading = false;
  }

  function getKey(item: Issue): string {
    return String(item.number);
  }

  let selectedKey = $derived(
    $selectedIssueNumber !== null ? String($selectedIssueNumber) : null,
  );

  function handleSelect(item: Issue) {
    loadIssueDetail(item.number);
  }

  function formatDate(iso: string): string {
    if (!iso) return "";
    const d = new Date(iso);
    const days = Math.floor((Date.now() - d.getTime()) / 86400000);
    if (days === 0) return "today";
    if (days === 1) return "yesterday";
    if (days < 30) return `${days}d ago`;
    return d.toLocaleDateString();
  }
</script>

<List
  items={!$hasActiveProvider ? [] : filteredList}
  loading={$issueListLoading}
  title={m.issues_title()}
  {selectedKey}
  {getKey}
  emptyMessage={m.issues_empty()}
  onSelect={handleSelect}
  onRefresh={fetchList}
>
  {#snippet headerActions()}
    <button class="action-btn-create" onclick={() => { showCreateDialog = true; }}>
      {m.issues_create()}
    </button>
    <button class="refresh-btn nf" onclick={fetchList} disabled={loading} title="Refresh">
      {loading ? "\uF110" : "\uF021"}
    </button>
  {/snippet}

  {#snippet afterHeader()}
    <SearchBar
      filters={issueFilters}
      bind:tags={searchTags}
      placeholder={m.issues_search_placeholder()}
      onSearch={handleSearch}
    />
  {/snippet}

  {#snippet emptyState()}
    {#if !$hasActiveProvider}
      <div class="empty-state">{m.issues_no_provider()}</div>
    {:else if $issueList.length > 0 && filteredList.length === 0}
      <div class="empty-state">{m.issues_no_filter_results()}</div>
    {:else}
      <div class="empty-state">{m.issues_empty()}</div>
    {/if}
  {/snippet}

  {#snippet row({ item })}
    <div class="row-status">
      {#if item.state === "closed"}
        <span class="state-icon state-icon--closed nf">&#xF00D;</span>
      {:else}
        <span class="state-icon state-icon--open nf">&#xF41E;</span>
      {/if}
    </div>
    <div class="row-center">
      <div class="row-title">
        <span class="issue-number">#{item.number}</span>
        <span class="issue-title-text">{item.title}</span>
      </div>
      <div class="row-meta">
        <span class="issue-author">{item.author}</span>
        {#if item.labels.length > 0}
          <span class="label-pills">
            {#each item.labels.slice(0, 3) as label}
              <span
                class="label-pill"
                style:background={label.color ? `#${label.color}20` : "rgba(255,255,255,0.1)"}
                style:color={label.color ? `#${label.color}` : "var(--text-secondary)"}
              >{label.name}</span>
            {/each}
            {#if item.labels.length > 3}
              <span class="label-overflow">+{item.labels.length - 3}</span>
            {/if}
          </span>
        {/if}
        {#if item.comments_count > 0}
          <span class="comment-count nf">{"\uF075"} {item.comments_count}</span>
        {/if}
      </div>
    </div>
    <div class="row-time">{formatDate(item.created_at)}</div>
  {/snippet}
</List>

{#if showCreateDialog}
  <CreateIssueDialog onClose={() => { showCreateDialog = false; }} />
{/if}

<style>
  .action-btn-create {
    padding: 4px 10px;
    background: var(--accent-blue);
    color: #fff;
    border: none;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
  }
  .action-btn-create:hover { opacity: 0.9; }

  .empty-state {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
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
  .state-icon--closed { color: var(--accent-purple); }

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
  .issue-number {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    flex-shrink: 0;
  }
  .issue-title-text {
    font-size: 12px;
    font-weight: 500;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-meta {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
  }
  .issue-author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .label-pills { display: flex; gap: 4px; }
  .label-pill {
    padding: 1px 6px;
    border-radius: 10px;
    font-size: 10px;
  }
  .label-overflow {
    font-size: 10px;
    color: var(--text-secondary);
  }
  .comment-count {
    font-size: 10px;
    color: var(--text-secondary);
  }
  .row-time {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
    margin-top: 2px;
  }
</style>
