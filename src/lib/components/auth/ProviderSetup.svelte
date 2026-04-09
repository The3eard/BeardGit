<script lang="ts">
  import {
    providerStatus,
    isConnecting,
    providerError,
    connect,
    disconnect,
    checkStatus,
  } from "../../stores/provider";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { ProviderKind } from "../../types";
  import * as m from "$lib/paraglide/messages";

  let selectedProvider = $state<ProviderKind>("gitlab");
  let instanceUrl = $state("");
  let token = $state("");
  let showAddForm = $state(false);

  // TODO: Re-enable CLI OAuth login after implementing ghostty terminal integration.
  // The gh/glab CLI login requires an interactive terminal which a GUI app can't
  // provide reliably. For now, only PAT auth is available.
  let authMethod = $state<"pat">("pat");
  let cliAvailable = $state(false);

  $effect(() => {
    checkStatus();
  });

  function defaultUrl(kind: ProviderKind): string {
    return kind === "github" ? "https://api.github.com" : "https://gitlab.com";
  }

  function tokenUrl(kind: ProviderKind, customUrl: string): string {
    if (kind === "github") {
      const base = customUrl.trim()
        ? customUrl.trim().replace(/\/api\/v3\/?$/, "")
        : "https://github.com";
      return `${base}/settings/tokens/new?scopes=repo,read:org,workflow&description=BeardGit`;
    }
    const base = customUrl.trim() || "https://gitlab.com";
    return `${base}/-/user_settings/personal_access_tokens?name=BeardGit&scopes=read_api,read_user`;
  }

  function handleOpenTokenUrl(e: Event) {
    e.preventDefault();
    openUrl(tokenUrl(selectedProvider, instanceUrl));
  }

  async function handleConnect(e: Event) {
    e.preventDefault();
    const url = instanceUrl.trim() || defaultUrl(selectedProvider);
    try {
      await connect(selectedProvider, url, token);
      token = "";
      instanceUrl = "";
      showAddForm = false;
    } catch {
      // error is set in store
    }
  }


  async function handleDisconnect(url: string) {
    await disconnect(url);
  }

  let showForm = $derived($providerStatus.providers.length === 0 || showAddForm);
</script>

