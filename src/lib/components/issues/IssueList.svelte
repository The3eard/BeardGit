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
  import { Button, IconButton } from "$lib/components/ui";
  import TwoLineRow from "../common/TwoLineRow.svelte";
  import AssigneeStack from "../common/AssigneeStack.svelte";
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
    <Button variant="primary" size="sm" onclick={() => { showCreateDialog = true; }}>
      {m.issues_create()}
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
      filters={issueFilters}
      bind:tags={searchTags}
      placeholder={m.issues_search_placeholder()}
      onSearch={handleSearch}
    />
  {/snippet}

  {#snippet emptyState()}
    <div class="empty-state">
      {#if !$hasActiveProvider}
        <h3 class="empty-state-title">{m.issues_no_provider()}</h3>
        <p class="empty-state-description">{m.issues_no_provider_description()}</p>
      {:else if $issueList.length > 0 && filteredList.length === 0}
        <h3 class="empty-state-title">{m.issues_no_filter_results()}</h3>
        <p class="empty-state-description">{m.issues_no_filter_description()}</p>
      {:else}
        <h3 class="empty-state-title">{m.issues_empty()}</h3>
        <p class="empty-state-description">{m.issues_empty_description()}</p>
      {/if}
    </div>
  {/snippet}

  {#snippet row({ item, selected })}
    <TwoLineRow {selected}>
      {#snippet leadIcon()}
        {#if item.state === "closed"}
          <span class="state-icon state-icon--closed nf" aria-hidden="true">&#xF00D;</span>
        {:else}
          <span class="state-icon state-icon--open nf" aria-hidden="true">&#xF41E;</span>
        {/if}
      {/snippet}
      {#snippet keyLabel()}
        <span class="issue-number">#{item.number}</span>
      {/snippet}
      {#snippet title()}
        <span class="issue-title-text">{item.title}</span>
      {/snippet}
      {#snippet trailingDate()}
        <span class="row-time">{formatDate(item.created_at)}</span>
      {/snippet}
      {#snippet meta()}
        <span class="issue-author">{item.author}</span>
        {#each item.labels as label (label.name)}
          <span
            class="label-pill"
            style:background={label.color ? `#${label.color}20` : "color-mix(in srgb, var(--text-primary) 10%, transparent)"} /* beardgit:allow-hex: dynamic GitHub API label color */
            style:color={label.color ? `#${label.color}` : "var(--text-secondary)"} /* beardgit:allow-hex: dynamic GitHub API label color */
          >{label.name}</span>
        {/each}
        {#if item.assignees.length > 0}
          <AssigneeStack assignees={item.assignees} max={3} />
        {/if}
        {#if item.milestone}
          <span class="milestone-chip" aria-label={m.issues_milestone_icon_aria()}>
            <span class="nf" aria-hidden="true">{""}</span>{item.milestone.title}
          </span>
        {/if}
        {#if item.comments_count > 0}
          <span class="comment-count">
            <span class="nf" aria-hidden="true">{""}</span>{item.comments_count}
          </span>
        {/if}
      {/snippet}
    </TwoLineRow>
  {/snippet}
</List>

{#if showCreateDialog}
  <CreateIssueDialog onClose={() => { showCreateDialog = false; }} />
{/if}

<style>
  /* The Nerd Font glyph lives in its own inner .nf span — putting `nf`
     on the chip itself inherits the global `width: 1.2em`, squeezing the
     milestone title into a 1-character column that wraps vertically. */
  .milestone-chip {
    font-size: 10px;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .state-icon { font-size: 14px; font-family: var(--font-icons); }
  .state-icon--open { color: var(--accent-green); }
  .state-icon--closed { color: var(--accent-purple); }
  .issue-number { font-family: var(--font-mono); color: var(--text-secondary); font-size: 11px; }
  .issue-author { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .label-pill { padding: 1px 6px; border-radius: 10px; font-size: 10px; }
  .comment-count {
    font-size: 10px;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
  }
  .row-time { font-size: 11px; color: var(--text-secondary); white-space: nowrap; }
  .issue-title-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
