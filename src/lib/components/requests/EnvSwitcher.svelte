<!--
  Environment switcher for the Requests panel.

  Lists named environments under the active project's
  `requests/_env/` folder. Picking one updates `currentEnv` (used by
  the resolver during runs) and persists the selection on the backend
  via `requests_set_env` so it survives reloads.

  Visually mirrors:
    - The BRANCHES panel header recipe (`src/lib/styles/list.css`'s
      `.list-header` + `.list-title`): no explicit background, 10px
      padding, 12px uppercase muted label flush left.
    - The toolbar AI dropdown (TabBar `.action-menu` + `.action-menu-item`):
      Button trigger with chevron + active state, popover anchored
      below, click-outside / Escape close, scoped `--bg-secondary`
      surface with `--text-primary 6%` tonal hover.

  Invariant: there is always an active env. The backend auto-creates
  `_env/default.json` whenever the panel is accessed, so on mount and
  after every env-list reload (including those triggered by a
  `treeReloadSignal` bump from a seed or env edit) we ensure
  `currentEnv` points at a name that's still in the list — defaulting
  to `"default"` when available, otherwise the first env. The dropdown
  has no "no env" option.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { get } from "svelte/store";
  import { Button, IconButton } from "$lib/components/ui";
  import { currentEnv, treeReloadSignal } from "./stores";
  import { activeProject } from "$lib/stores/projects";
  import EnvManagerDialog from "./EnvManagerDialog.svelte";

  type Summary = { name: string; vars_count: number; secrets: string[] };

  let envs: Summary[] = [];
  let showManager = false;
  let menuOpen = false;
  let menuRef: HTMLDivElement | null = null;

  $: projectPath = $activeProject?.path ?? "";

  /**
   * Fetch the list of available envs for the active project, then make
   * sure `currentEnv` points at a name that's actually in the list.
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
   * envs list. Prefers `default` (auto-created by the backend),
   * otherwise the first env in the list.
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
   * to disk so the next session restores it.
   */
  async function pick(name: string) {
    currentEnv.set(name);
    await invoke("requests_set_env", { projectPath, envName: name });
  }

  function toggleMenu() {
    menuOpen = !menuOpen;
  }

  function closeMenu() {
    menuOpen = false;
  }

  function handleSelect(name: string) {
    closeMenu();
    void pick(name);
  }

  function handleClickOutside(e: MouseEvent) {
    if (!menuOpen) return;
    if (menuRef && !menuRef.contains(e.target as Node)) {
      closeMenu();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && menuOpen) {
      e.preventDefault();
      closeMenu();
    }
  }

  // Reload whenever the project changes or a seed/create/delete bumps
  // `treeReloadSignal` — the seeded `default.json` only appears in
  // `requests_get_envs` after the seed completes.
  $: projectPath, $treeReloadSignal, reload();

  onMount(() => {
    void reload();
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeydown);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<div class="env-switcher">
  <span class="env-switcher__label">ENV</span>
  <div class="env-dropdown" bind:this={menuRef}>
    <Button
      variant="neutral"
      size="sm"
      ariaHaspopup="menu"
      ariaExpanded={menuOpen}
      active={menuOpen}
      disabled={!projectPath || envs.length === 0}
      onclick={toggleMenu}
      testid="requests-env-trigger"
    >
      <span class="env-name">{$currentEnv ?? "—"}</span>
      <span class="chevron nf" class:open={menuOpen} aria-hidden="true">{""}</span>
    </Button>
    {#if menuOpen}
      <div class="action-menu" role="menu" data-testid="requests-env-menu">
        {#each envs as e (e.name)}
          <button
            type="button"
            class="action-menu-item"
            role="menuitem"
            class:selected={$currentEnv === e.name}
            onclick={() => handleSelect(e.name)}
          >
            <span class="menu-item-label">{e.name}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
  <IconButton
    icon={""}
    description="Manage environments"
    disabled={!projectPath}
    onclick={() => (showManager = true)}
  />
</div>

<EnvManagerDialog
  {projectPath}
  bind:open={showManager}
  onChanged={reload}
/>

<style>
  /* Mirrors the BRANCHES panel header recipe (`.list-header` +
     `.list-title` from `src/lib/styles/list.css`): no explicit
     background so it inherits from the panel surface, 10px/12px
     padding, 12px uppercase muted label on the left. */
  .env-switcher {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .env-switcher__label {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  /* Trigger + popover share the same vocabulary as the toolbar AI
     dropdown — see `.ai-dropdown` / `.action-menu` in TabBar.svelte. */
  .env-dropdown {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .env-dropdown :global(.bg-btn) {
    width: 100%;
    font-family: var(--font-mono);
  }

  /* The Button primitive wraps slot content in `.bg-btn__label` with
     `display: inline`, so a name + chevron mix lays out by baseline —
     and the small Nerd Font glyph drifts visually relative to the
     mono name. Reach into the label and switch it to flex so the
     children align by center and the chevron parks on the right. */
  .env-dropdown :global(.bg-btn__label) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    min-width: 0;
  }

  .env-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .chevron {
    font-size: 9px;
    color: var(--text-secondary);
    transition: transform 0.15s;
    margin-left: 6px;
    flex-shrink: 0;
    line-height: 1;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  /* Same recipe as TabBar's `.action-menu`. */
  .action-menu {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 100;
    min-width: 160px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    /* beardgit:allow-hex: shadow neutral always-dark */
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    padding: 4px 0;
    margin-top: 2px;
  }

  .action-menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
    cursor: pointer;
    white-space: nowrap;
  }

  .action-menu-item:hover,
  .action-menu-item:focus-visible {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    outline: none;
  }

  .action-menu-item.selected {
    color: var(--accent-blue);
  }
</style>
