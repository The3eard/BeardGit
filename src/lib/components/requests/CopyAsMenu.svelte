<!--
  CopyAsMenu.svelte — "Copy as" dropdown next to the Send button.

  Calls the backend `requests_copy_as` command with a target generator
  (cURL / fetch / HTTPie / wget) and writes the result to the system
  clipboard via the same `navigator.clipboard.writeText` API used
  elsewhere in the app.

  Visuals follow the canonical app dropdown recipe used by
  `AddProjectMenu` (the toolbar "+" menu) so the popover surface, the
  hover tonal, and the row layout are pixel-identical to the rest of
  the app's dropdowns:

  - Trigger is the shared `Button` with `aria-haspopup="menu"` and
    `aria-expanded` for AT users.
  - Popover sits in `var(--bg-secondary)` with a 1 px `--border` rule
    and a 6 px radius.
  - Each menu row is a NerdFont icon + label, 6 px vertical padding,
    and hovers in the standard `color-mix(--text-primary 6%)` tonal.
  - Click-outside dismisses (mirrors `AddProjectMenu`'s mousedown
    handler) and Escape closes for keyboard parity.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Button } from "$lib/components/ui";
  import { activeProject } from "$lib/stores/projects";
  import { addToast } from "$lib/stores/toast";
  import { currentSource, currentEnv } from "./stores";

  /** Project root path for the active tab, or empty when none. */
  $: projectPath = $activeProject?.path ?? "";

  /** Whether the dropdown is currently open. */
  let open = false;

  /** Toggle visibility — also closes on a second click. */
  function toggle() {
    open = !open;
  }

  /**
   * Close on outside click. We register the listener at mount and
   * filter to only fire when the click is outside `.copy-as`. The
   * `mousedown` phase matches `AddProjectMenu`'s pattern so the menu
   * dismisses before the next click delivers focus elsewhere.
   */
  function handleClickOutside(e: MouseEvent) {
    if (!open) return;
    const target = e.target as HTMLElement | null;
    if (!target) return;
    if (target.closest(".copy-as")) return;
    open = false;
  }

  /** Close on Escape so the menu behaves like a native dropdown. */
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open) {
      open = false;
    }
  }

  onMount(() => {
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    document.removeEventListener("mousedown", handleClickOutside);
    document.removeEventListener("keydown", handleKeydown);
  });

  /**
   * Generate a shell/code snippet for the active request and copy it.
   * Silently no-ops when no request source is selected so the button
   * stays clickable without throwing.
   *
   * Backend resolution errors (missing token, parse failures) and
   * clipboard write failures both surface as red toasts so the user
   * gets immediate feedback — the previous code swallowed errors, which
   * is what created the "Copy as does nothing" bug report.
   */
  async function copy(target: "curl" | "fetch" | "httpie" | "wget") {
    open = false;
    if (!$currentSource) return;
    let result: string;
    try {
      result = await invoke<string>("requests_copy_as", {
        args: {
          source_kind: $currentSource.kind,
          source_path: $currentSource.path,
          project_path: projectPath || null,
          env_name: $currentEnv,
          target,
          overrides: {},
        },
      });
    } catch (err) {
      addToast({ message: `Copy as ${target} failed: ${err}`, type: "error" });
      return;
    }
    try {
      await navigator.clipboard.writeText(result);
    } catch (err) {
      addToast({ message: `Clipboard write failed: ${err}`, type: "error" });
      return;
    }
    addToast({
      message: `Copied ${result.length}-char ${target} command`,
      type: "success",
    });
  }

  /**
   * Targets exposed by the menu. The icon glyph is a clipboard
   * NerdFont code point — same family used elsewhere in the app's
   * row icons (e.g. AddProjectMenu's folder glyph).
   */
  const TARGETS: {
    id: "curl" | "fetch" | "httpie" | "wget";
    label: string;
    icon: string;
  }[] = [
    { id: "curl", label: "cURL", icon: "" },
    { id: "fetch", label: "fetch", icon: "" },
    { id: "httpie", label: "HTTPie", icon: "" },
    { id: "wget", label: "wget", icon: "" },
  ];
</script>

<div class="copy-as">
  <Button
    variant="neutral"
    size="md"
    ariaHaspopup="menu"
    ariaExpanded={open}
    onclick={toggle}
  >
    Copy as ▾
  </Button>
  {#if open}
    <div class="copy-as__menu" role="menu">
      {#each TARGETS as t (t.id)}
        <button
          type="button"
          role="menuitem"
          class="copy-as__item"
          on:click={() => copy(t.id)}
        >
          <span class="copy-as__icon nf" aria-hidden="true">{t.icon}</span>
          <span>{t.label}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .copy-as {
    position: relative;
  }

  .copy-as__menu {
    /* Mirrors `AddProjectMenu`'s popover surface so all app dropdowns
       look identical (background, border, radius, padding, offset). */
    position: absolute;
    right: 0;
    top: calc(100% + 2px);
    z-index: 100;
    min-width: 160px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: var(--shadow-overlay);
    padding: 4px 0;
  }

  .copy-as__item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .copy-as__item:hover {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }

  .copy-as__icon {
    font-family: var(--font-icons);
    font-size: 14px;
    width: 16px;
    text-align: center;
    flex-shrink: 0;
    color: var(--accent-primary);
  }
</style>
