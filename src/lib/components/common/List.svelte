<!--
  Generic list panel with header, optional filter bar, scrollable items,
  loading/empty states, and single-item keyboard selection.

  Consumers provide layout via Svelte 5 snippets: row, headerActions,
  and emptyState. Right-click is exposed via the onContextMenu callback.
-->
<script lang="ts" generics="T">
  import type { Snippet } from "svelte";
  import { debounce } from "../../utils/debounce";
  import { IconButton } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

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
    /** True when a background refresh is in flight while rows are already shown.
     *  Drives the 2 px polling bar between header/afterHeader and the items list. */
    refreshing?: boolean;
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
    /**
     * Pixel height of a single row. When set together with a list size
     * above `virtualizeOver`, the component switches to a windowed
     * renderer that only mounts rows visible in the viewport (plus a
     * small overscan above and below). Required for virtualization —
     * the implementation assumes a uniform row height.
     */
    rowHeight?: number;
    /**
     * Threshold above which virtualization kicks in. Defaults to 500
     * rows, matching the value documented in
     * `lib/components/common/CLAUDE.md`. Below this we keep the plain
     * `{#each}` path so layouts that depend on intrinsic row heights
     * stay correct. Set to `0` to force virtualization for any non-
     * empty list (useful in tests).
     */
    virtualizeOver?: number;
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
    refreshing = false,
    row,
    headerActions,
    emptyState,
    afterHeader,
    footer,
    customContent,
    rowHeight,
    virtualizeOver = 500,
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

  // ── Virtualization ────────────────────────────────────────────────────
  // Only kicks in when `rowHeight` is set AND the filtered list size is
  // above `virtualizeOver`. The window is scroll-driven: we track the
  // scroll container's `scrollTop` + measured viewport height and slice
  // a [start, end) range out of `filteredItems` with a small overscan
  // so fast scrolls don't reveal blank rows.
  const OVERSCAN = 6;
  let scrollTop = $state(0);
  let viewportHeight = $state(0);

  let isVirtualized = $derived(
    rowHeight !== undefined && filteredItems.length > virtualizeOver,
  );

  let virtualWindow = $derived.by(() => {
    if (!isVirtualized || rowHeight === undefined) {
      return { start: 0, end: filteredItems.length, totalHeight: 0 };
    }
    const total = filteredItems.length * rowHeight;
    const visible = Math.ceil((viewportHeight || 600) / rowHeight) + OVERSCAN * 2;
    const rawStart = Math.floor(scrollTop / rowHeight) - OVERSCAN;
    const start = Math.max(0, rawStart);
    const end = Math.min(filteredItems.length, start + visible);
    return { start, end, totalHeight: total };
  });

  function handleScroll(e: Event) {
    if (!isVirtualized) return;
    const el = e.currentTarget as HTMLDivElement;
    scrollTop = el.scrollTop;
    // Measure viewport height lazily on the scroll path so we never
    // depend on `bind:clientHeight` (which Svelte 5 wires through
    // ResizeObserver — not available in the JSDOM-based test env).
    if (viewportHeight !== el.clientHeight) {
      viewportHeight = el.clientHeight;
    }
  }

  // Measure viewport height once when virtualization first kicks in.
  // We avoid `bind:clientHeight` because Svelte 5 implements it via
  // ResizeObserver, which JSDOM does not provide and which would crash
  // every `List`-using test that doesn't shim it. The fallback in
  // `virtualWindow` (`viewportHeight || 600`) keeps the first paint
  // sane until either this measurement or the first real scroll runs.
  $effect(() => {
    if (!isVirtualized || !listEl) return;
    if (viewportHeight === 0 && listEl.clientHeight > 0) {
      viewportHeight = listEl.clientHeight;
    }
  });

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
      <IconButton
        icon={"\uF021"}
        description={m.tooltip_refresh()}
        loading={loading}
        onclick={onRefresh}
      />
    {/if}
  </div>

  {#if afterHeader}
    {@render afterHeader()}
  {/if}

  <!-- Top loading bar on refresh when the list is already populated. Two
       signals fold in here:
       - `refreshing` (explicit) — the preferred API for polling-driven lists
         (`PipelineList`) that want the bar without forcing `loading=true`.
       - `loading && items.length > 0` (legacy) — kept so existing consumers
         (IssueList, MrPrList, BranchList, …) don't change behaviour. -->
  {#if (refreshing || loading) && items.length > 0}
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
  <div
    class="list-items"
    bind:this={listEl}
    onscroll={handleScroll}
  >
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
    {:else if isVirtualized && rowHeight !== undefined}
      <!-- Virtualized window: a tall sizer keeps the scrollbar honest,
           and only the visible slice is mounted with absolute
           positioning anchored to (start * rowHeight). -->
      <div
        class="virt-sizer"
        style="height: {virtualWindow.totalHeight}px; position: relative"
      >
        {#each filteredItems.slice(virtualWindow.start, virtualWindow.end) as item, i (getKey(item))}
          {@const absoluteIndex = virtualWindow.start + i}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="list-row"
            class:selected={getKey(item) === selectedKey}
            style="position: absolute; left: 0; right: 0; top: {absoluteIndex * rowHeight}px; height: {rowHeight}px"
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
      </div>
      {#if footer}
        {@render footer()}
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
     .filter-row, .filter-input, .spinner (global via app.css) */

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
