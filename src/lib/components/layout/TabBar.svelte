<!--
  TabBar — the top toolbar.

  Renders the scrollable row of open tabs (project / terminal / composite),
  the "+" add-project affordance, the terminal button, the AI dropdown, and
  the fetch / pull / push cluster.

  Interaction model (post 2026-04-23 rework):
  • Terminal is a plain button. Single click routes through
    `handleTerminalClick`, which either adds a new terminal segment to the
    active project/composite tab or spawns a standalone terminal in `~`.
  • AI is an always-dropdown gated on `$aiProviders.length > 0`. The menu
    lists one row per installed provider (click → `openAiTerminalTab`) and,
    below a divider, a "Launch session in background…" row
    (click → `requestOpenCreateBackgroundRunDialog`). Escape and
    outside-click close the menu; arrow keys move focus between items.
  • fetch / pull / push only render when a project is active; each button
    dispatches via `runMutation` so toasts + task drawer stay in sync.
-->
<script lang="ts">
  import ProjectTab from "./ProjectTab.svelte";
  import TerminalTab from "./TerminalTab.svelte";
  import CompositeTab from "./CompositeTab.svelte";
  import AddProjectMenu from "./AddProjectMenu.svelte";
  import WindowControls from "./WindowControls.svelte";
  import {
    activeProject,
    switchToTab,
    closeTab,
    toggleAddMenu,
  } from "$lib/stores/projects";
  import {
    openTabs,
    activeTabIndex,
    openTerminalTab,
    openStandaloneTerminal,
    openAiTerminalTab,
    switchSegment,
    closeSegment,
    closeProjectSegment,
    reorderTab,
    tabReordering,
  } from "$lib/stores/tabs";
  import type { Tab } from "$lib/types";
  import { repoInfo } from "$lib/stores/repo";
  import { aiProviders } from "$lib/stores/ai";
  import { requestOpenCreateBackgroundRunDialog } from "$lib/stores/aiBackground";
  import type { AiProviderKind } from "$lib/types";
  import { fetchRemote, pullRemote, pushRemote } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import * as m from "$lib/paraglide/messages";
  import { Button, IconButton } from "$lib/components/ui";

  import { onMount, tick } from "svelte";
  import { flip } from "svelte/animate";
  import ProviderIcon from "../ai-sessions/ProviderIcon.svelte";
  import { providerName } from "$lib/data/ai-providers";

  let tabsRef = $state<HTMLDivElement | null>(null);

  // ── Tab drag-to-reorder ────────────────────────────────────────────────
  // Pointer-based (NOT HTML5 drag-and-drop): native DnD is unreliable inside
  // the macOS WKWebView, so drops silently never fire. Any tab can be dragged
  // to a new position; a composite pill moves as a whole unit (its internal
  // segments are never reordered here). When a project/composite tab changes
  // order, `reorderTab` syncs the backend.
  let draggingIndex = $state<number | null>(null);
  let dropIndicator = $state<number | null>(null);
  let dropSide = $state<"before" | "after">("before");
  // Horizontal offset of the dragged pill so it follows the cursor.
  let dragDx = $state(0);
  // Width (incl. inter-tab gap) of the dragged pill — how far the other
  // tabs slide to open a gap for it.
  let dragWidth = $state(0);

  // Final index the dragged pill would land at, given the current pointer.
  let currentTo = $derived.by(() => {
    if (draggingIndex === null || dropIndicator === null) return null;
    const insertion = dropSide === "after" ? dropIndicator + 1 : dropIndicator;
    return insertion > draggingIndex ? insertion - 1 : insertion;
  });

  /** How far tab `i` slides to open the gap for the dragged pill. */
  function slideOffset(i: number): number {
    if (draggingIndex === null || currentTo === null || i === draggingIndex) return 0;
    const from = draggingIndex;
    if (currentTo > from && i > from && i <= currentTo) return -dragWidth;
    if (currentTo < from && i >= currentTo && i < from) return dragWidth;
    return 0;
  }

  // Non-reactive gesture bookkeeping.
  let pointerDownIndex: number | null = null;
  let pointerStartX = 0;
  let activePointerId: number | null = null;
  let dragSlotEl: HTMLElement | null = null;
  let dragActive = false;
  // A drag ends with a synthetic click on the tab; swallow it so the drop
  // doesn't also switch tabs.
  let suppressNextClick = false;

  /** Stable per-tab key so Svelte moves DOM nodes on reorder instead of
   *  rebuilding them (keeps hover state from jumping between pills). */
  function tabKey(tab: Tab): string {
    return tab.kind === "terminal"
      ? `t-${tab.terminal.sessionId}`
      : `p-${tab.project.path}`;
  }

  /** Which tab, and which side of it, the pointer is over — ignoring the
   *  dragged pill itself (it's translated out of place). */
  function dropTargetAt(clientX: number): { index: number; side: "before" | "after" } | null {
    if (!tabsRef) return null;
    const slots = Array.from(tabsRef.querySelectorAll<HTMLElement>(".tab-slot"));
    let last = -1;
    for (let i = 0; i < slots.length; i++) {
      if (i === draggingIndex) continue; // skip the pill being dragged
      last = i;
      const r = slots[i].getBoundingClientRect();
      if (clientX < r.left + r.width / 2) {
        return { index: i, side: "before" };
      }
    }
    return last >= 0 ? { index: last, side: "after" } : null;
  }

  function onTabPointerDown(e: PointerEvent, i: number) {
    if (e.button !== 0) return; // left button only
    pointerDownIndex = i;
    pointerStartX = e.clientX;
    activePointerId = e.pointerId;
    dragSlotEl = e.currentTarget as HTMLElement;
    dragActive = false;
    suppressNextClick = false;
  }

  function onTabPointerMove(e: PointerEvent) {
    if (pointerDownIndex === null) return;
    if (!dragActive) {
      if (Math.abs(e.clientX - pointerStartX) < 5) return; // movement threshold
      dragActive = true;
      draggingIndex = pointerDownIndex;
      dragWidth = (dragSlotEl?.offsetWidth ?? 0) + 4; // pill + inter-tab gap
      tabReordering.set(true); // suppress hover tooltips while dragging
      // Capture only once a drag actually begins — capturing on pointerdown
      // would redirect the follow-up `click` away from the child button and
      // break click-to-switch. Capture guarantees we still get pointerup even
      // if the release lands outside the tab strip.
      if (activePointerId !== null) {
        try { dragSlotEl?.setPointerCapture(activePointerId); } catch { /* ignore */ }
      }
    }
    dragDx = e.clientX - pointerStartX; // the pill follows the cursor
    const target = dropTargetAt(e.clientX);
    if (target) {
      dropIndicator = target.index;
      dropSide = target.side;
    }
    e.preventDefault();
  }

  function onTabPointerUp() {
    if (activePointerId !== null && dragSlotEl) {
      try { dragSlotEl.releasePointerCapture(activePointerId); } catch { /* not captured */ }
    }
    if (dragActive && draggingIndex !== null && dropIndicator !== null) {
      const from = draggingIndex;
      const insertion = dropSide === "after" ? dropIndicator + 1 : dropIndicator;
      // Removing the dragged item shifts later targets left by one.
      const to = insertion > from ? insertion - 1 : insertion;
      reorderTab(from, to);
      suppressNextClick = true; // eat the click that follows this drag
    }
    resetDragState();
  }

  function onTabPointerCancel() {
    if (activePointerId !== null && dragSlotEl) {
      try { dragSlotEl.releasePointerCapture(activePointerId); } catch { /* not captured */ }
    }
    resetDragState();
  }

  function resetDragState() {
    pointerDownIndex = null;
    activePointerId = null;
    dragSlotEl = null;
    dragActive = false;
    draggingIndex = null;
    dropIndicator = null;
    dragDx = 0;
    dragWidth = 0;
    tabReordering.set(false);
  }

  function onTabClickCapture(e: MouseEvent) {
    if (suppressNextClick) {
      e.stopPropagation();
      e.preventDefault();
      suppressNextClick = false;
    }
  }

  let fetchInProgress = $state(false);
  let pullInProgress = $state(false);
  let pushInProgress = $state(false);
  let aiMenuOpen = $state(false);
  let aiMenuRef = $state<HTMLDivElement | null>(null);

  async function getHomePath(): Promise<string> {
    try {
      const { homeDir } = await import("@tauri-apps/api/path");
      return await homeDir();
    } catch {
      return "/";
    }
  }

  function getActiveCwd(): string {
    return $activeProject?.path ?? "/";
  }

  function getActiveLabel(): string {
    return $activeProject?.path.split("/").pop() ?? "Terminal";
  }

  function toggleAiMenu() {
    aiMenuOpen = !aiMenuOpen;
  }

  function closeAiMenu() {
    aiMenuOpen = false;
  }

  async function handleAiCliClick(kind: AiProviderKind) {
    closeAiMenu();
    await openAiTerminalTab(
      getActiveCwd(),
      `${providerName(kind)} · ${getActiveLabel()}`,
      kind,
    );
  }

  function handleAiBackgroundClick() {
    closeAiMenu();
    requestOpenCreateBackgroundRunDialog();
  }

  function handleAiMenuClickOutside(e: MouseEvent) {
    if (!aiMenuOpen) return;
    if (aiMenuRef && !aiMenuRef.contains(e.target as Node)) {
      aiMenuOpen = false;
    }
  }

  /**
   * Menu-level keydown:
   * - Escape closes the menu and returns focus to the trigger button.
   * - ArrowDown / ArrowUp move focus between menu items (the trigger
   *   button is excluded from the focus cycle).
   * - Enter activates the focused item via the browser's default button
   *   behaviour — we don't intercept it here.
   */
  async function handleAiMenuKeydown(e: KeyboardEvent) {
    if (!aiMenuOpen) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      aiMenuOpen = false;
      await tick();
      const trigger = aiMenuRef?.querySelector<HTMLButtonElement>(
        '[data-testid="toolbar-ai-btn"]',
      );
      trigger?.focus();
      return;
    }
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const items = Array.from(
      aiMenuRef?.querySelectorAll<HTMLButtonElement>(
        '[role="menuitem"]',
      ) ?? [],
    );
    if (items.length === 0) return;
    const current = document.activeElement as HTMLElement | null;
    const idx = current ? items.indexOf(current as HTMLButtonElement) : -1;
    const delta = e.key === "ArrowDown" ? 1 : -1;
    const next = (idx + delta + items.length) % items.length;
    items[next]?.focus();
  }

  /* True when running inside the macOS app: the window uses
     `titleBarStyle: Overlay` (tauri.conf.json), so the native traffic
     lights float over this bar and the tabs need a left inset to clear
     them. Browser/test contexts (and other OSes) keep the flush edge. */
  let trafficLightInset = $state(false);

  onMount(async () => {
    try {
      const { type } = await import("@tauri-apps/plugin-os");
      trafficLightInset = type() === "macos";
    } catch {
      // Not running under Tauri (vite dev in browser, Playwright).
    }
  });

  onMount(() => {
    /* Capture phase, NOT bubble. Several embedded surfaces (xterm.js
       terminals, the CodeMirror editor) call `stopPropagation()` on
       mousedown for their own gesture handling, so a bubble-phase
       document listener never fires when the user clicks them — the
       AI dropdown then stays open even though the click was clearly
       outside it. Capture phase fires before any descendant handler
       can stop the event, which is what "click outside to close"
       semantically wants. */
    document.addEventListener(
      "mousedown",
      handleAiMenuClickOutside,
      { capture: true },
    );
    return () =>
      document.removeEventListener(
        "mousedown",
        handleAiMenuClickOutside,
        { capture: true },
      );
  });

  function handleWheel(e: WheelEvent) {
    if (tabsRef) {
      e.preventDefault();
      tabsRef.scrollLeft += e.deltaY;
    }
  }

  async function handleTerminalClick() {
    const tab = $openTabs[$activeTabIndex];

    // Project or composite tab → always add a new terminal to the composite
    if (tab?.kind === "project" || tab?.kind === "composite") {
      const cwd = tab.project.path;
      const name = cwd.split("/").pop() ?? "Terminal";
      await openTerminalTab(cwd, `${m.tab_terminal()} · ${name}`);
      return;
    }

    // Standalone terminal tab or no tabs → open in ~
    const home = await getHomePath();
    await openStandaloneTerminal(home, m.tab_terminal());
  }

  async function handleFetch() {
    if (fetchInProgress) return;
    fetchInProgress = true;
    try {
      await runMutation({
        kind: "fetch",
        invoke: () => fetchRemote("origin"),
        // `fetchRemote` returns the task-runner's TaskId (a monotonic
        // u64), not a ref count — the background `git fetch` finishes
        // later and the refs-updated number isn't threaded back to the
        // caller. Toast just reports that the op was spawned; the Tasks
        // drawer (Cmd+J) carries the completion status.
        successToast: () => "Fetched origin",
        failureToastPrefix: "Fetch failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      fetchInProgress = false;
    }
  }

  async function handlePull() {
    if (pullInProgress || !$repoInfo?.head_branch) return;
    const branch = $repoInfo.head_branch;
    pullInProgress = true;
    try {
      await runMutation({
        kind: "pull",
        invoke: () => pullRemote("origin", branch),
        // `pullRemote` returns a TaskId, not a commit count — see the
        // fetch handler above. Toast reports spawn; Tasks drawer (Cmd+J)
        // carries the final commit summary.
        successToast: () => `Pulled origin/${branch}`,
        failureToastPrefix: "Pull failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      pullInProgress = false;
    }
  }

  async function handlePush() {
    if (pushInProgress || !$repoInfo?.head_branch) return;
    const branch = $repoInfo.head_branch;
    pushInProgress = true;
    try {
      await runMutation({
        kind: "push",
        invoke: () => pushRemote("origin", branch, false),
        successToast: () => `Pushed to origin/${branch}`,
        failureToastPrefix: "Push failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      pushInProgress = false;
    }
  }
</script>

<div class="tab-bar" class:tab-bar--traffic-lights={trafficLightInset} data-tauri-drag-region>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="tabs-scroll" class:reordering={draggingIndex !== null} bind:this={tabsRef} onwheel={handleWheel}>
    {#each $openTabs as tab, i (tabKey(tab))}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="tab-slot"
        class:dragging={draggingIndex === i}
        style:transform={draggingIndex === i
          ? `translateX(${dragDx}px)`
          : slideOffset(i) !== 0
            ? `translateX(${slideOffset(i)}px)`
            : null}
        animate:flip={{ duration: 160 }}
        onpointerdown={(e) => onTabPointerDown(e, i)}
        onpointermove={onTabPointerMove}
        onpointerup={onTabPointerUp}
        onpointercancel={onTabPointerCancel}
        onclickcapture={onTabClickCapture}
      >
        {#if tab.kind === "project"}
          <ProjectTab
            project={tab.project}
            isActive={i === $activeTabIndex}
            index={i}
            onSwitch={(idx) => switchToTab(idx)}
            onClose={(idx) => closeTab(idx)}
          />
        {:else if tab.kind === "terminal"}
          <TerminalTab
            terminal={tab.terminal}
            isActive={i === $activeTabIndex}
            onSwitch={() => switchToTab(i)}
            onClose={() => closeTab(i)}
          />
        {:else if tab.kind === "composite"}
          <CompositeTab
            project={tab.project}
            segments={tab.segments}
            activeSegmentIndex={tab.activeSegmentIndex}
            isActiveTab={i === $activeTabIndex}
            onSwitchSegment={(segmentIndex) => {
              switchSegment(i, segmentIndex);
              if (i !== $activeTabIndex) switchToTab(i);
            }}
            onCloseProject={async () => {
              const projectIdx = (await import("$lib/stores/tabs")).tabIndexToProjectIndex(i);
              closeProjectSegment(i);
              if (projectIdx >= 0) {
                const { closeProject } = await import("$lib/api/tauri");
                await closeProject(projectIdx);
              }
            }}
            onCloseSegment={(segmentIndex) => closeSegment(i, segmentIndex)}
          />
        {/if}
      </div>
    {/each}
  </div>

  <div class="add-button-wrapper">
    <IconButton icon={""} description="Add project" onclick={toggleAddMenu} />
    <AddProjectMenu />
  </div>

  <div class="actions">
    <IconButton
      icon={""}
      description={m.tab_terminal_here()}
      testid="toolbar-terminal-btn"
      tone="default"
      onclick={handleTerminalClick}
    />
    {#if ($aiProviders?.length ?? 0) > 0}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="ai-dropdown"
        bind:this={aiMenuRef}
        onkeydown={handleAiMenuKeydown}
      >
        <Button
          variant="neutral"
          size="sm"
          testid="toolbar-ai-btn"
          description={m.ai_background_tab_button_tooltip()}
          ariaHaspopup="menu"
          ariaExpanded={aiMenuOpen}
          active={aiMenuOpen}
          onclick={toggleAiMenu}
        >
          <span class="ai-bg-label">{m.ai_background_tab_button_label()}</span>
          <span class="chevron nf" class:open={aiMenuOpen} aria-hidden="true">{""}</span>
        </Button>
        {#if aiMenuOpen}
          <div
            class="action-menu"
            role="menu"
            data-testid="toolbar-ai-menu"
          >
            {#each $aiProviders as provider (provider.kind)}
              <button
                class="action-menu-item"
                role="menuitem"
                data-testid={`toolbar-ai-item-${provider.kind}`}
                onclick={() => handleAiCliClick(provider.kind)}
              >
                <ProviderIcon provider={provider.kind} size={16} />
                <span class="menu-item-label">
                  {providerName(provider.kind)}
                  {#if provider.version}
                    <span class="menu-item-meta">{provider.version}</span>
                  {/if}
                </span>
              </button>
            {/each}
            <div class="action-menu-divider" role="separator"></div>
            <button
              class="action-menu-item"
              role="menuitem"
              data-testid="toolbar-ai-item-background"
              onclick={handleAiBackgroundClick}
            >
              <span class="nf menu-icon" aria-hidden="true">{""}</span>
              <span class="menu-item-label">{m.ai_menu_launch_background()}</span>
            </button>
          </div>
        {/if}
      </div>
    {/if}
    {#if $activeProject}
      <Button
        variant="neutral"
        size="sm"
        icon={""}
        disabled={fetchInProgress}
        description={m.toolbar_fetch()}
        onclick={handleFetch}
      >{m.toolbar_fetch()}</Button>
      <Button
        variant="neutral"
        size="sm"
        icon={""}
        disabled={pullInProgress || !$repoInfo?.head_branch}
        description={m.toolbar_pull()}
        onclick={handlePull}
      >{m.toolbar_pull()}</Button>
      <Button
        variant="neutral"
        size="sm"
        icon={""}
        disabled={pushInProgress || !$repoInfo?.head_branch}
        description={m.toolbar_push()}
        onclick={handlePush}
      >{m.toolbar_push()}</Button>
    {/if}
    <WindowControls />
  </div>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: center;
    height: 36px;
    min-height: 36px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--border);
    user-select: none;
    padding: 0 8px;
    gap: 4px;
  }

  /* macOS overlay title bar: clear the native traffic lights, which
     float over the bar's left edge (≈ 70 px including margins). */
  .tab-bar--traffic-lights {
    padding-left: 84px;
  }

  .tabs-scroll {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    min-width: 0;
    scrollbar-width: none;
  }

  .tabs-scroll::-webkit-scrollbar {
    display: none;
  }

  /* Each tab lives in a draggable slot so the whole pill reorders as a
     unit. The drop indicator sits in the 4px gap between slots. */
  .tab-slot {
    position: relative;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    /* Keep the pointer gesture as a drag rather than letting the scroller
       or the OS interpret it. */
    touch-action: none;
  }

  /* Only animate while a drag is in progress. Off-drag the transition is
     absent, so when the drop reorders the DOM the transforms clear
     instantly and the pills don't jump. */
  .tabs-scroll.reordering .tab-slot {
    transition: transform 0.15s ease;
  }

  /* The dragged pill must track the cursor 1:1 — no easing lag. */
  .tabs-scroll.reordering .tab-slot.dragging {
    opacity: 0.85;
    cursor: grabbing;
    z-index: 5;
    box-shadow: var(--shadow-overlay);
    border-radius: 14px;
    transition: none;
  }

  .add-button-wrapper {
    position: relative;
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }


  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    margin-left: auto;
  }

  /* AI-background button carries a bold "AI"/"IA" label rather than a
     glyph — shorter, locale-aware, and self-explanatory. */
  .ai-bg-label {
    font-weight: 700;
    font-size: var(--font-size-xs);
    letter-spacing: 0.5px;
  }

  .chevron {
    font-size: 9px;
    margin-left: 2px;
    color: var(--text-secondary);
  }

  /* ── AI dropdown ── */

  .ai-dropdown {
    position: relative;
    display: flex;
    align-items: stretch;
  }

  .action-menu {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 100;
    min-width: 200px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: var(--shadow-overlay);
    padding: 4px 0;
    margin-top: 2px;
  }

  .action-menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    white-space: nowrap;
  }

  .action-menu-item:hover,
  .action-menu-item:focus-visible {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    outline: none;
  }

  .action-menu-item .menu-icon {
    width: 16px;
    text-align: center;
    color: var(--text-secondary);
  }

  .action-menu-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }

  .menu-item-label {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .menu-item-meta {
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
  }
</style>
