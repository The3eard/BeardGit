<script lang="ts">
  import ProviderSetup from "../auth/ProviderSetup.svelte";
  import AppearanceSettings from "./AppearanceSettings.svelte";
  import RepositorySettings from "./RepositorySettings.svelte";
  import * as m from "$lib/paraglide/messages";

  type SettingsSection = { labelKey: () => string; id: string; wip?: boolean };

  const sections: SettingsSection[] = [
    { labelKey: () => m.settings_connection(), id: "connection" },
    { labelKey: () => m.settings_appearance(), id: "appearance" },
    { labelKey: () => m.settings_repository(), id: "repository" },
    { labelKey: () => m.settings_editor(), id: "editor", wip: true },
  ];

  let activeSection = $state("connection");
</script>

<div class="settings-page">
  <nav class="settings-nav">
    <div class="nav-title">{m.settings_title()}</div>
    {#each sections as section}
      <button
        class="settings-nav-item"
        class:active={activeSection === section.id}
        onclick={() => activeSection = section.id}
      >
        {section.labelKey()}
        {#if section.wip}
          <span class="wip-tag">{m.settings_wip()}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="settings-content">
    {#if activeSection === "connection"}
      <ProviderSetup />
    {:else if activeSection === "appearance"}
      <AppearanceSettings />
    {:else if activeSection === "repository"}
      <RepositorySettings />
    {:else}
      <div class="wip-section">
        <div class="wip-icon">&#128679;</div>
        <h3>{m.settings_coming_soon()}</h3>
        <p>{m.settings_section_unavailable({ section: activeSection })}</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-page {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .settings-nav {
    width: clamp(140px, 14vw, 200px);
    min-width: 0;
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    padding: 16px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-title {
    padding: 4px 16px 12px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .settings-nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 16px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s;
  }

  .settings-nav-item:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .settings-nav-item.active {
    background: rgba(88, 166, 255, 0.1);
    color: var(--accent-blue);
  }

  .wip-tag {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: 0;
  }

  .wip-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 8px;
    color: var(--text-secondary);
  }

  .wip-icon {
    font-size: 36px;
    opacity: 0.5;
  }

  .wip-section h3 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .wip-section p {
    font-size: 13px;
    margin: 0;
  }
</style>
