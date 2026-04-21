<!--
  ConnectionRow.svelte — single dense row inside the unified Connections
  card on the Integrations settings page (Spec 4 Phase 8).

  Dispatches on the `kind` prop:
  - `github` | `gitlab`: connected to the forge PAT store
    (`$lib/stores/provider`). Row shows a display name, connection
    status, and an action — "Manage" (expands the inline token form +
    a disconnect affordance) when one or more providers of that kind
    are connected, otherwise "Connect" (expands an inline token form).
  - `gh` | `glab`: connected to the shell CLI auth check
    (`cliCheckAuthStatus` + `cliGetAuthCommand` / `cliGetLogoutCommand`
    via a standalone terminal tab). Replaces `CliAuthSection.svelte`.

  The row is a 3-column grid (name | status | action) so the card
  stays dense — four rows fit comfortably in a single viewport.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    providerStatus,
    isConnecting,
    providerError,
    connect,
    disconnect,
    checkStatus,
  } from "$lib/stores/provider";
  import {
    cliCheckAuthStatus,
    cliGetAuthCommand,
    cliGetLogoutCommand,
    terminalWrite,
  } from "$lib/api/tauri";
  import { openStandaloneTerminal } from "$lib/stores/tabs";
  import type { CliAuthStatus, ProviderKind } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

  type Kind = ProviderKind | "gh" | "glab";

  interface Props {
    kind: Kind;
  }

  const { kind }: Props = $props();

  const isProviderKind = (k: Kind): k is ProviderKind =>
    k === "github" || k === "gitlab";

  const displayName = $derived(
    kind === "github"
      ? m.provider_github()
      : kind === "gitlab"
        ? m.provider_gitlab()
        : kind === "gh"
          ? m.settings_integrations_row_gh()
          : m.settings_integrations_row_glab(),
  );

  // ————— provider (github / gitlab) state ——————————————————————————————
  // Matching providers for this row's kind.
  const matchingProviders = $derived(
    isProviderKind(kind)
      ? $providerStatus.providers.filter((p) => p.kind === kind)
      : [],
  );
  const providerConnected = $derived(
    isProviderKind(kind) && matchingProviders.length > 0,
  );

  // Inline token form state — only relevant for github / gitlab rows.
  let formOpen = $state(false);
  let instanceUrl = $state("");
  let token = $state("");

  function defaultUrl(k: ProviderKind): string {
    return k === "github" ? "https://api.github.com" : "https://gitlab.com";
  }

  async function handleConnect(e: Event) {
    e.preventDefault();
    if (!isProviderKind(kind)) return;
    const url = instanceUrl.trim() || defaultUrl(kind);
    try {
      await connect(kind, url, token);
      token = "";
      instanceUrl = "";
      formOpen = false;
    } catch {
      /* providerError is set in store */
    }
  }

  async function handleDisconnect(url: string) {
    await disconnect(url);
  }

  // ————— CLI (gh / glab) state —————————————————————————————————————————
  let cliStatuses = $state<CliAuthStatus[]>([]);
  let cliLaunching = $state(false);

  const cliStatus = $derived(
    !isProviderKind(kind)
      ? cliStatuses.find((s) => s.tool === kind)
      : undefined,
  );
  const cliInstalled = $derived(cliStatus?.installed ?? false);
  const cliAuthenticated = $derived(cliStatus?.authenticated ?? false);
  const cliUsername = $derived(cliStatus?.username ?? null);

  async function refreshCli() {
    try {
      cliStatuses = await cliCheckAuthStatus();
    } catch {
      /* ignore */
    }
  }

  async function handleCliAuth() {
    if (isProviderKind(kind)) return;
    cliLaunching = true;
    try {
      const cmd = await cliGetAuthCommand(kind);
      const { homeDir } = await import("@tauri-apps/api/path");
      const cwd = await homeDir();
      const sessionId = await openStandaloneTerminal(
        cwd,
        `${kind} auth login`,
      );
      await terminalWrite(sessionId, btoa(cmd + "\n"));
    } catch {
      /* ignore */
    } finally {
      cliLaunching = false;
    }
  }

  async function handleCliLogout() {
    if (isProviderKind(kind)) return;
    cliLaunching = true;
    try {
      const cmd = await cliGetLogoutCommand(kind);
      const { homeDir } = await import("@tauri-apps/api/path");
      const cwd = await homeDir();
      const sessionId = await openStandaloneTerminal(
        cwd,
        `${kind} auth logout`,
      );
      await terminalWrite(sessionId, btoa(cmd + "\n"));
    } catch {
      /* ignore */
    } finally {
      cliLaunching = false;
    }
  }

  onMount(async () => {
    if (isProviderKind(kind)) {
      await checkStatus();
    } else {
      await refreshCli();
    }
  });

  // Row's connection status + "connected" flag, used for badge styling.
  const connected = $derived(
    isProviderKind(kind) ? providerConnected : cliAuthenticated,
  );

  const statusLabel = $derived.by(() => {
    if (isProviderKind(kind)) {
      return connected
        ? m.settings_integrations_row_connected()
        : m.settings_integrations_row_not_connected();
    }
    if (!cliInstalled) return m.cli_auth_not_installed();
    if (cliAuthenticated) {
      return cliUsername
        ? m.cli_auth_username({ username: cliUsername })
        : m.cli_auth_authenticated();
    }
    return m.cli_auth_not_authenticated();
  });
