<script lang="ts">
  import ProviderSetup from "../auth/ProviderSetup.svelte";
  import AppearanceSettings from "./AppearanceSettings.svelte";
  import GitConfigSettings from "./GitConfigSettings.svelte";
  import AiSettings from "./AiSettings.svelte";
  import UpdateSettings from "./UpdateSettings.svelte";
  import CliAuthSection from "./CliAuthSection.svelte";
  import ConnectionHowTo from "./ConnectionHowTo.svelte";
  import { pendingSettingsSection } from "$lib/stores/navigation";
  import * as m from "$lib/paraglide/messages";

  type SettingsSection = { labelKey: () => string; id: string; wip?: boolean };

  const sections: SettingsSection[] = [
    { labelKey: () => m.settings_connection(), id: "connection" },
    { labelKey: () => m.settings_appearance(), id: "appearance" },
    { labelKey: () => m.settings_git_config(), id: "git-config" },
    { labelKey: () => m.ai_settings_title(), id: "ai" },
    { labelKey: () => m.update_settings_title(), id: "updates" },
  ];

  let activeSection = $state("connection");

  // Deep-link bridge — when a statusbar slot (or anything else) wants to
  // open a specific Settings sub-section, it writes the id to the
  // `pendingSettingsSection` store and flips the top-level view to
  // "settings". We mirror that value into local state exactly once per
  // request, then clear the store so a subsequent manual navigation to
  // Settings doesn't re-trigger the deep-link.
  $effect(() => {
    const pending = $pendingSettingsSection;
    if (pending && sections.some((s) => s.id === pending)) {
      activeSection = pending;
      pendingSettingsSection.set(null);
    }
  });
</script>

<div class="settings-page" data-testid="settings-page">
  <nav class="settings-nav">
    <div class="nav-title">{m.settings_title()}</div>
    {#each sections as section}
      <button
        class="settings-nav-item"
        class:active={activeSection === section.id}
        data-testid="settings-nav-{section.id}"
        onclick={() => activeSection = section.id}
      >
        {section.labelKey()}
        {#if section.wip}
          <span class="wip-tag">{m.settings_wip()}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="settings-content" data-testid="settings-content">
    {#if activeSection === "connection"}
      <ConnectionHowTo />
      <div class="auth-section">
        <h3 class="auth-section-title">{m.settings_token_auth()}</h3>
        <ProviderSetup />
      </div>
      <div class="auth-section">
        <CliAuthSection />
      </div>
    {:else if activeSection === "appearance"}
      <AppearanceSettings />
    {:else if activeSection === "git-config"}
      <GitConfigSettings />
    {:else if activeSection === "ai"}
      <AiSettings />
    {:else if activeSection === "updates"}
      <UpdateSettings />
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

  .auth-section {
    display: flex;
    flex-direction: column;
  }

  .auth-section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-secondary);
    padding: 16px 48px 0;
    margin: 0;
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
