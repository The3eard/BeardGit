<!--
  Generic list panel with header, optional filter bar, scrollable items,
  loading/empty states, and single-item keyboard selection.

  Consumers provide layout via Svelte 5 snippets: row, headerActions,
  and emptyState. Right-click is exposed via the onContextMenu callback.
-->
<script lang="ts" generics="T">
  import type { Snippet } from "svelte";
  import { debounce } from "../../utils/debounce";

  interface Props {
    /** Items to display. */
    items: T[];
    /** Whether data is loading. */
    loading: boolean;
    /** Panel title displayed in the header. */
    title: string;
    /** Key of the currently selected item, or null. */
    selectedKey: string | null;
    /** Extract a unique string key from an item. */
    getKey: (item: T) => string;
    /** Optional client-side filter function. */
    filterFn?: (item: T, query: string) => boolean;
    /** Placeholder for the filter input. */
    filterPlaceholder?: string;
    /** Message shown when items is empty and not loading. */
    emptyMessage?: string;
    /** Fired when the user clicks/selects an item. */
    onSelect?: (item: T) => void;
    /** Fired when the user clicks the refresh button. */
    onRefresh?: () => void;
    /** Fired on right-click of a row. */
    onContextMenu?: (e: MouseEvent, item: T) => void;
    /** Fired when the user double-clicks a row. */
    onDoubleClick?: (item: T) => void;
    /** Debounce delay for filter input (ms). Default 150. */
    filterDelay?: number;
    /** Row snippet — renders each item. Receives { item, selected }. Required unless customContent is provided. */
    row?: Snippet<[{ item: T; selected: boolean }]>;
    /** Header actions snippet — right side of header. */
    headerActions?: Snippet;
    /** Custom empty state snippet. */
    emptyState?: Snippet;
    /** Optional content rendered between header and items (e.g., a search bar). */
    afterHeader?: Snippet;
    /** Optional footer rendered after all items inside the scroll container. */
    footer?: Snippet;
    /** When provided, replaces the entire items area. Loading/empty/each are skipped. */
    customContent?: Snippet;
  }

  let {
    items,
    loading,
    title,
    selectedKey,
    getKey,
    filterFn,
    filterPlaceholder = "Filter...",
    emptyMessage = "No items",
    onSelect,
    onRefresh,
    onContextMenu,
    onDoubleClick,
    filterDelay = 150,
    row,
    headerActions,
    emptyState,
    afterHeader,
    footer,
    customContent,
  }: Props = $props();

  // ── Filter state ──────────────────────────────────────────────────────
  let filterInput = $state("");
  let filterQuery = $state("");

  // svelte-ignore state_referenced_locally
  const applyFilter = debounce((value: string) => {
    filterQuery = value;
  }, filterDelay);

  function onFilterInput(value: string) {
    filterInput = value;
    applyFilter(value);
  }

  let filteredItems = $derived(
    filterFn && filterQuery
      ? items.filter((item) => filterFn!(item, filterQuery))
      : items,
  );

  // ── Keyboard navigation ───────────────────────────────────────────────
  let listEl: HTMLDivElement | undefined = $state();

  function handleKeydown(e: KeyboardEvent) {
    if (!filteredItems.length) return;

    const currentIndex = selectedKey
      ? filteredItems.findIndex((item) => getKey(item) === selectedKey)
      : -1;

    let targetIndex: number | null = null;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      targetIndex = Math.min(currentIndex + 1, filteredItems.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      targetIndex = Math.max(currentIndex - 1, 0);
    }

    if (targetIndex !== null) {
      onSelect?.(filteredItems[targetIndex]);
      // Scroll the newly-selected row into view on the next frame.
      requestAnimationFrame(() => {
        const rows = listEl?.querySelectorAll<HTMLElement>(".list-row");
        rows?.[targetIndex!]?.scrollIntoView({ block: "nearest" });
      });
    }
  }

  /** Reset filter when onRefresh is called externally. */
  export function resetFilter() {
    filterInput = "";
    filterQuery = "";
  }
</script>

<div
  class="list-panel"
  role="listbox"
  tabindex="0"
  onkeydown={handleKeydown}
>
  <!-- Header -->
  <div class="list-header">
    <span class="list-title">{title}</span>
    {#if headerActions}
      <div class="header-actions">
        {@render headerActions()}
      </div>
    {:else if onRefresh}
      <button
        class="refresh-btn nf"
        onclick={onRefresh}
        disabled={loading}
        title="Refresh"
      >
        {loading ? "\uF110" : "\uF021"}
      </button>
    {/if}
  </div>

  {#if afterHeader}
    {@render afterHeader()}
  {/if}

  <!-- Top loading bar on refresh when the list is already populated —
       mirrors `PipelineList`'s pattern so every consumer of `List` gets
       "click section → section appears instantly → bar animates while
       fresh data loads" without having to open its own spinner. The
       centred spinner below handles the empty-list case. -->
  {#if loading && items.length > 0}
    <div class="list-loading-bar" data-testid="list-loading-bar">
      <div class="loading-bar-track"><div class="loading-bar-fill"></div></div>
    </div>
  {/if}

  <!-- Filter bar (only if filterFn is provided) -->
  {#if filterFn}
    <div class="filter-row">
      <input
        type="text"
        class="filter-input"
        placeholder={filterPlaceholder}
        value={filterInput}
        oninput={(e) => onFilterInput(e.currentTarget.value)}
      />
    </div>
  {/if}

  <!-- Items -->
  <div class="list-items" bind:this={listEl}>
    {#if customContent}
      {@render customContent()}
    {:else if loading && items.length === 0}
      <div class="list-loading">
        <div class="spinner"></div>
      </div>
    {:else if filteredItems.length === 0}
      {#if emptyState}
        {@render emptyState()}
      {:else}
        <div class="list-empty">{emptyMessage}</div>
      {/if}
    {:else}
      {#each filteredItems as item (getKey(item))}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="list-row"
          class:selected={getKey(item) === selectedKey}
          onclick={() => onSelect?.(item)}
          ondblclick={() => onDoubleClick?.(item)}
          oncontextmenu={(e) => {
            if (onContextMenu) {
              e.preventDefault();
              onContextMenu(e, item);
            }
          }}
          role="option"
          tabindex="-1"
          aria-selected={getKey(item) === selectedKey}
        >
          {#if row}
            {@render row({ item, selected: getKey(item) === selectedKey })}
          {/if}
        </div>
      {/each}
      {#if footer}
        {@render footer()}
      {/if}
    {/if}
  </div>
</div>

<style>
  /* list.css provides: .list-header, .list-title, .header-actions,
     .list-items, .list-loading, .list-empty, .list-row, .list-row.selected,
     .filter-row, .filter-input, .refresh-btn, .spinner (global via app.css) */

  .list-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    outline: none;
  }

  .list-loading-bar {
    padding: 0;
    flex-shrink: 0;
  }
  .loading-bar-track {
    height: 2px;
    background: var(--overlay-hover);
    overflow: hidden;
  }
  .loading-bar-fill {
    height: 100%;
    width: 40%;
    background: var(--accent-blue);
    border-radius: 1px;
    animation: list-loading-slide 1s ease-in-out infinite;
  }
  @keyframes list-loading-slide {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(350%); }
  }
</style>
