<!--
  ReleaseList — scrollable list of releases with state badge, tag, name,
  asset count, and publication date. The "New release" button in the header
  opens CreateReleaseDialog.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import List from "../common/List.svelte";
  import {
    releases,
    releasesLoading,
    selectedReleaseTag,
    refreshReleases,
    selectRelease,
  } from "../../stores/releases";
  import { hasActiveProvider } from "../../stores/provider";
  import { formatRelativeTime } from "../../utils/time";
  import * as m from "$lib/paraglide/messages";
  import CreateReleaseDialog from "./CreateReleaseDialog.svelte";
  import { Button } from "$lib/components/ui";
  import type { Release } from "../../types";

  let showCreate = $state(false);
  let initialized = false;

  $effect(() => {
    if ($hasActiveProvider && !initialized) {
      initialized = true;
      void refreshReleases();
    }
  });

  onMount(() => {
    if ($hasActiveProvider && !initialized) {
      initialized = true;
      void refreshReleases();
    }
  });

  function filterFn(item: Release, q: string): boolean {
    const needle = q.toLowerCase();
    return (
      item.tag.toLowerCase().includes(needle) ||
      item.name.toLowerCase().includes(needle) ||
      item.author.toLowerCase().includes(needle)
    );
  }

  function stateLabel(s: Release["state"]): string {
    if (s === "draft") return m.release_state_draft();
    if (s === "prerelease") return m.release_state_prerelease();
    return m.release_state_published();
  }

  function getKey(r: Release): string {
    return r.tag;
  }

  function handleSelect(r: Release): void {
    selectRelease(r.tag);
  }
</script>

<List
  items={!$hasActiveProvider ? [] : $releases}
  loading={$releasesLoading}
  title={m.sidebar_releases()}
  selectedKey={$selectedReleaseTag}
  {getKey}
  {filterFn}
  filterPlaceholder={m.release_filter_placeholder()}
  emptyMessage={m.release_list_empty()}
  onSelect={handleSelect}
  onRefresh={refreshReleases}
>
  {#snippet headerActions()}
    <Button
      variant="primary"
      size="sm"
      onclick={() => (showCreate = true)}
      description={m.release_new_button()}
    >
      {m.release_new_button()}
    </Button>
  {/snippet}

  {#snippet emptyState()}
    <div class="empty-state">
      {#if !$hasActiveProvider}
        <h3 class="empty-state-title">{m.release_no_provider()}</h3>
        <p class="empty-state-description">{m.release_no_provider_description()}</p>
      {:else}
        <h3 class="empty-state-title">{m.release_list_empty()}</h3>
        <p class="empty-state-description">{m.release_list_empty_description()}</p>
      {/if}
    </div>
  {/snippet}

  {#snippet row({ item })}
    <div class="release-row">
      <span class="badge badge-{item.state}">{stateLabel(item.state)}</span>
      <div class="release-center">
        <div class="release-title">
          <span class="tag">{item.tag}</span>
          {#if item.name && item.name !== item.tag}
            <span class="name">{item.name}</span>
          {/if}
        </div>
        <div class="release-meta">
          <span class="author">{item.author}</span>
          {#if item.asset_count > 0}
            <span class="asset-count" title="assets">
              <span class="nf">{"\uF187"}</span>
              {item.asset_count}
            </span>
          {/if}
        </div>
      </div>
      <span class="date">
        {item.published_at
          ? formatRelativeTime(item.published_at)
          : m.release_state_draft()}
      </span>
    </div>
  {/snippet}
</List>

{#if showCreate}
  <CreateReleaseDialog onClose={() => (showCreate = false)} />
{/if}

<style>
  .release-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    overflow: hidden;
  }
  .release-center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .release-title {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
  }
  .tag {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    flex-shrink: 0;
  }
  .name {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .release-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }
  .author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .asset-count {
    display: inline-flex;
    gap: 3px;
    align-items: center;
    font-size: var(--font-size-xs);
  }
  .asset-count .nf {
    font-family: var(--font-icons);
  }
  .date {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .badge {
    font-size: 9px;
    padding: 1px 6px;
    border-radius: 3px;
    font-weight: 600;
    text-transform: uppercase;
    flex-shrink: 0;
    letter-spacing: 0.3px;
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
</style>
