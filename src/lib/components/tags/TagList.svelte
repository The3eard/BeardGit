<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import {
    filteredTags,
    selectedTagName,
    tagsLoading,
    hasMoreTags,
    tagFilter,
    tags,
    selectTag,
    loadMoreTags,
    refreshTags,
    searchTagsBackend,
    restorePreFilterTags,
    doDeleteTag,
    doPushTag,
  } from "../../stores/tags";
  import TagCreateDialog from "./TagCreateDialog.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import { formatRelativeTime } from "../../utils/time";
  import { debounce } from "../../utils/debounce";

  let loadingMore = $state(false);
  let showCreateDialog = $state(false);
  let confirmDelete = $state<string | null>(null);
  let filterValue = $state("");
  let searchingBackend = $state(false);
  const debouncedBackendSearch = debounce(async (value: string) => {
    if (value.length >= 2 && $filteredTags.length === 0) {
      searchingBackend = true;
      await searchTagsBackend(value);
      searchingBackend = false;
    }
  }, 300);

  function handleFilterInput() {
    if (!filterValue) {
      tagFilter.set("");
      restorePreFilterTags();
      searchingBackend = false;
      return;
    }

    tagFilter.set(filterValue);
    debouncedBackendSearch(filterValue);
  }

  async function handleLoadMore() {
    loadingMore = true;
    try {
      await loadMoreTags();
    } finally {
      loadingMore = false;
    }
  }

  function handleRefresh() {
    filterValue = "";
    tagFilter.set("");
    refreshTags();
  }
</script>

