<!--
  AdvancedSettings.svelte — escape-hatch category.

  Bundles three clusters of low-use-but-important operations under
  one roof so the other 6 categories stay focused:

   1. **Updates** — migrated verbatim from the old
      `UpdateSettings.svelte`. Check-for-updates + install + auto-
      check toggle, all wired to the existing `autoUpdate` store.
   2. **Diagnostics** — "Open log directory" button that shells out
      to the host file manager via `open_log_directory` IPC, plus the
      log-level selector (`get_log_level` / `set_log_level`), which
      takes effect live with no restart.
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
      id: "advanced.log-level",
      label: "Log level",
      description:
        "How much detail BeardGit writes to its log file — error, info, or debug. Applies immediately, no restart. Logging verbosity for diagnostics and bug reports.",
      category: "advanced",
      anchor: "log-level",
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
    detectOs,
    installUpdate,
    relaunchApp,
    resetAutoUpdateState,
    updateAvailableMessage,
    type AutoUpdateOs,
  } from "$lib/stores/autoUpdate";
  import {
    getAutoCheckUpdates,
    setAutoCheckUpdates,
    openLogDirectory,
    clearLayoutCache,
    getLogLevel,
    setLogLevel,
    LOG_LEVELS,
    type LogLevel,
  } from "$lib/api/tauri";
  import { addToast } from "$lib/stores/toast";
  import { Card, SettingSection, FormRow, Button, Switch } from "$lib/components/ui";
  import { formatRelativeTimeMs } from "$lib/utils/time";

  /* Hardcoded copy of the updater endpoint configured in
     `src-tauri/tauri.conf.json` → `plugins.updater.endpoints[0]`. The
     plugin doesn't expose this at runtime, but surfacing it in the
     diagnostics line lets developers immediately see *which* URL the
     check tried to fetch — invaluable when debugging "is the system
     working" against a release pipeline that hasn't published
     `latest.json` yet. Update both places together. */
  const UPDATE_ENDPOINT_URL =
    "https://github.com/The3eard/BeardGit/releases/latest/download/latest.json";

  const appVersion: string =
    (import.meta.env.VITE_APP_VERSION as string | undefined) ?? "0.0.0";

  let autoCheck = $state(true);
  let checking = $state(false);
  let installing = $state(false);
  let clearingCache = $state(false);
  let openingLogs = $state(false);
  let logLevel = $state<LogLevel>("info");
  let savingLogLevel = $state(false);
  /* The selector starts on the default, which is indistinguishable from a
     loaded value. Gate it until hydration resolves so the user can't act
     on a stale reading — and so `onMount` can't overwrite a choice made
     inside that window. */
  let logLevelReady = $state(false);

  /* Drives the unsigned-build notice in the "available" helper line.
     This is the only place a Settings-initiated install can surface it:
     on Windows the NSIS installer kills the process mid-install, so
     there is no post-download surface to fall back on. */
  let os = $state<AutoUpdateOs>("other");

  const status = $derived($autoUpdateState.status);
  const availableVersion = $derived($autoUpdateState.availableVersion ?? "");
  const rawErrorMessage = $derived($autoUpdateState.error ?? "");
  const lastCheckedAt = $derived($autoUpdateState.lastCheckedAt);

  /* The Tauri updater plugin surfaces low-level failures verbatim
     (e.g. "could not fetch json" when the latest.json endpoint 404s,
     "the network has temporary issue" for offline). Those strings
     leak implementation detail into a setting most users will never
     debug, so map the recognisable "endpoint unreachable" shapes to
     a localized hint and keep the raw text only for unexpected ones. */
  const errorMessage = $derived.by(() => {
    const raw = rawErrorMessage.toLowerCase();
    if (
      raw.includes("could not fetch json") ||
      raw.includes("network") ||
      raw.includes("404") ||
      raw.includes("unexpected token") ||
      raw === ""
    ) {
      return m.update_server_unreachable();
    }
    return rawErrorMessage;
  });

  onMount(() => {
    // Three independent loads, deliberately not chained. The log-level
    // selector is gated on its own fetch resolving, so serializing it
    // behind either of the others would mean an unrelated hang leaves the
    // select disabled for good.
    void detectOs().then((v) => (os = v));

    void getAutoCheckUpdates()
      .then((v) => (autoCheck = v))
      // IPC unavailable (tests / dev) — keep the default.
      .catch(() => {});

    void getLogLevel()
      .then((persisted) => {
        if ((LOG_LEVELS as readonly string[]).includes(persisted)) {
          logLevel = persisted as LogLevel;
        }
      })
      // IPC unavailable (tests / dev) — keep the default.
      .catch(() => {})
      // Lift the gate either way: a failed read must not lock the selector.
      .finally(() => (logLevelReady = true));
  });

  /** Localized label for a level, so the option text isn't a raw enum. */
  function logLevelLabel(level: LogLevel): string {
    if (level === "error") return m.settings_advanced_log_level_error();
    if (level === "debug") return m.settings_advanced_log_level_debug();
    return m.settings_advanced_log_level_info();
  }

  async function handleLogLevelChange(event: Event) {
    const next = (event.target as HTMLSelectElement).value as LogLevel;
    const previous = logLevel;
    logLevel = next;
    savingLogLevel = true;
    try {
      await setLogLevel(next);
    } catch (e) {
      // Revert so the selector never claims a level the backend rejected.
      logLevel = previous;
      addToast({
        message: `${m.settings_advanced_log_level_failed()}: ${e}`,
        type: "error",
      });
    } finally {
      savingLogLevel = false;
    }
  }

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
      const outcome = await installUpdate();
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
              ? updateAvailableMessage(availableVersion, os)
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
          <Button variant="neutral" size="sm" onclick={handleDismissError}>
            {m.toast_dismiss()}
          </Button>
        {/if}
        <Button
          variant="neutral"
          size="sm"
          loading={checking}
          disabled={status === "downloading"}
          onclick={handleCheck}
        >
          {m.update_check_button()}
        </Button>
      </FormRow>

      {#if lastCheckedAt || status === "error"}
        <div class="update-diagnostics" data-testid="update-diagnostics">
          {#if lastCheckedAt}
            <div class="diag-line">
              {m.update_last_checked({
                when: formatRelativeTimeMs(lastCheckedAt),
              })}
            </div>
          {/if}
          {#if status === "error" && rawErrorMessage}
            <div class="diag-line diag-error" data-testid="update-error-detail">
              {m.update_error_detail({ message: rawErrorMessage })}
            </div>
          {/if}
          <div class="diag-line diag-endpoint">
            {m.update_check_endpoint({ url: UPDATE_ENDPOINT_URL })}
          </div>
        </div>
      {/if}
    </div>

    <div data-setting-anchor="update-auto">
      <FormRow
        label={m.update_settings_auto_check_label()}
        for="update-auto-toggle"
        helperText={m.update_settings_auto_check_hint()}
      >
        <Switch
          id="update-auto-toggle"
          testid="update-auto-toggle"
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
          variant="neutral"
          size="sm"
          loading={openingLogs}
          onclick={handleOpenLogs}
        >
          {m.settings_advanced_log_directory_button()}
        </Button>
      </FormRow>
    </div>

    <div data-setting-anchor="log-level">
      <FormRow
        label={m.settings_advanced_log_level_label()}
        for="log-level-select"
        helperText={m.settings_advanced_log_level_hint()}
      >
        <select
          id="log-level-select"
          class="bg-select"
          data-testid="log-level-select"
          value={logLevel}
          disabled={savingLogLevel || !logLevelReady}
          onchange={handleLogLevelChange}
        >
          {#each LOG_LEVELS as level (level)}
            <option value={level}>{logLevelLabel(level)}</option>
          {/each}
        </select>
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
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .update-diagnostics {
    margin-top: 4px;
    padding-left: 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .diag-line {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .diag-line.diag-error {
    font-family: var(--font-mono);
    color: var(--accent-red);
    word-break: break-word;
  }

  /* Matches the select styling used by Editor + General settings. */
  .bg-select {
    padding: 5px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    outline: none;
    cursor: pointer;
    min-width: 96px;
    font-family: inherit;
  }

  .bg-select:focus {
    border-color: var(--accent-primary);
  }

  .diag-line.diag-endpoint {
    font-family: var(--font-mono);
    word-break: break-all;
  }
</style>