<div class="setup-card">
  <h2 class="card-title">{m.provider_title()}</h2>

  {#if !showForm}
    <!-- State B: one or more providers connected -->
    <div class="provider-list">
      {#each $providerStatus.providers as provider, index}
        <div class="provider-entry">
          <div class="provider-icon">
            {#if provider.kind === "github"}
              <svg viewBox="0 0 24 24" width="20" height="20"><path d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.865 8.17 6.839 9.49.5.092.682-.217.682-.482 0-.237-.008-.866-.013-1.7-2.782.604-3.369-1.34-3.369-1.34-.454-1.156-1.11-1.464-1.11-1.464-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.831.092-.646.35-1.086.636-1.336-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0112 6.836a9.59 9.59 0 012.504.337c1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.203 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.578.688.48C19.138 20.167 22 16.418 22 12c0-5.523-4.477-10-10-10z" fill="currentColor"/></svg>
            {:else}
              <svg viewBox="0 0 24 24" width="20" height="20"><path d="M12 21.35L3.26 14.05l1.64-5.05L6.53 3.71h2.95L12 9.18l2.52-5.47h2.95l1.63 5.29 1.64 5.05z" fill="currentColor"/></svg>
            {/if}
          </div>

          <div class="provider-info">
            <div class="provider-name-row">
              <span class="provider-display-name">{provider.user.display_name}</span>
              <span class="provider-username">@{provider.user.username}</span>
              {#if index === $providerStatus.active_index}
                <span class="active-badge">{m.provider_active()}</span>
              {/if}
            </div>
            <div class="provider-url">{provider.instance_url}</div>
            {#if provider.project_name}
              <div class="provider-project">{provider.project_name}</div>
            {/if}
          </div>

          <button
            class="btn-disconnect"
            onclick={() => handleDisconnect(provider.instance_url)}
          >
            {m.provider_disconnect()}
          </button>
        </div>
      {/each}
    </div>

    <button class="btn-add-provider" onclick={() => { showAddForm = true; }}>
      {m.provider_add()}
    </button>
  {:else}
    <!-- State A: no providers OR showAddForm is true -->
    {#if $providerStatus.providers.length === 0}
      <p class="card-description">
        {m.provider_description()}
      </p>
    {/if}

    <div class="setup-form">
      <div class="provider-selector">
        <button
          type="button"
          class="provider-btn"
          class:active={selectedProvider === "gitlab"}
          onclick={() => { selectedProvider = "gitlab"; instanceUrl = ""; }}
        >{m.provider_gitlab()}</button>
        <button
          type="button"
          class="provider-btn"
          class:active={selectedProvider === "github"}
          onclick={() => { selectedProvider = "github"; instanceUrl = ""; }}
        >{m.provider_github()}</button>
      </div>

      <!-- TODO: Re-enable OAuth toggle after ghostty terminal integration -->
      <form class="pat-form" onsubmit={handleConnect}>
          <label class="field">
            <span class="field-label">
              {m.provider_instance_url()}
              <span class="field-hint" title={selectedProvider === 'github' ? m.provider_instance_hint_github() : m.provider_instance_hint_gitlab()}>?</span>
            </span>
            <input
              type="text"
              class="field-input"
              bind:value={instanceUrl}
              placeholder={selectedProvider === 'github' ? m.provider_instance_placeholder_github() : m.provider_instance_placeholder_gitlab()}
              disabled={$isConnecting}
            />
          </label>

          <label class="field">
            <span class="field-label">
              {m.provider_token_label()}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <span class="token-link" onclick={handleOpenTokenUrl} title={selectedProvider === 'github' ? m.provider_token_get_title_github() : m.provider_token_get_title_gitlab()}>{m.provider_token_get_link()}</span>
            </span>
            <input
              type="password"
              class="field-input"
              bind:value={token}
              placeholder={selectedProvider === "github" ? m.provider_token_placeholder_github() : m.provider_token_placeholder_gitlab()}
              disabled={$isConnecting}
            />
          </label>

          {#if $providerError}
            <div class="error-message">{$providerError}</div>
          {/if}

          <div class="form-actions">
            <button
              type="submit"
              class="btn btn-connect"
              disabled={$isConnecting || !token.trim()}
            >
              {#if $isConnecting}
                {m.provider_connecting()}
              {:else}
                {m.provider_connect()}
              {/if}
            </button>

            {#if showAddForm && $providerStatus.providers.length > 0}
              <button
                type="button"
                class="btn btn-cancel"
                onclick={() => { showAddForm = false; token = ""; instanceUrl = ""; }}
              >
                {m.provider_cancel()}
              </button>
            {/if}
          </div>
        </form>
    </div>
  {/if}
</div>

<style>
  .setup-card { max-width: 480px; margin: 48px auto; padding: 32px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; }
  .card-title { font-size: 18px; font-weight: 600; color: var(--text-primary); margin-bottom: 8px; }
  .card-description { font-size: 13px; color: var(--text-secondary); line-height: 1.5; margin-bottom: 24px; }
  .setup-form { display: flex; flex-direction: column; gap: 16px; }

  .provider-selector { display: flex; gap: 8px; }
  .provider-btn { flex: 1; padding: 8px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; color: var(--text-secondary); font-size: 13px; font-weight: 500; cursor: pointer; transition: all 0.15s; }
  .provider-btn:hover { border-color: var(--text-secondary); }
  .provider-btn.active { border-color: var(--accent-blue); color: var(--accent-blue); background: rgba(88,166,255,0.08); }

  .field { display: flex; flex-direction: column; gap: 6px; }
  .field-label { font-size: 12px; font-weight: 600; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 4px; }
  .field-hint { display: inline-flex; align-items: center; justify-content: center; width: 14px; height: 14px; border-radius: 50%; border: 1px solid var(--text-secondary); font-size: 10px; font-weight: 700; color: var(--text-secondary); cursor: help; text-transform: none; letter-spacing: 0; position: relative; }
  .token-link { margin-left: auto; font-size: 11px; font-weight: 500; color: var(--accent-blue); text-transform: none; letter-spacing: 0; cursor: pointer; text-decoration: underline; text-decoration-style: dotted; text-underline-offset: 2px; }
  .token-link:hover { opacity: 0.8; }
  .field-input { padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none; transition: border-color 0.15s; }
  .field-input:focus { border-color: var(--accent-blue); }
  .field-input:disabled { opacity: 0.5; }

  .form-actions { display: flex; gap: 8px; margin-top: 4px; }
  .btn { padding: 8px 20px; border: none; border-radius: 6px; font-size: 13px; font-weight: 500; cursor: pointer; transition: opacity 0.15s; }
  .btn:hover:not(:disabled) { opacity: 0.9; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-connect { background: var(--accent-blue); color: var(--text-primary); flex: 1; }
  .btn-cancel { background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-secondary); }

  .error-message { font-size: 12px; color: var(--accent-red); padding: 8px 12px; background: rgba(248, 81, 73, 0.1); border-radius: 6px; word-break: break-word; }

  /* Provider list */
  .provider-list { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; }
  .provider-entry { display: flex; align-items: center; gap: 12px; padding: 12px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; }
  .provider-icon { color: var(--text-secondary); flex-shrink: 0; display: flex; align-items: center; }
  .provider-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .provider-name-row { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .provider-display-name { font-size: 13px; font-weight: 600; color: var(--text-primary); }
  .provider-username { font-size: 12px; color: var(--text-secondary); }
  .active-badge { font-size: 10px; font-weight: 600; padding: 1px 6px; background: rgba(63,185,80,0.15); color: var(--accent-green); border-radius: 8px; white-space: nowrap; }
  .provider-url { font-size: 11px; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .provider-project { font-size: 11px; color: var(--accent-blue); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .btn-disconnect { flex-shrink: 0; padding: 5px 12px; background: rgba(248,81,73,0.1); border: 1px solid rgba(248,81,73,0.2); border-radius: 5px; color: var(--accent-red); font-size: 12px; font-weight: 500; cursor: pointer; transition: opacity 0.15s; white-space: nowrap; }
  .btn-disconnect:hover { opacity: 0.85; }

  .btn-add-provider { width: 100%; padding: 8px; background: none; border: 1px dashed var(--border); border-radius: 6px; color: var(--accent-blue); font-size: 13px; font-weight: 500; cursor: pointer; transition: background 0.15s; }
  .btn-add-provider:hover { background: rgba(88,166,255,0.06); }

  /* TODO: Re-enable auth-method-toggle, oauth-desc after ghostty integration */
  .pat-form { display: flex; flex-direction: column; gap: 16px; }
</style>
