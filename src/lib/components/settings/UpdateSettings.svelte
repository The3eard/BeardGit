<!--
  UpdateSettings.svelte — Settings → Updates section.

  Surfaces three pieces of UX around the auto-update lifecycle:

  - The current app version, inlined at build time via
    `VITE_APP_VERSION` so the panel works even when the IPC layer is
    unreachable (see `vite.config.js`).
  - A **Check for updates** button that drives the existing
    `checkForUpdates()` helper and — when an update is available — a
    secondary **Install** button that kicks off `startInstallFlow()`
    (including the re-auth apology on macOS / Windows).
  - A toggle mirroring the `auto_check_updates` preference, wired to
    the `get_auto_check_updates` / `set_auto_check_updates` IPC pair.

  MT-5 will reshuffle the settings IA; this component is intentionally
  a single flat card so that reorganisation is a move-and-rewire rather
  than a rewrite.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import {
    autoUpdateState,
    checkForUpdates,
    startInstallFlow,
    relaunchApp,
    resetAutoUpdateState,
  } from "$lib/stores/autoUpdate";
  import {
    getAutoCheckUpdates,
    setAutoCheckUpdates,
  } from "$lib/api/tauri";

  /**
   * App version resolved from `package.json` at bundle time. Vite
   * substitutes this via the `define` block; the `?? "0.0.0"` fallback
   * keeps the tests happy when the env isn't populated.
   */
  const appVersion: string =
    (import.meta.env.VITE_APP_VERSION as string | undefined) ?? "0.0.0";

  let autoCheck = $state(true);
  let checking = $state(false);
  let installing = $state(false);

  // Local mirror of the store's status so template conditionals stay
  // declarative without sprinkling `$autoUpdateState` everywhere.
  let status = $derived($autoUpdateState.status);
  let availableVersion = $derived($autoUpdateState.availableVersion ?? "");
  let errorMessage = $derived($autoUpdateState.error ?? "");

  onMount(async () => {
    try {
      autoCheck = await getAutoCheckUpdates();
    } catch {
      // IPC unavailable (tests / dev-mode) — leave the default value.
    }
  });

  async function handleCheck() {
    checking = true;
    try {
      await checkForUpdates();
    } finally {
      checking = false;
    }
  }

  async function handleInstall() {
    installing = true;
    try {
      const outcome = await startInstallFlow();
      if (outcome === "ready") {
        // Download finished inline (Linux or dismissed reauth) — relaunch
        // immediately so the user isn't left wondering what to do next.
        await relaunchApp();
      }
    } finally {
      installing = false;
    }
  }

  async function handleToggle(event: Event) {
    const input = event.target as HTMLInputElement;
    autoCheck = input.checked;
    try {
      await setAutoCheckUpdates(autoCheck);
    } catch {
      // Revert the visual state if persistence failed.
      autoCheck = !autoCheck;
      input.checked = autoCheck;
    }
  }

  function handleDismissError() {
    resetAutoUpdateState();
  }
</script>

<div class="update-card" data-testid="update-settings">
  <h2 class="card-title">{m.update_settings_title()}</h2>

  <div class="setting-row">
    <div class="setting-label-group">
      <span class="setting-label">{m.update_current_version()}</span>
    </div>
    <span class="version-badge" data-testid="update-current-version">
      {appVersion}
    </span>
  </div>

  <div class="setting-row">
    <div class="setting-label-group">
      <span class="setting-label">{m.update_check_button()}</span>
      <span class="status-text" data-testid="update-status">
        {#if status === "checking" || checking}
          {m.update_checking()}
        {:else if status === "up_to_date"}
          {m.update_up_to_date()}
        {:else if status === "available"}
          {m.update_available({ version: availableVersion })}
        {:else if status === "downloading"}
          {m.update_downloading({ percent: "0" })}
        {:else if status === "ready"}
          {m.update_ready()}
        {:else if status === "error"}
          {errorMessage || m.update_error()}
        {/if}
      </span>
    </div>
    <div class="setting-actions">
      {#if status === "available"}
        <button
          type="button"
          class="btn btn-primary"
          data-testid="update-install-btn"
          disabled={installing}
          onclick={handleInstall}
        >
          {m.update_install()}
        </button>
      {:else if status === "error"}
        <button
          type="button"
          class="btn btn-cancel"
          data-testid="update-error-dismiss"
          onclick={handleDismissError}
        >
          {m.toast_dismiss()}
        </button>
      {/if}
      <button
        type="button"
        class="btn btn-cancel"
        data-testid="update-check-btn"
        disabled={checking || status === "downloading"}
        onclick={handleCheck}
      >
        {m.update_check_button()}
      </button>
    </div>
  </div>

  <div class="setting-row">
    <div class="setting-label-group">
      <label class="setting-label" for="update-auto-toggle">
        {m.update_settings_auto_check_label()}
      </label>
      <span class="hint-text">{m.update_settings_auto_check_hint()}</span>
    </div>
    <input
      id="update-auto-toggle"
      type="checkbox"
      data-testid="update-auto-toggle"
      checked={autoCheck}
      onchange={handleToggle}
    />
  </div>
</div>

<style>
  .update-card {
    max-width: 480px;
    margin: 48px auto;
    padding: 32px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .card-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .setting-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }

  .setting-row:last-child {
    border-bottom: none;
  }

  .setting-label-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    flex: 1;
  }

  .setting-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .status-text {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .hint-text {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .setting-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .version-badge {
    padding: 4px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
  }

  input[type="checkbox"] {
    accent-color: var(--accent-blue);
    width: 16px;
    height: 16px;
    cursor: pointer;
    margin-top: 2px;
  }
</style>
