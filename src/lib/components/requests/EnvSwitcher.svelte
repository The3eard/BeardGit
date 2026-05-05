<!--
  Environment switcher dropdown for the Requests panel.

  Lists named environments under the active project's
  `requests/_env/` folder. Picking one updates `currentEnv` (used by
  the resolver during runs) and persists the selection on the backend
  via `requests_set_env` so it survives reloads.

  Invariant: there is always an active env. The backend auto-creates
  `_env/default.json` whenever the requests folder exists, so on mount
  (and after every env-list reload, including those triggered by a
  `treeReloadSignal` bump from a seed) we ensure `currentEnv` points at
  a name that's still in the list — defaulting to `"default"` whenever
  it's available, otherwise the first env. There is no "no env" entry
  in the dropdown.

  Wraps the native `<select>` in the shared `Field` primitive so the
  label / description / error layout matches every other settings-style
  control in the app. The select itself reuses the `bg-select` styling
  recipe used in `LookAndFeelSection` and `FeaturesSection`.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { get } from "svelte/store";
  import { Button, Field } from "$lib/components/ui";
  import { currentEnv, treeReloadSignal } from "./stores";
  import { activeProject } from "$lib/stores/projects";
  import EnvManagerDialog from "./EnvManagerDialog.svelte";

  type Summary = { name: string; vars_count: number; secrets: string[] };

  let envs: Summary[] = [];
  let showManager = false;

  $: projectPath = $activeProject?.path ?? "";

  /**
   * Fetch the list of available envs for the active project, then make
   * sure `currentEnv` points at a name that's actually in the list.
   * Prefers `"default"` (which the backend auto-creates) so the panel
   * always opens on a valid env after a seed.
   */
  async function reload() {
    if (!projectPath) {
      envs = [];
      return;
    }
    envs = await invoke<Summary[]>("requests_get_envs", { projectPath });
    ensureActiveEnv();
  }

  /**
   * Coerce `currentEnv` to a name that exists in the freshly-loaded
   * `envs` list. If it's already valid we leave it alone; otherwise we
   * fall back to `"default"`, then to the first env, then to `null`
   * (only reachable when the project has no envs yet, which the
   * backend prevents whenever the requests folder exists).
   */
  function ensureActiveEnv() {
    if (envs.length === 0) return;
    const current = get(currentEnv);
    const valid = current !== null && envs.some((e) => e.name === current);
    if (valid) return;
    const fallback =
      envs.find((e) => e.name === "default")?.name ?? envs[0].name;
    void pick(fallback);
  }

  /**
   * Set both the in-memory `currentEnv` store and persist the choice
   * to disk so the next session restores it. The dropdown never emits
   * an empty string — every option is a real env name.
   */
  async function pick(name: string) {
    currentEnv.set(name);
    await invoke("requests_set_env", { projectPath, envName: name });
  }

  /**
   * Format the trailing `(N vars, M secrets)` summary so users see at a
   * glance whether an env actually has content. Singular/plural kept
   * simple (S-suffix only) — the dropdown is a tight visual element.
   */
  function summary(e: Summary): string {
    const v = e.vars_count;
    const s = e.secrets.length;
    return `(${v} var${v === 1 ? "" : "s"}, ${s} secret${s === 1 ? "" : "s"})`;
  }

  // Reload whenever the project changes or a seed/create/delete bumps
  // `treeReloadSignal` — the seeded `default.json` only appears in
  // `requests_get_envs` after the seed completes.
  $: projectPath, $treeReloadSignal, reload();
  onMount(reload);
</script>

<div class="env-switcher">
  <Field label="Environment" for="requests-env-select">
    <div class="env-switcher__row">
      <select
        id="requests-env-select"
        class="bg-select"
        on:change={(e) =>
          pick((e.currentTarget as HTMLSelectElement).value)}
      >
        {#each envs as e}
          <option value={e.name} selected={$currentEnv === e.name}
            >{e.name} {summary(e)}</option
          >
        {/each}
      </select>
      <Button
        variant="neutral"
        size="xs"
        disabled={!projectPath}
        onclick={() => (showManager = true)}
      >
        Manage
      </Button>
    </div>
  </Field>
</div>

<EnvManagerDialog
  {projectPath}
  bind:open={showManager}
  onChanged={reload}
/>

<style>
  .env-switcher {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .env-switcher__row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .env-switcher__row .bg-select {
    flex: 1;
  }

  .bg-select {
    width: 100%;
    height: 30px;
    line-height: 28px;
    padding: 0 26px 0 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    cursor: pointer;
    font-family: var(--font-mono);
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'><path fill='none' stroke='%23888' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round' d='M1 1l4 4 4-4'/></svg>");
    background-repeat: no-repeat;
    background-position: right 10px center;
    box-sizing: border-box;
  }

  .bg-select:focus {
    border-color: var(--accent-blue);
  }
</style>
