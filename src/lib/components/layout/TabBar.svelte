<!--
  TabBar — the top toolbar.

  Renders the scrollable row of open tabs (project / terminal / composite),
  the "+" add-project affordance, the AI dropdown, the terminal button, and
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
  } from "$lib/stores/tabs";
  import { repoInfo } from "$lib/stores/repo";
  import { aiProviders } from "$lib/stores/ai";
  import { requestOpenCreateBackgroundRunDialog } from "$lib/stores/aiBackground";
  import type { AiProviderKind } from "$lib/types";
  import { fetchRemote, pullRemote, pushRemote } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import * as m from "$lib/paraglide/messages";

  import { onMount, tick } from "svelte";
  import ProviderIcon from "../ai-sessions/ProviderIcon.svelte";
  import { providerName } from "$lib/data/ai-providers";

  let tabsRef = $state<HTMLDivElement | null>(null);
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

<div class="tab-bar" data-tauri-drag-region>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="tabs-scroll" bind:this={tabsRef} onwheel={handleWheel}>
    {#each $openTabs as tab, i}
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
    {/each}
  </div>

  <div class="add-button-wrapper">
    <button class="add-btn" onclick={toggleAddMenu} title="Add project"><span class="nf">{""}</span></button>
    <AddProjectMenu />
  </div>

  <div class="actions">
    {#if $aiProviders.length > 0}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="ai-dropdown"
        bind:this={aiMenuRef}
        onkeydown={handleAiMenuKeydown}
      >
        <button
          class="action-btn ai-bg-btn"
          data-testid="toolbar-ai-btn"
          title={m.ai_background_tab_button_tooltip()}
          aria-label={m.ai_background_tab_button_tooltip()}
          aria-haspopup="menu"
          aria-expanded={aiMenuOpen}
          onclick={toggleAiMenu}
        >
          <span class="ai-bg-label">{m.ai_background_tab_button_label()}</span>
          <span class="nf chevron" aria-hidden="true">{""}</span>
        </button>
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
    <button
      class="action-btn"
      data-testid="toolbar-terminal-btn"
      title={m.tab_terminal_here()}
      aria-label={m.tab_terminal_here()}
      onclick={handleTerminalClick}
    >
      <span class="nf">{""}</span>
    </button>
    {#if $activeProject}
      <button
        class="action-btn"
        disabled={fetchInProgress}
        title={m.toolbar_fetch()}
        onclick={handleFetch}
      >
        <span class="nf">{""}</span> {m.toolbar_fetch()}
      </button>
      <button
        class="action-btn"
        disabled={pullInProgress || !$repoInfo?.head_branch}
        title={m.toolbar_pull()}
        onclick={handlePull}
      >
        <span class="nf">{""}</span> {m.toolbar_pull()}
      </button>
      <button
        class="action-btn"
        disabled={pushInProgress || !$repoInfo?.head_branch}
        title={m.toolbar_push()}
        onclick={handlePush}
      >
        <span class="nf">{""}</span> {m.toolbar_push()}
      </button>
    {/if}
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

  .add-button-wrapper {
    position: relative;
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .add-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 28px;
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    font-family: var(--font-icons);
    line-height: 1;
    cursor: pointer;
    border-radius: 14px;
    transition: background 0.15s;
    padding: 0 10px;
  }

  .add-btn:hover {
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
    color: var(--text-primary);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    margin-left: auto;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 11px;
    cursor: pointer;
    transition: background 0.15s;
    min-width: 44px;
  }

  .action-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--text-primary) 10%, transparent);
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  /* AI-background button carries a bold "AI"/"IA" label rather than a
     glyph — shorter, locale-aware, and self-explanatory. */
  .ai-bg-label {
    font-weight: 700;
    font-size: 11px;
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
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3); /* beardgit:allow-hex: shadow neutral always-dark */
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
    font-size: 12px;
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
    font-size: 10px;
    color: var(--text-secondary);
  }
</style>