</script>

<div
  class="connection-row"
  class:connected
  data-testid={`integrations-row-${kind}`}
>
  <span class="name" data-role="name">{displayName}</span>
  <span class="status" data-role="status">{statusLabel}</span>
  <span class="action" data-role="action">
    {#if isProviderKind(kind)}
      {#if providerConnected}
        <button
          type="button"
          class="btn btn-ghost"
          onclick={() => (formOpen = !formOpen)}
        >
          {m.settings_integrations_row_manage()}
        </button>
      {:else}
        <button
          type="button"
          class="btn btn-primary"
          onclick={() => (formOpen = !formOpen)}
        >
          {m.settings_integrations_row_connect()}
        </button>
      {/if}
    {:else if !cliInstalled}
      <!-- Nothing to do when the tool isn't installed, but keep a
           visual slot (disabled button) so every row has exactly one
           action button for layout + test consistency. -->
      <button type="button" class="btn btn-ghost" disabled>
        {m.settings_integrations_row_connect()}
      </button>
    {:else if cliAuthenticated}
      <button
        type="button"
        class="btn btn-ghost"
        disabled={cliLaunching}
        onclick={handleCliLogout}
      >
        {m.cli_auth_logout()}
      </button>
    {:else}
      <button
        type="button"
        class="btn btn-primary"
        disabled={cliLaunching}
        onclick={handleCliAuth}
      >
        {m.cli_auth_authenticate()}
      </button>
    {/if}
  </span>

  {#if formOpen && isProviderKind(kind)}
    <div class="inline-form">
      {#each matchingProviders as provider (provider.instance_url)}
        <div class="existing-provider">
          <span class="existing-label">
            {provider.user.display_name} @{provider.user.username}
            <span class="existing-url">{provider.instance_url}</span>
          </span>
          <button
            type="button"
            class="btn btn-danger"
            onclick={() => handleDisconnect(provider.instance_url)}
          >
            {m.provider_disconnect()}
          </button>
        </div>
      {/each}

      <form class="pat-form" onsubmit={handleConnect}>
        <input
          type="text"
          class="field-input"
          placeholder={kind === "github"
            ? m.provider_instance_placeholder_github()
            : m.provider_instance_placeholder_gitlab()}
          bind:value={instanceUrl}
          disabled={$isConnecting}
        />
        <input
          type="password"
          class="field-input"
          placeholder={kind === "github"
            ? m.provider_token_placeholder_github()
            : m.provider_token_placeholder_gitlab()}
          bind:value={token}
          disabled={$isConnecting}
        />
        <button
          type="submit"
          class="btn btn-primary"
          disabled={$isConnecting || !token.trim()}
        >
          {#if $isConnecting}
            {m.provider_connecting()}
          {:else}
            {m.provider_connect()}
          {/if}
        </button>
        {#if $providerError}
          <div class="error">{$providerError}</div>
        {/if}
      </form>
    </div>
  {/if}
</div>

<style>
  .connection-row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-primary);
    transition: border-color 0.15s;
  }

  .connection-row.connected {
    border-color: var(--accent-green);
    background: var(--overlay-accent-green);
  }

  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .status {
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .connection-row.connected .status {
    color: var(--accent-green);
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .btn {
    font-size: 11px;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition:
      background 0.15s,
      border-color 0.15s,
      opacity 0.15s;
    font-family: inherit;
  }
  .btn:hover:not(:disabled) {
    background: var(--overlay-hover);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    color: #fff;
  }
  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
    background: var(--accent-blue);
  }

  .btn-ghost {
    color: var(--text-secondary);
  }

  .btn-danger {
    color: var(--accent-red, #f85149);
  }

  .inline-form {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 8px;
    border-top: 1px dashed var(--border);
    margin-top: 2px;
  }

  .existing-provider {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 0;
  }

  .existing-label {
    font-size: 12px;
    color: var(--text-primary);
    display: flex;
    gap: 8px;
    align-items: baseline;
    flex-wrap: wrap;
  }

  .existing-url {
    color: var(--text-secondary);
    font-size: 11px;
  }

  .pat-form {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 6px;
    align-items: center;
  }

  .field-input {
    padding: 4px 8px;
    font-size: 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-family: var(--font-mono);
    outline: none;
  }
  .field-input:focus {
    border-color: var(--accent-blue);
  }

  .error {
    grid-column: 1 / -1;
    font-size: 11px;
    color: var(--accent-red, #f85149);
  }
</style>
