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
  import { openTabs, activeTabIndex, openTerminalTab, openStandaloneTerminal, openAiTerminalTab, switchSegment, closeSegment, closeProjectSegment, getActiveTerminalSegment } from "$lib/stores/tabs";
  import { repoInfo } from "$lib/stores/repo";
  import { aiProviders } from "$lib/stores/ai";
  import { requestOpenCreateBackgroundRunDialog } from "$lib/stores/aiBackground";
  import type { AiProviderKind } from "$lib/types";
  import { fetchRemote, pullRemote, pushRemote } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import * as m from "$lib/paraglide/messages";

  import { onMount } from "svelte";
  import ProviderBrandIcon from "../ai/ProviderBrandIcon.svelte";
  import { providerName } from "$lib/data/ai-providers";

  let tabsRef = $state<HTMLDivElement | null>(null);
  let fetchInProgress = $state(false);
  let pullInProgress = $state(false);
  let pushInProgress = $state(false);
  let terminalMenuOpen = $state(false);
  let terminalMenuRef = $state<HTMLDivElement | null>(null);

  function toggleTerminalMenu() {
    terminalMenuOpen = !terminalMenuOpen;
  }

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

  async function handleTerminalHome() {
    terminalMenuOpen = false;
    const home = await getHomePath();
    await openTerminalTab(home, m.tab_terminal());
  }

  async function handleTerminalAi(kind: AiProviderKind) {
    terminalMenuOpen = false;
    await openAiTerminalTab(getActiveCwd(), `${providerName(kind)} · ${getActiveLabel()}`, kind);
  }

  function handleTerminalMenuClickOutside(e: MouseEvent) {
    if (terminalMenuRef && !terminalMenuRef.contains(e.target as Node)) {
      terminalMenuOpen = false;
    }
  }

  onMount(() => {
    document.addEventListener("mousedown", handleTerminalMenuClickOutside);
    return () => document.removeEventListener("mousedown", handleTerminalMenuClickOutside);
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

    // Standalone terminal tab or no tabs → open in ~ via dropdown only
    // (this path is reached only from standalone terminal tabs with no project)
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
        successToast: (n) => `Fetched origin — ${n} ref${n === 1 ? "" : "s"}`,
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
        successToast: (n) =>
          `Pulled origin/${branch} — ${n} commit${n === 1 ? "" : "s"}`,
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
        invoke: () => pushRemote("origin", branch),
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
    <button class="add-btn" onclick={toggleAddMenu} title="Add project"><span class="nf">{"\uF067"}</span></button>
    <AddProjectMenu />
  </div>

  <div class="actions">
    {#if $aiProviders.length > 0}
      <button
        class="action-btn ai-bg-btn"
        title={m.ai_background_tab_button_tooltip()}
        aria-label={m.ai_background_tab_button_tooltip()}
        onclick={() => requestOpenCreateBackgroundRunDialog()}
      >
        <span class="ai-bg-label">{m.ai_background_tab_button_label()}</span>
      </button>
    {/if}
    <div class="terminal-split" bind:this={terminalMenuRef}>
      <button
        class="action-btn terminal-left"
        title={m.tab_terminal_here()}
        onclick={handleTerminalClick}
      >
        <span class="nf">{"\uF489"}</span>
      </button>
      <button
        class="action-btn terminal-right"
        title="Terminal options"
        onclick={toggleTerminalMenu}
      >
        <span class="nf chevron">{"\uF078"}</span>
      </button>
      {#if terminalMenuOpen}
        <div class="terminal-menu">
          <button class="terminal-menu-item" onclick={handleTerminalHome}>
            <span class="nf menu-icon">{"\uF489"}</span> {m.tab_terminal_home()}
          </button>
          {#if $aiProviders.length > 0}
            <div class="terminal-menu-divider"></div>
            {#each $aiProviders as provider}
              <button class="terminal-menu-item" onclick={() => handleTerminalAi(provider.kind)}>
                <ProviderBrandIcon provider={provider.kind} size={16} />
                <span class="provider-label">
                  {providerName(provider.kind)}
                  {#if provider.version}
                    <span class="provider-version">{provider.version}</span>
                  {/if}
                </span>
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
    {#if $activeProject}
      <button
        class="action-btn"
        disabled={fetchInProgress}
        title={m.toolbar_fetch()}
        onclick={handleFetch}
      >
        <span class="nf">{"\uF0ED"}</span> {m.toolbar_fetch()}
      </button>
      <button
        class="action-btn"
        disabled={pullInProgress || !$repoInfo?.head_branch}
        title={m.toolbar_pull()}
        onclick={handlePull}
      >
        <span class="nf">{"\uF063"}</span> {m.toolbar_pull()}
      </button>
      <button
        class="action-btn"
        disabled={pushInProgress || !$repoInfo?.head_branch}
        title={m.toolbar_push()}
        onclick={handlePush}
      >
        <span class="nf">{"\uF062"}</span> {m.toolbar_push()}
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
    background: rgba(255, 255, 255, 0.04);
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
    background: rgba(255, 255, 255, 0.08);
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
    background: rgba(255, 255, 255, 0.06);
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
    background: rgba(255, 255, 255, 0.1);
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

  /* ── Terminal split button ── */

  .terminal-split {
    position: relative;
    display: flex;
    align-items: stretch;
  }

  .terminal-left {
    border-radius: 6px 0 0 6px;
    border-right: none;
    min-width: unset;
    padding: 3px 8px;
  }

  .terminal-right {
    border-radius: 0 6px 6px 0;
    color: var(--text-secondary);
    min-width: unset;
    padding: 0 6px;
  }

  .chevron {
    font-size: 9px;
  }

  .terminal-menu {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 100;
    min-width: 180px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    padding: 4px 0;
    margin-top: 2px;
  }

  .terminal-menu-item {
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

  .terminal-menu-item:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .terminal-menu-item .menu-icon {
    width: 16px;
    text-align: center;
    color: var(--text-secondary);
  }

  .terminal-menu-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }

  .provider-label {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .provider-version {
    font-size: 10px;
    color: var(--text-secondary);
  }
</style>
