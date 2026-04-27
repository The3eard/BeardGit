<script lang="ts">
  import { onMount } from "svelte";
  import type { RecentRepo } from "$lib/types";
  import { getRecentRepos } from "$lib/api/tauri";
  import { openProjectTab, openFolderAsProject, addMenuOpen } from "$lib/stores/projects";
  import { openCloneDialog } from "$lib/stores/cloneDialog";
  import { get } from "svelte/store";
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

  function handleCloneProject() {
    addMenuOpen.set(false);
    openCloneDialog();
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
    if (!get(addMenuOpen)) return;
    // Ignore clicks on the + button itself (it toggles via its own handler)
    const target = e.target as HTMLElement;
    if (target.closest(".add-button-wrapper")) return;
    addMenuOpen.set(false);
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
      <span class="menu-icon">{"\uF07C"}</span>
      <span>{m.tab_add_open_folder()}</span>
    </button>

    <button class="menu-item" onclick={handleCloneProject}>
      <span class="menu-icon">{"\uF019"}</span>
      <span>{m.tab_add_clone_project()}</span>
    </button>

    <div class="menu-divider"></div>

    <div class="menu-section-label">{m.tab_add_recent()}</div>

    {#if recentRepos.length === 0}
      <div class="menu-empty">{m.tab_add_no_recent()}</div>
    {:else}
      {#each recentRepos as repo}
        <button class="menu-item" onclick={() => handleRecentClick(repo.path)} title={repo.path}>
          <span class="menu-icon">{"\uF07C"}</span>
          <span>{repo.name}</span>
        </button>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .add-menu {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 100;
    min-width: 200px;
    max-width: 320px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3); /* beardgit:allow-hex: shadow neutral always-dark */
    padding: 4px 0;
    margin-top: 2px;
  }

  .menu-item {
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
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu-item:hover {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }

  .menu-icon {
    font-family: var(--font-icons);
    font-size: 14px;
    width: 16px;
    text-align: center;
    flex-shrink: 0;
    color: var(--accent-blue);
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
