<!--
  AdvancedSettings.svelte — escape-hatch category.

  Bundles three clusters of low-use-but-important operations under
  one roof so the other 6 categories stay focused:

   1. **Updates** — migrated verbatim from the old
      `UpdateSettings.svelte`. Check-for-updates + install + auto-
      check toggle, all wired to the existing `autoUpdate` store.
   2. **Diagnostics** — "Open log directory" button that shells out
      to the host file manager via `open_log_directory` IPC.
   3. **Cache management** — "Clear graph layout cache" button that
      wipes `<config_dir>/beardgit/layouts/` via the new
      `clear_layout_cache` IPC.

  Everything sits inside shared `Card` + `SettingSection` +
  `FormRow` + `Button` primitives.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  export const settingsIndex: SettingDescriptor[] = [
    {
      id: "advanced.update-check",
      label: "Check for updates",
      description:
        "Manually poll the update server and — if a new version is out — kick off the install flow.",
      category: "advanced",
      anchor: "update-check",
    },
    {
      id: "advanced.update-auto",
      label: "Automatic update checks",
      description:
        "Whether BeardGit polls for new releases on startup (the in-app updater).",
      category: "advanced",
      anchor: "update-auto",
    },
    {
      id: "advanced.log-directory",
      label: "Open log directory",
      description:
        "Reveals the BeardGit log folder in the system file manager — useful for bug reports.",
      category: "advanced",
      anchor: "log-directory",
    },
    {
      id: "advanced.clear-cache",
      label: "Clear graph layout cache",
      description:
        "Deletes cached graph layouts. They rebuild on the next repo open — use if the graph looks stale.",
      category: "advanced",
      anchor: "clear-cache",
    },
  ];
</script>

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
    openLogDirectory,
    clearLayoutCache,
  } from "$lib/api/tauri";
  import { addToast } from "$lib/stores/toast";
  import { Card, SettingSection, FormRow, Button } from "$lib/components/ui";

  const appVersion: string =
    (import.meta.env.VITE_APP_VERSION as string | undefined) ?? "0.0.0";

  let autoCheck = $state(true);
  let checking = $state(false);
  let installing = $state(false);
  let clearingCache = $state(false);
  let openingLogs = $state(false);

  const status = $derived($autoUpdateState.status);
  const availableVersion = $derived($autoUpdateState.availableVersion ?? "");
  const errorMessage = $derived($autoUpdateState.error ?? "");

  onMount(async () => {
    try {
      autoCheck = await getAutoCheckUpdates();
    } catch {
      // IPC unavailable (tests / dev) — keep the default.
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
        await relaunchApp();
      }
    } finally {
      installing = false;
    }
  }

  async function handleToggleAutoCheck(event: Event) {
    const input = event.target as HTMLInputElement;
    autoCheck = input.checked;
    try {
      await setAutoCheckUpdates(autoCheck);
    } catch {
      // Revert on persistence failure.
      autoCheck = !autoCheck;
      input.checked = autoCheck;
    }
  }

  function handleDismissError() {
    resetAutoUpdateState();
  }

  async function handleClearCache() {
    clearingCache = true;
    try {
      await clearLayoutCache();
      addToast({
        message: m.settings_advanced_clear_cache_done(),
        type: "success",
      });
    } catch (e) {
      addToast({
        message: `${m.settings_advanced_clear_cache_failed()}: ${e}`,
        type: "error",
      });
    } finally {
      clearingCache = false;
    }
  }

  async function handleOpenLogs() {
    openingLogs = true;
    try {
      await openLogDirectory();
    } catch (e) {
      addToast({
        message: `${m.settings_advanced_log_directory_failed()}: ${e}`,
        type: "error",
      });
    } finally {
      openingLogs = false;
    }
  }
</script>

<Card
  title={m.settings_advanced_updates_title()}
  description={m.update_settings_auto_check_hint()}
>
  <SettingSection title={m.update_settings_title()}>
    <FormRow label={m.update_current_version()}>
      <span class="version-badge" data-testid="update-current-version">
        {appVersion}
      </span>
    </FormRow>

    <div data-setting-anchor="update-check">
      <FormRow
        label={m.update_check_button()}
        helperText={status === "checking" || checking
          ? m.update_checking()
          : status === "up_to_date"
            ? m.update_up_to_date()
            : status === "available"
              ? m.update_available({ version: availableVersion })
              : status === "downloading"
                ? m.update_downloading({ percent: "0" })
                : status === "ready"
                  ? m.update_ready()
                  : status === "error"
                    ? errorMessage || m.update_error()
                    : ""}
      >
        {#if status === "available"}
          <Button
            variant="primary"
            size="sm"
            loading={installing}
            onclick={handleInstall}
          >
            {m.update_install()}
          </Button>
        {:else if status === "error"}
          <Button variant="ghost" size="sm" onclick={handleDismissError}>
            {m.toast_dismiss()}
          </Button>
        {/if}
        <Button
          variant="secondary"
          size="sm"
          loading={checking}
          disabled={status === "downloading"}
          onclick={handleCheck}
        >
          {m.update_check_button()}
        </Button>
      </FormRow>
    </div>

    <div data-setting-anchor="update-auto">
      <FormRow
        label={m.update_settings_auto_check_label()}
        for="update-auto-toggle"
        helperText={m.update_settings_auto_check_hint()}
      >
        <input
          id="update-auto-toggle"
          type="checkbox"
          class="bg-checkbox"
          data-testid="update-auto-toggle"
          checked={autoCheck}
          onchange={handleToggleAutoCheck}
        />
      </FormRow>
    </div>
  </SettingSection>
</Card>

<Card
  title={m.settings_advanced_diagnostics_title()}
  description={m.settings_advanced_diagnostics_description()}
>
  <SettingSection title={m.settings_advanced_diagnostics_title()}>
    <div data-setting-anchor="log-directory">
      <FormRow
        label={m.settings_advanced_log_directory_label()}
        helperText={m.settings_advanced_log_directory_hint()}
      >
        <Button
          variant="secondary"
          size="sm"
          loading={openingLogs}
          onclick={handleOpenLogs}
        >
          {m.settings_advanced_log_directory_button()}
        </Button>
      </FormRow>
    </div>

    <div data-setting-anchor="clear-cache">
      <FormRow
        label={m.settings_advanced_clear_cache_label()}
        helperText={m.settings_advanced_clear_cache_description()}
      >
        <Button
          variant="danger"
          size="sm"
          loading={clearingCache}
          onclick={handleClearCache}
        >
          {m.settings_advanced_clear_cache_button()}
        </Button>
      </FormRow>
    </div>
  </SettingSection>
</Card>

<style>
  .version-badge {
    padding: 4px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
  }

  .bg-checkbox {
    accent-color: var(--accent-blue);
    width: 16px;
    height: 16px;
    cursor: pointer;
  }
</style>
