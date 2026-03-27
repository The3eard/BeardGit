<script lang="ts">
  import ProjectTab from "./ProjectTab.svelte";
  import AddProjectMenu from "./AddProjectMenu.svelte";
  import {
    openProjects,
    activeProjectIndex,
    activeProject,
    switchProjectTab,
    closeProjectTab,
    toggleAddMenu,
  } from "$lib/stores/projects";
  import { repoInfo } from "$lib/stores/repo";
  import { fetchRemote, pullRemote, pushRemote } from "$lib/api/tauri";
  import * as m from "$lib/paraglide/messages";

  let tabsRef = $state<HTMLDivElement | null>(null);
  let fetchInProgress = $state(false);
  let pullInProgress = $state(false);
  let pushInProgress = $state(false);

  function handleWheel(e: WheelEvent) {
    if (tabsRef) {
      e.preventDefault();
      tabsRef.scrollLeft += e.deltaY;
    }
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

<div class="tab-bar">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="tabs-scroll" bind:this={tabsRef} onwheel={handleWheel}>
    {#each $openProjects as project, i}
      <ProjectTab
        {project}
        isActive={i === $activeProjectIndex}
        index={i}
        onSwitch={switchProjectTab}
        onClose={closeProjectTab}
      />
    {/each}
  </div>

  <div class="add-button-wrapper">
    <button class="add-btn" onclick={toggleAddMenu} title="Add project">{"\uEA60"}</button>
    <AddProjectMenu />
  </div>

  {#if $activeProject}
    <div class="actions">
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
    </div>
  {/if}
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
    -webkit-app-region: drag;
  }

  .tabs-scroll {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    flex: 1;
    min-width: 0;
    scrollbar-width: none;
    -webkit-app-region: no-drag;
  }

  .tabs-scroll::-webkit-scrollbar {
    display: none;
  }

  .add-button-wrapper {
    position: relative;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    -webkit-app-region: no-drag;
  }

  .add-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    font-family: var(--font-icons);
    line-height: 1;
    cursor: pointer;
    border-radius: 6px;
    transition: background 0.15s;
    padding: 3px 8px;
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
    -webkit-app-region: no-drag;
  }

  .action-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 11px;
    cursor: pointer;
    transition: background 0.15s;
    min-width: 44px;
    text-align: center;
  }

  .action-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
