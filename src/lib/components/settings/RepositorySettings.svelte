<script lang="ts">
  import { getRemotes, renameRemote, removeRemote } from "$lib/api/tauri";
  import ConfirmDialog from "$lib/components/common/ConfirmDialog.svelte";
  import * as m from "$lib/paraglide/messages";
  import type { RemoteInfo } from "$lib/types";

  let remotes = $state<RemoteInfo[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Rename state — which remote is being renamed and the input value
  let renamingName = $state<string | null>(null);
  let renameValue = $state("");
  let renameError = $state<string | null>(null);
  let renameInputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (renamingName !== null && renameInputEl) {
      renameInputEl.focus();
    }
  });

  // Remove confirm state
  let removingName = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      remotes = await getRemotes();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    load();
  });

  function startRename(name: string) {
    renamingName = name;
    renameValue = name;
    renameError = null;
  }

  function cancelRename() {
    renamingName = null;
    renameValue = "";
    renameError = null;
  }

  async function confirmRename() {
    if (!renamingName) return;
    const trimmed = renameValue.trim();
    if (!trimmed || trimmed === renamingName) {
      cancelRename();
      return;
    }
    try {
      await renameRemote(renamingName, trimmed);
      renamingName = null;
      renameValue = "";
      renameError = null;
      await load();
    } catch (e) {
      renameError = String(e);
    }
  }

  function handleRenameKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      confirmRename();
    } else if (e.key === "Escape") {
      cancelRename();
    }
  }

  function startRemove(name: string) {
    removingName = name;
  }

  async function confirmRemove() {
    if (!removingName) return;
    try {
      await removeRemote(removingName);
      removingName = null;
      await load();
    } catch (e) {
      error = String(e);
      removingName = null;
    }
  }

  function cancelRemove() {
    removingName = null;
  }
</script>

<div class="repository-settings">
  <h2 class="section-title">{m.settings_remotes()}</h2>

  {#if loading}
    <p class="state-message">Loading...</p>
  {:else if error}
    <p class="state-message error">{error}</p>
  {:else if remotes.length === 0}
    <p class="state-message">{m.settings_remote_no_repo()}</p>
  {:else}
    <ul class="remotes-list">
      {#each remotes as remote (remote.name)}
        <li class="remote-item">
          {#if renamingName === remote.name}
            <div class="rename-row">
              <input
                class="rename-input"
                type="text"
                bind:value={renameValue}
                bind:this={renameInputEl}
                onkeydown={handleRenameKeydown}
                placeholder={m.settings_remote_rename_placeholder()}
              />
              {#if renameError}
                <span class="inline-error">{renameError}</span>
              {/if}
              <button class="btn btn-primary" onclick={confirmRename}>{m.settings_remote_rename()}</button>
              <button class="btn btn-ghost" onclick={cancelRename}>{m.confirm_cancel()}</button>
            </div>
          {:else}
            <div class="remote-info">
              <span class="remote-name">{remote.name}</span>
              {#if remote.url}
                <span class="remote-url">{remote.url}</span>
              {/if}
            </div>
            <div class="remote-actions">
              <button class="btn btn-ghost" onclick={() => startRename(remote.name)}>
                {m.settings_remote_rename()}
              </button>
              <button class="btn btn-danger-ghost" onclick={() => startRemove(remote.name)}>
                {m.settings_remote_remove()}
              </button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if removingName !== null}
  <ConfirmDialog
    title={m.settings_remote_rename_title()}
    message={m.settings_remote_remove_confirm({ name: removingName })}
    confirmLabel={m.settings_remote_remove()}
    destructive={true}
    onConfirm={confirmRemove}
    onCancel={cancelRemove}
  />
{/if}

<style>
  .repository-settings {
    padding: 24px 28px;
    max-width: 600px;
  }

  .section-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 16px;
  }

  .state-message {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
  }

  .state-message.error {
    color: #f85149;
  }

  .remotes-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .remote-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    min-height: 48px;
  }

  .remote-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    flex: 1;
  }

  .remote-name {
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: #3fb950;
  }

  .remote-url {
    font-size: 12px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .remote-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .rename-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .rename-input {
    flex: 1;
    padding: 5px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--accent-blue);
    border-radius: 5px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
    outline: none;
  }

  .rename-input:focus {
    box-shadow: 0 0 0 2px rgba(88, 166, 255, 0.2);
  }

  .inline-error {
    font-size: 11px;
    color: #f85149;
    flex-shrink: 0;
  }

  .btn {
    padding: 5px 12px;
    border-radius: 5px;
    font-size: 12px;
    cursor: pointer;
    border: 1px solid var(--border);
    transition: background 0.15s, opacity 0.15s;
    white-space: nowrap;
  }

  .btn-primary {
    background: var(--accent-blue);
    color: #fff;
    border-color: var(--accent-blue);
  }

  .btn-primary:hover {
    opacity: 0.85;
  }

  .btn-ghost {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .btn-ghost:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .btn-danger-ghost {
    background: transparent;
    color: #f85149;
    border-color: transparent;
  }

  .btn-danger-ghost:hover {
    background: rgba(248, 81, 73, 0.1);
    border-color: rgba(248, 81, 73, 0.3);
  }
</style>
