<script lang="ts">
  import { onMount } from "svelte";
  import { fileStatuses } from "../../stores/changes";
  import { hasActiveProvider, activeProvider } from "../../stores/provider";
  import { sidebarLayout, updateLayout } from "../../stores/sidebarLayout";
  import {
    applyLayout,
    DEFAULT_ORDER,
    type SidebarNavItem,
  } from "../../utils/applyLayout";
  import { startPointerReorder } from "../../utils/pointerReorder";
  import { addToast } from "../../stores/toast";
  import { IconButton } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

  let {
    onNavigate,
    activeView = "graph",
    collapsed = false,
    onToggleCollapse,
  }: {
    onNavigate?: (view: string) => void;
    activeView?: string;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
  } = $props();

  /** All registered Navigation items. */
  const navItems: SidebarNavItem[] = [
    { label: m.sidebar_graph(), icon: "", id: "graph" },
    { label: m.sidebar_changes(), icon: "", id: "changes" },
    { label: m.sidebar_editor(), icon: "", id: "editor" },
    { label: m.sidebar_branches(), icon: "", id: "branches" },
    { label: m.sidebar_tags(), icon: "", id: "tags" },
    { label: m.sidebar_stashes(), icon: "", id: "stashes" },
    { label: m.sidebar_worktrees(), icon: "", id: "worktrees" },
    { label: m.sidebar_reflog(), icon: "", id: "reflog" },
    { label: m.sidebar_bisect(), icon: "", id: "bisect" },
    { label: m.sidebar_submodules(), icon: "", id: "submodules" },
    { label: m.sidebar_ai_config(), icon: "", id: "ai-config" },
    { label: m.sidebar_ai_sessions(), icon: "", id: "ai-sessions" },
    { label: m.sidebar_requests(), icon: "", id: "requests" },
  ];

  // Edit-mode state.
  let editMode = $state(false);
  let dragIndex = $state<number | null>(null);
  let dragOverIndex = $state<number | null>(null);
  /** Reveal hidden items inline below the visible list (normal-mode only). */
  let showHidden = $state(false);
  let sidebarEl: HTMLElement | undefined = $state();

  /** Normal-mode list: respects order + hidden. */
  let visibleNavItems = $derived(
    applyLayout(navItems, $sidebarLayout.order, $sidebarLayout.hidden),
  );

  /** Hidden items in saved order (normal-mode "Show more…" expansion). */
  let hiddenNavItems = $derived.by(() => {
    const all = applyLayout(navItems, $sidebarLayout.order, []);
    const hiddenSet = new Set($sidebarLayout.hidden);
    return all.filter((i) => hiddenSet.has(i.id));
  });

  /** Edit-mode list: full set in saved order, hidden items included
   *  (rendered with `.nav-item--hidden` styling). */
  let editModeItems = $derived(
    applyLayout(navItems, $sidebarLayout.order, []),
  );

  /** Force-exit edit mode if the sidebar collapses. */
  $effect(() => {
    if (collapsed && editMode) editMode = false;
  });

  /** Escape + outside-click handlers while in edit mode. */
  $effect(() => {
    if (!editMode) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") editMode = false;
    }
    function onPointer(e: MouseEvent) {
      if (!sidebarEl) return;
      if (!sidebarEl.contains(e.target as Node)) editMode = false;
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onPointer);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onPointer);
    };
  });

  // MR/PR label depends on the active forge.
  let providerItems = $derived.by<SidebarNavItem[]>(() => {
    const base: SidebarNavItem[] = [
      { label: m.sidebar_pipelines(), icon: "", id: "pipelines" },
      { label: m.sidebar_issues(), icon: "", id: "issues" },
      {
        label:
          $activeProvider?.kind === "github"
            ? m.sidebar_pull_requests()
            : m.sidebar_merge_requests(),
        icon: "",
        id: "merge-requests",
      },
      { label: m.sidebar_releases(), icon: "", id: "releases" },
    ];
    const kind = $activeProvider?.kind;
    if (kind === "github" || kind === "gitlab") {
      base.push({
        label: m.sidebar_repo_config(),
        icon: "",
        id: "repo-config",
      });
    }
    return base;
  });

  function handleNav(id: string) {
    if (editMode) return; // Clicks on the row are reorder/toggle targets,
                          // not navigation, while editing.
    onNavigate?.(id);
  }

  /** Toggle an id between visible and hidden, guarding "at least one visible". */
  function toggleHidden(id: string) {
    const hidden = new Set($sidebarLayout.hidden);
    if (hidden.has(id)) {
      hidden.delete(id);
      updateLayout({ hidden: [...hidden] });
      return;
    }
    // About to hide → ensure at least one other item remains visible.
    const nextHidden = new Set(hidden);
    nextHidden.add(id);
    const remainingVisible = navItems.filter((i) => !nextHidden.has(i.id));
    if (remainingVisible.length === 0) {
      addToast({ message: m.sidebar_min_visible(), type: "warning" });
      return;
    }
    updateLayout({ hidden: [...nextHidden] });
  }

  /** Compute the current saved-order array, filling any gaps with DEFAULT_ORDER. */
  function currentOrder(): string[] {
    const saved = $sidebarLayout.order;
    const seen = new Set<string>();
    const out: string[] = [];
    for (const id of saved) {
      if (!seen.has(id) && navItems.some((n) => n.id === id)) {
        out.push(id);
        seen.add(id);
      }
    }
    for (const id of DEFAULT_ORDER) {
      if (!seen.has(id)) {
        out.push(id);
        seen.add(id);
      }
    }
    return out;
  }

  function moveItem(id: string, delta: number) {
    const order = currentOrder();
    const from = order.indexOf(id);
    const to = from + delta;
    if (from < 0 || to < 0 || to >= order.length) return;
    const next = [...order];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    updateLayout({ order: next });
  }

  function handleKeydownHandle(e: KeyboardEvent, id: string) {
    if (e.key === "ArrowUp") {
      e.preventDefault();
      moveItem(id, -1);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      moveItem(id, 1);
    }
  }

  /**
   * Mouse-based reorder (see `$lib/utils/pointerReorder`): HTML5 drag &
   * drop is swallowed by Tauri's native drag handler (`dragDropEnabled`)
   * on Windows and on recent macOS WebKit builds, so the rows track
   * plain mousemove instead. The drop lands the moved item AT the
   * hovered index, matching the full-row `.drag-over` tint.
   */
  function handleRowMouseDown(e: MouseEvent, index: number) {
    const target = e.target as HTMLElement;
    // The eye-toggle keeps its click; the handle and the rest of the
    // row start a drag.
    if (target.closest("button") && !target.closest(".drag-handle")) return;
    if (!sidebarEl) return;
    dragIndex = index;
    startPointerReorder({
      event: e,
      index,
      container: sidebarEl,
      rowSelector: ".edit-row",
      onDragOver: (i) => (dragOverIndex = i),
      onDrop: (from, to) => {
        const next = [...currentOrder()];
        const [moved] = next.splice(from, 1);
        next.splice(to, 0, moved);
        updateLayout({ order: next });
      },
      onEnd: () => {
        dragIndex = null;
        dragOverIndex = null;
      },
    });
  }

  function resetLayout() {
    updateLayout({ order: [...DEFAULT_ORDER], hidden: [] });
  }

  let changeCount = $derived($fileStatuses.length);
