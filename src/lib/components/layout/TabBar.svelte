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
    switchSegment,
    closeTerminalSegment,
    closeProjectSegment,
  } from "$lib/stores/projects";
  import { openTabs, activeTabIndex, openTerminalTab, openStandaloneTerminal, openAiTerminalTab } from "$lib/stores/tabs";
  import { repoInfo } from "$lib/stores/repo";
  import { aiProviders } from "$lib/stores/ai";
  import type { AiProviderKind } from "$lib/types";
  import { fetchRemote, pullRemote, pushRemote } from "$lib/api/tauri";
  import * as m from "$lib/paraglide/messages";

  import { onMount } from "svelte";

  const PROVIDER_NAMES: Record<string, string> = {
    claude_code: "Claude Code",
    codex: "Codex",
    open_code: "OpenCode",
  };

  const PROVIDER_COLORS: Record<string, string> = {
    claude_code: "#d97757",
    codex: "#ffffff",
    open_code: "#8b8b8b",
  };

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
    const name = PROVIDER_NAMES[kind] ?? kind;
    await openAiTerminalTab(getActiveCwd(), `${name} · ${getActiveLabel()}`, kind);
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
    const home = await getHomePath();

    // Already viewing a terminal → open new standalone in ~
    if (
      tab?.kind === "terminal" ||
      (tab?.kind === "composite" && tab.activeSegment === "terminal")
    ) {
      await openStandaloneTerminal(home, m.tab_terminal());
      return;
    }

    // Composite tab on project segment → switch to existing terminal
    if (tab?.kind === "composite" && tab.activeSegment === "project") {
      switchSegment($activeTabIndex, "terminal");
      return;
    }

    // Project tab → promote to composite (linked terminal)
    if (tab?.kind === "project") {
      const name = tab.project.path.split("/").pop() ?? "Terminal";
      await openTerminalTab(tab.project.path, `${m.tab_terminal()} · ${name}`);
      return;
    }

    // No project context → standalone in ~
    await openStandaloneTerminal(home, m.tab_terminal());
  }

  async function handleFetch() {
    if (fetchInProgress) return;
    fetchInProgress = true;
    try {
      await fetchRemote("origin");
    } finally {
      fetchInProgress = false;
    }
  }

  async function handlePull() {
    if (pullInProgress || !$repoInfo?.head_branch) return;
    pullInProgress = true;
    try {
      await pullRemote("origin", $repoInfo.head_branch);
    } finally {
      pullInProgress = false;
    }
  }

  async function handlePush() {
    if (pushInProgress || !$repoInfo?.head_branch) return;
    pushInProgress = true;
    try {
      await pushRemote("origin", $repoInfo.head_branch);
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
          terminal={tab.terminal}
          activeSegment={tab.activeSegment}
          isActiveTab={i === $activeTabIndex}
          onSwitchSegment={(segment) => {
            switchSegment(i, segment);
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
          onCloseTerminal={() => closeTerminalSegment(i)}
        />
      {/if}
    {/each}
  </div>

  <div class="add-button-wrapper">
    <button class="add-btn" onclick={toggleAddMenu} title="Add project"><span class="nf">{"\uF067"}</span></button>
    <AddProjectMenu />
  </div>

  <div class="actions">
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
                {#if provider.kind === "claude_code"}
                  <svg class="brand-icon" viewBox="0 0 1200 1200" fill={PROVIDER_COLORS.claude_code}>
                    <path d="M233.96 800.21L468.64 668.54l3.95-11.44-3.95-6.36h-11.44l-39.22-2.42-134.09-3.62-116.3-4.83L54.93 633.83l-26.58-5.96L0 592.75l2.74-17.48 23.84-16.03 34.15 2.98 75.46 5.15 113.24 7.81 82.15 4.83 121.69 12.65 19.33 0 2.74-7.81-6.6-4.83-5.16-4.83L346.39 495.79 219.54 411.87l-66.44-48.32-35.92-24.48-18.12-22.95-7.81-50.1 32.62-35.92 43.81 2.98 11.19 2.98 44.38 34.15 94.79 73.37 123.79 91.17 18.12 15.06 7.25-5.15.89-3.63-8.13-13.62-67.46-121.69-71.84-123.79-31.97-51.3-8.46-30.76c-2.98-12.64-5.15-23.27-5.15-36.24l37.13-50.42 20.54-6.6 49.53 6.6 20.86 18.12 30.76 70.39 49.85 110.82 77.32 150.68 22.63 44.7 12.08 41.4 4.51 12.64h7.81v-7.25l6.36-84.89 11.76-104.21 11.44-134.09 3.94-37.77 18.68-45.26 37.13-24.48 29.03 13.85 23.84 34.15-3.3 22.07-14.17 92.13-27.79 144.32-18.12 96.64h10.55l12.08-12.08 48.89-64.91 82.15-102.68 36.24-40.75 42.28-45.02 27.14-21.42h51.3l37.77 56.13-16.91 58.0-52.83 67.01-43.81 56.78-62.82 84.56-39.22 67.65 3.62 5.4 9.34-0.89 141.91-30.2 76.67-13.85 91.49-15.71 41.4 19.33 4.51 19.65-16.27 40.19-97.85 24.16-114.77 22.95-170.9 40.43-2.09 1.53 2.42 2.98 76.99 7.25 32.94 1.77 80.62 0 150.12 11.19 39.22 25.93 23.52 31.73-3.95 24.16-60.4 30.76-81.5-19.33-190.23-45.26-65.46-16.27-9.02 0v5.4l54.36 53.15 99.62 89.96 124.75 115.97 6.36 28.67-16.03 22.63-16.91-2.42-109.61-82.47-42.28-37.13-95.76-80.62h-5.66v8.46l22.07 32.3 116.54 175.16 6.04 53.72-8.46 17.48-30.2 10.55-33.18-6.04-68.21-95.76-70.39-107.84-56.78-96.64-6.93 3.95-33.5 360.89-15.71 18.44-36.24 13.85-30.2-22.95-16.03-37.13 16.03-73.37 19.33-95.76 15.7-76.11 14.17-94.55 8.46-31.41-.56-2.09-6.93.89-71.23 97.85-108.4 146.5-85.77 91.81-20.54 8.13-35.6-18.44 3.3-32.94 20.14-29.32 118.72-150.91 71.6-93.58 46.23-53.95-.32-7.81h-2.74L205.29 929.4l-56.13 7.25-24.16-22.63 2.98-37.13 11.44-12.08 94.79-65.24z"/>
                  </svg>
                {:else if provider.kind === "codex"}
                  <svg class="brand-icon" viewBox="0 0 24 24" fill={PROVIDER_COLORS.codex}>
                    <path d="M22.418 9.822a5.903 5.903 0 0 0-.52-4.91 6.1 6.1 0 0 0-2.822-2.48 6.007 6.007 0 0 0-3.78-.381A6.053 6.053 0 0 0 10.868.5a6.093 6.093 0 0 0-5.788 4.143 6.033 6.033 0 0 0-4.126 2.896 6.052 6.052 0 0 0 .734 7.139 5.903 5.903 0 0 0 .52 4.911 6.1 6.1 0 0 0 2.822 2.48 6.007 6.007 0 0 0 3.78.38A6.053 6.053 0 0 0 13.132 23.5a6.093 6.093 0 0 0 5.788-4.143 6.033 6.033 0 0 0 4.126-2.896 6.052 6.052 0 0 0-.734-7.139h.106Zm-9.088 12.39a4.577 4.577 0 0 1-2.924-1.05c.037-.02.1-.056.143-.081l4.855-2.803a.788.788 0 0 0 .399-.69v-6.841l2.052 1.185a.073.073 0 0 1 .04.056v5.663a4.59 4.59 0 0 1-4.564 4.56h-.001Zm-9.79-4.186a4.547 4.547 0 0 1-.548-3.073c.036.022.098.06.143.085l4.855 2.803a.792.792 0 0 0 .798 0l5.928-3.423v2.37a.073.073 0 0 1-.03.062l-4.909 2.835a4.587 4.587 0 0 1-6.237-1.66ZM2.182 7.88A4.556 4.556 0 0 1 4.571 5.87v5.768a.786.786 0 0 0 .398.69l5.929 3.421-2.052 1.186a.072.072 0 0 1-.07.005L3.868 14.1a4.59 4.59 0 0 1-1.686-6.222ZM19.2 11.67l-5.928-3.422L15.324 7.062a.072.072 0 0 1 .07-.006l4.907 2.835a4.579 4.579 0 0 1-.71 8.273v-5.768a.786.786 0 0 0-.398-.69l.007-.037Zm2.042-3.083a6.22 6.22 0 0 0-.143-.085l-4.855-2.803a.792.792 0 0 0-.798 0l-5.928 3.423V6.752a.073.073 0 0 1 .03-.063l4.909-2.829a4.583 4.583 0 0 1 6.785 4.727Zm-12.84 4.222L6.35 11.624a.073.073 0 0 1-.04-.057V5.904a4.58 4.58 0 0 1 7.49-3.51 1.17 1.17 0 0 0-.143.082l-4.855 2.803a.788.788 0 0 0-.399.69l-.001 6.841Zm1.114-2.399 2.64-1.524 2.64 1.524v3.047l-2.64 1.524-2.64-1.524v-3.047Z"/>
                  </svg>
                {:else if provider.kind === "open_code"}
                  <svg class="brand-icon" viewBox="0 0 24 24" fill={PROVIDER_COLORS.open_code}>
                    <path d="M9.4 16.6L4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4Zm5.2 0L19.2 12l-4.6-4.6L16 6l6 6-6 6-1.4-1.4Z"/>
                  </svg>
                {/if}
                <span class="provider-label">
                  {PROVIDER_NAMES[provider.kind] ?? provider.kind}
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

  .brand-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
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
