<script lang="ts">
  import { onMount } from "svelte";
  import type { RecentRepo } from "$lib/types";
  import { getRecentRepos } from "$lib/api/tauri";
  import { openProjectTab, openFolderAsProject, addMenuOpen } from "$lib/stores/projects";
  import * as m from "$lib/paraglide/messages";

  let recentRepos = $state<RecentRepo[]>([]);
  let menuRef = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if ($addMenuOpen) {
      loadRecent();
    }
  });

  async function loadRecent() {
    recentRepos = await getRecentRepos();
  }

  function handleOpenFolder() {
    addMenuOpen.set(false);
    openFolderAsProject();
  }

  async function handleRecentClick(path: string) {
    addMenuOpen.set(false);
    await openProjectTab(path);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      addMenuOpen.set(false);
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (menuRef && !menuRef.contains(e.target as Node)) {
      addMenuOpen.set(false);
    }
  }

  onMount(() => {
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeydown);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

{#if $addMenuOpen}
  <div class="add-menu" bind:this={menuRef}>
    <button class="menu-item" onclick={handleOpenFolder}>
      {m.tab_add_open_folder()}
    </button>

    <div class="menu-divider"></div>

    <div class="menu-section-label">{m.tab_add_recent()}</div>

    {#if recentRepos.length === 0}
      <div class="menu-empty">{m.tab_add_no_recent()}</div>
    {:else}
      {#each recentRepos as repo}
        <button class="menu-item" onclick={() => handleRecentClick(repo.path)} title={repo.path}>
          {repo.name}
        </button>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .add-menu {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 100;
    min-width: 200px;
    max-width: 320px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    padding: 4px 0;
    margin-top: 2px;
  }

  .menu-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu-item:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .menu-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }

  .menu-section-label {
    padding: 4px 12px 2px;
    font-size: 11px;
    color: var(--text-secondary);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .menu-empty {
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