</script>

<aside
  class="sidebar"
  class:collapsed
  class:edit-mode={editMode}
  bind:this={sidebarEl}
>
  <nav class="nav-section">
    {#if !collapsed}
      <div class="section-label nav-header">
        <span>{m.sidebar_navigation()}</span>
        {#if editMode}
          <span class="edit-actions">
            <button
              type="button"
              class="edit-action"
              data-testid="sidebar-edit-reset"
              onclick={resetLayout}
            >{m.sidebar_reset()}</button>
            <button
              type="button"
              class="edit-action primary"
              data-testid="sidebar-edit-done"
              onclick={() => (editMode = false)}
            >{m.sidebar_done()}</button>
          </span>
        {:else}
          <IconButton
            icon={"\uF040"}
            description={m.tooltip_customize_sidebar()}
            size="sm"
            testid="sidebar-edit-toggle"
            onclick={() => (editMode = true)}
          />
        {/if}
      </div>
    {/if}

    {#if editMode}
      <div class="sr-only" role="status" aria-live="polite">
        {m.sidebar_customize()}. Press Escape to finish.
      </div>
      {#each editModeItems as item, i (item.id)}
        {@const isHidden = $sidebarLayout.hidden.includes(item.id)}
        {@const visibleCount = navItems.length - $sidebarLayout.hidden.length}
        {@const isLastVisible = !isHidden && visibleCount <= 1}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="nav-item edit-row"
          class:nav-item--hidden={isHidden}
          class:dragging={dragIndex === i}
          class:drag-over={dragOverIndex === i}
          data-testid="nav-{item.id}"
          onmousedown={(e) => handleRowMouseDown(e, i)}
        >
          <button
            type="button"
            class="drag-handle"
            data-testid="sidebar-reorder-{item.id}"
            aria-label={m.sidebar_reorder_aria({ label: item.label })}
            onkeydown={(e) => handleKeydownHandle(e, item.id)}
          >{"☰"}</button>
          <span class="nav-icon">{item.icon}</span>
          <span class="nav-label">{item.label}</span>
          <button
            type="button"
            class="eye-toggle"
            data-testid="sidebar-hide-{item.id}"
            aria-pressed={!isHidden}
            aria-disabled={isLastVisible}
            disabled={isLastVisible}
            aria-label={isHidden
              ? m.sidebar_show_aria({ label: item.label })
              : m.sidebar_hide_aria({ label: item.label })}
            onclick={() => toggleHidden(item.id)}
          >{isHidden ? "" : ""}</button>
        </div>
      {/each}
    {:else}
      {#each visibleNavItems as item}
        <button
          class="nav-item"
          class:active={activeView === item.id}
          onclick={() => handleNav(item.id)}
          title={collapsed ? item.label : undefined}
          data-testid="nav-{item.id}"
        >
          <span class="nav-icon">{item.icon}</span>
          {#if !collapsed}
            <span class="nav-label">{item.label}</span>
            {#if item.id === "changes" && changeCount > 0}
              <span class="nav-badge">{changeCount}</span>
            {/if}
          {/if}
        </button>
      {/each}

      {#if !collapsed && hiddenNavItems.length > 0}
        <button
          type="button"
          class="nav-item show-more"
          data-testid="sidebar-show-hidden"
          onclick={() => (showHidden = !showHidden)}
        >
          <span class="nav-icon">{showHidden ? "" : ""}</span>
          <span class="nav-label">
            {showHidden ? m.sidebar_hide_hidden() : m.sidebar_show_hidden()}
          </span>
          <span class="nav-badge nav-badge--muted">{hiddenNavItems.length}</span>
        </button>

        {#if showHidden}
          {#each hiddenNavItems as item (item.id)}
            <button
              class="nav-item nav-item--hidden-row"
              class:active={activeView === item.id}
              onclick={() => handleNav(item.id)}
              data-testid="nav-{item.id}"
            >
              <span class="nav-icon">{item.icon}</span>
              <span class="nav-label">{item.label}</span>
            </button>
          {/each}
        {/if}
      {/if}
    {/if}
  </nav>

  {#if $hasActiveProvider}
    <nav class="nav-section">
      {#if !collapsed}
        <div class="section-label">
          <span class="provider-status-dot connected"></span>
          {$activeProvider?.kind === 'github' ? m.provider_github() : m.provider_gitlab()}
        </div>
      {/if}
      {#each providerItems as item}
        <button
          class="nav-item"
          class:active={activeView === item.id}
          onclick={() => handleNav(item.id)}
          title={collapsed ? item.label : undefined}
          data-testid="nav-{item.id}"
        >
          <span class="nav-icon">{item.icon}</span>
          {#if !collapsed}
            <span class="nav-label">{item.label}</span>
          {/if}
        </button>
      {/each}
    </nav>
  {/if}

  <div class="spacer"></div>

  <div class="nav-section bottom-section">
    <button
      class="nav-item"
      class:active={activeView === "settings"}
      onclick={() => handleNav("settings")}
      title={collapsed ? m.sidebar_settings() : undefined}
      data-testid="nav-settings"
    >
      <span class="nav-icon">{""}</span>
      {#if !collapsed}
        <span class="nav-label">{m.sidebar_settings()}</span>
      {/if}
    </button>
    <button
      class="nav-item collapse-btn"
      onclick={onToggleCollapse}
      title={collapsed ? m.sidebar_expand() : m.sidebar_collapse()}
    >
      <span class="nav-icon">{collapsed ? "" : ""}</span>
      {#if !collapsed}
        <span class="nav-label">{m.sidebar_collapse()}</span>
      {/if}
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: clamp(180px, 15vw, 240px);
    min-width: 0;
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    user-select: none;
    transition: width 150ms ease;
  }

  .sidebar.collapsed {
    width: 44px;
  }

  .nav-section {
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .nav-section:last-child {
    border-bottom: none;
  }

  .bottom-section {
    border-top: 1px solid var(--border);
    border-bottom: none;
  }

  .section-label {
    padding: 4px 16px 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    overflow: hidden;
    white-space: nowrap;
  }

  .nav-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .edit-actions {
    display: inline-flex;
    gap: 6px;
  }

  .edit-action {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 3px;
    cursor: pointer;
    text-transform: none;
    letter-spacing: 0;
  }

  .edit-action:hover {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }

  .edit-action.primary {
    color: var(--accent-primary);
    border-color: var(--accent-primary);
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 16px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s;
    overflow: hidden;
    white-space: nowrap;
  }

  .sidebar.collapsed .nav-item {
    justify-content: center;
    padding: 6px 0;
  }

  .nav-item:hover {
    background: color-mix(in srgb, var(--text-primary) 5%, transparent);
  }

  .sidebar:not(.edit-mode) .nav-item.active {
    background: var(--overlay-accent-blue);
    color: var(--accent-primary);
  }

  .sidebar:not(.edit-mode) .nav-item.active .nav-icon {
    color: var(--accent-primary);
  }

  .nav-icon {
    width: 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 14px;
    font-family: var(--font-icons);
    flex-shrink: 0;
  }

  .nav-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .nav-badge {
    font-size: 10px;
    background: var(--accent-primary);
    color: var(--text-primary);
    border-radius: 8px;
    padding: 0 5px;
    min-width: 16px;
    text-align: center;
    line-height: 16px;
  }

  .provider-status-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-secondary);
    margin-right: 4px;
    vertical-align: middle;
  }

  .provider-status-dot.connected {
    background: var(--accent-green);
  }

  .spacer {
    flex: 1;
  }

  .collapse-btn .nav-icon {
    font-size: 12px;
  }

  /* Edit-mode row layout: [drag][icon][label][eye] */
  .edit-row {
    cursor: grab;
  }

  .edit-row.dragging {
    opacity: 0.5;
  }

  .edit-row.drag-over {
    background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
  }

  .drag-handle {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: grab;
    padding: 0 2px;
    font-size: 14px;
    line-height: 1;
  }

  .drag-handle:focus {
    outline: 1px solid var(--accent-primary);
    border-radius: 2px;
  }

  .eye-toggle {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-family: var(--font-icons);
    font-size: 13px;
    cursor: pointer;
    padding: 0 4px;
  }

  .eye-toggle:hover:not([disabled]) {
    color: var(--text-primary);
  }

  .eye-toggle[disabled] {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .nav-item--hidden {
    opacity: 0.5;
  }

  .nav-item--hidden .nav-label {
    text-decoration: line-through;
  }

  /* "Show more…" expander row in normal mode */
  .nav-item.show-more {
    color: var(--text-secondary);
    font-style: italic;
  }

  .nav-item.show-more .nav-icon {
    font-size: 11px;
  }

  /* Hidden items revealed inline below the visible list — dimmer than a
     normal nav row so they read as "you've chosen not to show these by
     default", but still clickable + active-state aware. */
  .nav-item--hidden-row {
    opacity: 0.55;
  }

  .nav-item--hidden-row:hover {
    opacity: 1;
  }

  .nav-badge--muted {
    background: color-mix(in srgb, var(--text-secondary) 30%, transparent);
    color: var(--text-secondary);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
