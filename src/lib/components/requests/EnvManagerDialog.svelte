<!--
  EnvManagerDialog.svelte — modal CRUD for project environment files.

  Wraps the Dialog primitive around a two-column form: a leftside list
  of envs (with a "+ New env" entry) and a rightside editor for the
  selected env's vars + secrets. Save writes via `requests_save_env`,
  Delete removes the file via `requests_delete_env`.

  Vars are plaintext key/value pairs. Secrets store their _names_ in
  the env file; the actual values live in the encrypted credential
  store and are written through `SecretPrompt` (which calls
  `requests_set_secret`). Removing a secret name from this dialog does
  NOT delete the previously-stored value from the credential store —
  that's an acceptable v1 limitation (the orphaned key is harmless and
  costs no resolution time once nothing references it).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    requestsGetEnvs,
    requestsLoadEnv,
    requestsSaveEnv,
    requestsDeleteEnv,
  } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import type { RequestEnvFile } from "$lib/types/requests";
  import { Button, Dialog, IconButton } from "$lib/components/ui";
  import ConfirmDialog from "$lib/components/common/ConfirmDialog.svelte";
  import SecretPrompt from "./SecretPrompt.svelte";

  /** EnvFile shape mirroring `requests_runner::env::EnvFile`. */
  type EnvFile = {
    $schema?: string;
    vars: Record<string, string>;
    secrets: string[];
  };

  interface Props {
    /** Project root for env operations. Required. */
    projectPath: string;
    /** Two-way bindable. */
    open: boolean;
    /**
     * Fired after any save / delete so callers can re-fetch the env
     * list and refresh the switcher dropdown.
     */
    onChanged?: () => void;
  }

  let { projectPath, open = $bindable(false), onChanged }: Props = $props();

  /** Names of all env files in the project. */
  let names = $state<string[]>([]);
  /** Currently selected env name (null = "+ New env" placeholder). */
  let selected = $state<string | null>(null);
  /** Working copy of the selected env, edited locally before Save. */
  let working = $state<EnvFile>({ vars: {}, secrets: [] });
  /** Vars rendered as an array so add/remove rows are stable. */
  let varRows = $state<Array<{ key: string; value: string }>>([]);
  /** Name input shown when `selected === null` (creating a new env). */
  let newName = $state("");
  /** Save / delete in flight. */
  let busy = $state(false);
  /** Surface backend errors next to the action buttons. */
  let error = $state<string | null>(null);
  /** Drives the SecretPrompt overlay used to set a secret value. */
  let promptingSecret = $state<string | null>(null);
  /**
   * Name of the env queued for deletion. While non-null the
   * `ConfirmDialog` overlay is open and the actual `requests_delete_env`
   * IPC only fires after the user clicks Delete in the confirm.
   */
  let pendingDeleteName = $state<string | null>(null);

  /** Fetch the full list of envs from the backend. */
  async function reloadList() {
    if (!projectPath) {
      names = [];
      return;
    }
    const summaries = await requestsGetEnvs(projectPath);
    names = summaries.map((s) => s.name);
  }

  /** Load `name` into the editor; null resets to a blank "+ New env" form. */
  async function pick(name: string | null) {
    error = null;
    selected = name;
    if (name === null) {
      working = { vars: {}, secrets: [] };
      varRows = [];
      newName = "";
      return;
    }
    working = await requestsLoadEnv(projectPath, name);
    varRows = Object.entries(working.vars ?? {}).map(([key, value]) => ({
      key,
      value,
    }));
    if (!Array.isArray(working.secrets)) working.secrets = [];
  }

  function addVar() {
    varRows = [...varRows, { key: "", value: "" }];
  }

  function removeVar(i: number) {
    varRows = varRows.filter((_, j) => j !== i);
  }

  function addSecret() {
    working.secrets = [...working.secrets, ""];
  }

  function removeSecret(i: number) {
    working.secrets = working.secrets.filter((_, j) => j !== i);
  }

  /** Persist the working copy to disk under the chosen name. */
  async function save() {
    error = null;
    const target = selected ?? newName.trim();
    if (!target) {
      error = "Env name is required.";
      return;
    }
    // Reject characters that would break the filename.
    if (!/^[A-Za-z0-9_.-]+$/.test(target)) {
      error = "Env names may only contain letters, digits, _, ., or -.";
      return;
    }
    // Roll varRows back into the map, dropping rows with empty keys.
    const vars: Record<string, string> = {};
    for (const r of varRows) {
      if (r.key.trim()) vars[r.key.trim()] = r.value;
    }
    const env: RequestEnvFile = {
      $schema: working.$schema ?? "beardgit-env/v1",
      vars,
      // Drop blank secret names but keep duplicates' first occurrence.
      secrets: Array.from(
        new Set(working.secrets.map((s) => s.trim()).filter(Boolean)),
      ),
    };
    busy = true;
    try {
      await runMutation({
        kind: "requests_save_env",
        invoke: () => requestsSaveEnv(projectPath, target, env),
        successToast: () => `Saved env "${target}"`,
        failureToastPrefix: "Save env failed",
      });
      await reloadList();
      selected = target;
      onChanged?.();
      // Close on success so the user gets a clear "done" signal. On
      // failure runMutation surfaces the toast and we keep the dialog
      // open so the partial edit isn't lost.
      close();
    } catch {
      // runMutation already surfaced the failure toast.
    } finally {
      busy = false;
    }
  }

  /**
   * Stage the currently-selected env for deletion. The actual IPC
   * fires from `confirmDelete()` once the user confirms in the
   * `ConfirmDialog` overlay below.
   */
  function deleteEnv() {
    if (!selected) return;
    pendingDeleteName = selected;
  }

  /**
   * Resolve the staged delete: call `requests_delete_env`, refresh
   * the list, and reset the editor to "+ New env".
   */
  async function confirmDelete() {
    const target = pendingDeleteName;
    pendingDeleteName = null;
    if (!target) return;
    busy = true;
    error = null;
    try {
      await runMutation({
        kind: "requests_delete_env",
        invoke: () => requestsDeleteEnv(projectPath, target),
        successToast: () => `Deleted env "${target}"`,
        failureToastPrefix: "Delete env failed",
      });
      await reloadList();
      await pick(null);
      onChanged?.();
    } catch {
      // runMutation already surfaced the failure toast.
    } finally {
      busy = false;
    }
  }

  function close() {
    open = false;
  }

  /**
   * When the dialog opens, reload the list and default to the first
   * env (or the "+ New env" placeholder when there are none).
   */
  $effect(() => {
    if (open) {
      void reloadList().then(() => {
        if (selected === null && names.length > 0) {
          void pick(names[0]);
        }
      });
    }
  });

  onMount(() => {
    void reloadList();
  });