<div class="tag-list">
  <div class="list-header">
    <span class="list-title">{m.tags_title()}</span>
    <div class="header-actions">
      <button class="btn-create" onclick={() => (showCreateDialog = true)}>
        {m.tags_create_button()}
      </button>
      <button
        class="refresh-btn nf"
        onclick={handleRefresh}
        disabled={$tagsLoading}
        title="Refresh"
      >
        {$tagsLoading ? "\uF110" : "\uF021"}
      </button>
    </div>
  </div>

  <div class="filter-row">
    <input
      type="text"
      class="filter-input"
      placeholder={m.tags_filter_placeholder()}
      bind:value={filterValue}
      oninput={handleFilterInput}
    />
  </div>

  {#if $tagsLoading && $tags.length === 0}
    <div class="list-loading">
      <div class="spinner"></div>
      <span>{m.tags_title()}...</span>
    </div>
  {:else if $filteredTags.length === 0 && !searchingBackend}
    <div class="list-empty">
      {m.tags_empty()}
    </div>
  {:else if searchingBackend}
    <div class="list-loading">
      <div class="spinner"></div>
      <span>{m.tags_no_results_searching()}</span>
    </div>
  {:else}
    <div class="list-items">
      {#each $filteredTags as tag (tag.name)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="tag-row"
          class:selected={$selectedTagName === tag.name}
          onclick={() => selectTag(tag.name)}
          onkeydown={(e) => { if (e.key === "Enter") selectTag(tag.name); }}
          role="button"
          tabindex="0"
        >
          <div class="tag-top">
            <span class="tag-name">{tag.name}</span>
            <span class="tag-time">
              {tag.date ? formatRelativeTime(tag.date) : ""}
            </span>
          </div>
          <div class="tag-bottom-container">
            <div class="tag-bottom">
              {#if tag.annotated}
                <span class="tag-badge-annotated">{m.tags_badge_annotated()}</span>
              {/if}
              <span class="tag-oid">{tag.commit_oid.slice(0, 8)}</span>
            </div>
            <div class="tag-bottom-hover">
              <div class="tag-actions">
                <button
                  class="action-btn action-btn-push"
                  onclick={(e: MouseEvent) => { e.stopPropagation(); doPushTag(tag.name, "origin"); }}
                >{m.tags_action_push()}</button>
                <button
                  class="action-btn action-btn-danger"
                  onclick={(e: MouseEvent) => { e.stopPropagation(); confirmDelete = tag.name; }}
                >{m.tags_action_delete()}</button>
              </div>
              <span class="tag-oid">{tag.commit_oid.slice(0, 8)}</span>
            </div>
          </div>
        </div>
      {/each}

      {#if $hasMoreTags && !filterValue}
        <button class="load-more-btn" onclick={handleLoadMore} disabled={loadingMore}>
          {#if loadingMore}
            <div class="spinner"></div>
          {:else}
            {m.tags_load_more({ count: String($tags.length) })}
          {/if}
        </button>
      {/if}

      <button class="push-all-btn" onclick={() => doPushTag(null, "origin")}>
        {m.tags_push_all_button()}
      </button>
    </div>
  {/if}
</div>

{#if showCreateDialog}
  <TagCreateDialog onClose={() => (showCreateDialog = false)} />
{/if}

{#if confirmDelete !== null}
  {@const deleteTag = $filteredTags.find((t) => t.name === confirmDelete)}
  <ConfirmDialog
    title={m.tags_delete_dialog_title()}
    detail={deleteTag ? `${deleteTag.name}\n${deleteTag.commit_oid.slice(0, 8)}` : confirmDelete}
    message={m.tags_delete_body({ name: confirmDelete })}
    confirmLabel={m.tags_delete_confirm()}
    destructive={true}
    onConfirm={() => {
      doDeleteTag(confirmDelete!);
      confirmDelete = null;
    }}
    onCancel={() => (confirmDelete = null)}
  />
{/if}

<style>
  .tag-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
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
    gap: 4px;
  }

  .btn-create {
    padding: 3px 10px;
    background: rgba(88, 166, 255, 0.1);
    border: 1px solid rgba(88, 166, 255, 0.3);
    border-radius: 4px;
    color: var(--accent-blue);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-create:hover {
    background: rgba(88, 166, 255, 0.18);
  }

  .filter-row {
    padding: 8px;
  }

  .filter-input {
    width: 100%;
    padding: 5px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    box-sizing: border-box;
  }

  .filter-input:focus {
    border-color: var(--accent-blue);
  }

  .tag-row {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
  }

  .tag-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .tag-row.selected {
    background: rgba(88, 166, 255, 0.08);
  }

  .tag-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .tag-name {
    font-size: 12px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .tag-time {
    font-size: 11px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .tag-bottom-container {
    display: grid;
  }

  .tag-bottom,
  .tag-bottom-hover {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .tag-bottom-hover {
    visibility: hidden;
  }

  .tag-row:hover .tag-bottom {
    visibility: hidden;
  }

  .tag-row:hover .tag-bottom-hover {
    visibility: visible;
  }

  .tag-actions {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .action-btn {
    padding: 2px 8px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    font-size: 10px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .action-btn-push:hover {
    background: rgba(63, 185, 80, 0.15);
    border-color: var(--accent-green);
    color: var(--accent-green);
  }

  .action-btn-danger:hover {
    background: rgba(248, 81, 73, 0.2);
    border-color: var(--accent-red);
    color: var(--accent-red);
  }

  .push-all-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    padding: 10px;
    background: none;
    border: none;
    border-top: 1px solid var(--border);
    color: var(--accent-blue);
    font-size: 11px;
    cursor: pointer;
    opacity: 0.7;
  }

  .push-all-btn:hover {
    background: rgba(88, 166, 255, 0.05);
    opacity: 1;
  }

  .tag-badge-annotated {
    font-size: 9px;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 3px;
    background: rgba(240, 136, 62, 0.15);
    color: #f0883e;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .tag-oid {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .load-more-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 12px;
    background: none;
    border: none;
    border-top: 1px solid var(--border);
    color: var(--accent-blue);
    font-size: 12px;
    cursor: pointer;
  }

  .load-more-btn:hover:not(:disabled) {
    background: rgba(88, 166, 255, 0.05);
  }

  .load-more-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