</script>

<Dialog bind:open title="Manage environments" size="lg" onClose={close}>
  <div class="env-manager">
    <aside class="env-manager__list">
      <ul>
        {#each names as n (n)}
          <li>
            <button
              type="button"
              class="env-manager__item"
              class:env-manager__item--selected={selected === n}
              onclick={() => pick(n)}
            >
              {n}
            </button>
          </li>
        {/each}
        <li>
          <button
            type="button"
            class="env-manager__item env-manager__item--new"
            class:env-manager__item--selected={selected === null}
            onclick={() => pick(null)}
          >
            + New env
          </button>
        </li>
      </ul>
    </aside>

    <section class="env-manager__editor">
      {#if selected === null}
        <label class="env-manager__label" for="env-new-name">Name</label>
        <input
          id="env-new-name"
          class="bg-input"
          bind:value={newName}
          placeholder="dev"
        />
      {:else}
        <div class="env-manager__heading">
          <span>Editing</span>
          <code>{selected}</code>
        </div>
      {/if}

      <h3 class="env-manager__section-title">Variables</h3>
      <p class="env-manager__hint">
        Plaintext key/value pairs available as <code>{"{{name}}"}</code> in
        URLs, headers, and bodies.
      </p>
      <div class="kv">
        {#each varRows as row, i (i)}
          <div class="kv__row">
            <input
              class="bg-input"
              bind:value={row.key}
              placeholder="name"
            />
            <input
              class="bg-input"
              bind:value={row.value}
              placeholder="value"
            />
            <IconButton
              icon={""}
              description="Remove variable"
              tone="danger"
              size="sm"
              onclick={() => removeVar(i)}
            />
          </div>
        {/each}
        <div class="kv__add">
          <Button variant="neutral" size="sm" icon={""} onclick={addVar}>
            Add variable
          </Button>
        </div>
      </div>

      <h3 class="env-manager__section-title">Secrets</h3>
      <p class="env-manager__hint">
        Secret <em>names</em> live in the env file; values are stored
        encrypted in your OS keychain. Removing a name here does not
        delete the previously-stored value.
      </p>
      <div class="kv">
        {#each working.secrets as _name, i (i)}
          <div class="kv__row kv__row--secret">
            <input
              class="bg-input"
              bind:value={working.secrets[i]}
              placeholder="TOKEN"
            />
            <Button
              variant="neutral"
              size="xs"
              disabled={!working.secrets[i] || selected === null}
              onclick={() => (promptingSecret = working.secrets[i])}
            >
              Set value
            </Button>
            <IconButton
              icon={""}
              description="Remove secret"
              tone="danger"
              size="sm"
              onclick={() => removeSecret(i)}
            />
          </div>
        {/each}
        <div class="kv__add">
          <!--
            Secret rows use a key glyph instead of the generic plus to
            differentiate the two affordances at a glance — they look
            identical otherwise (same Button variant + size).
          -->
          <Button
            variant="neutral"
            size="sm"
            icon={""}
            onclick={addSecret}
          >
            Add secret
          </Button>
        </div>
      </div>

      {#if error}
        <p class="env-manager__error" role="alert">{error}</p>
      {/if}
    </section>
  </div>

  {#snippet footer()}
    {#if selected !== null}
      <Button
        variant="danger"
        size="sm"
        onclick={deleteEnv}
        disabled={busy}
      >
        Delete
      </Button>
    {/if}
    <span style="flex: 1"></span>
    <Button variant="neutral" onclick={close}>Cancel</Button>
    <Button variant="primary" loading={busy} onclick={save}>Save</Button>
  {/snippet}
</Dialog>

{#if promptingSecret && selected}
  <SecretPrompt
    envName={selected}
    secretName={promptingSecret}
    on:saved={() => (promptingSecret = null)}
    on:cancel={() => (promptingSecret = null)}
  />
{/if}

{#if pendingDeleteName !== null}
  <ConfirmDialog
    title="Delete environment?"
    detail={pendingDeleteName}
    message={`Permanently delete env file "${pendingDeleteName}.json". This cannot be undone.`}
    confirmLabel="Delete"
    destructive={true}
    onConfirm={confirmDelete}
    onCancel={() => (pendingDeleteName = null)}
  />
{/if}

<style>
  .env-manager {
    display: grid;
    grid-template-columns: 180px 1fr;
    gap: 16px;
    min-height: 360px;
  }

  .env-manager__list {
    border-right: 1px solid var(--border);
    padding-right: 12px;
  }

  .env-manager__list ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .env-manager__item {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 6px 10px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    cursor: pointer;
  }

  .env-manager__item:hover {
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
  }

  .env-manager__item--selected {
    background: var(--overlay-accent-blue);
    color: var(--accent-primary);
  }

  .env-manager__item--new {
    color: var(--text-secondary);
    font-style: italic;
  }

  .env-manager__editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .env-manager__label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .env-manager__heading {
    display: flex;
    align-items: baseline;
    gap: 6px;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .env-manager__heading code {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .env-manager__section-title {
    margin: 12px 0 0 0;
    font-size: var(--font-size-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .env-manager__hint {
    margin: 0 0 4px 0;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .env-manager__hint code {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    padding: 0 3px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 3px;
  }

  .env-manager__error {
    margin: 8px 0 0 0;
    color: var(--accent-red);
    font-size: var(--font-size-sm);
  }

  .kv {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .kv__row {
    display: grid;
    grid-template-columns: 1fr 2fr 28px;
    gap: 6px;
    align-items: center;
  }

  .kv__row--secret {
    grid-template-columns: 1fr auto 28px;
  }

  .kv__add {
    margin-top: 4px;
  }

  .bg-input {
    width: 100%;
    height: 30px;
    line-height: 28px;
    padding: 0 10px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    outline: none;
    box-sizing: border-box;
  }

  .bg-input:focus {
    border-color: var(--accent-primary);
  }
</style>
