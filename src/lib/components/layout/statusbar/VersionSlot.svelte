<!--
  VersionSlot — right-aligned `vX.Y.Z` label with an "update available"
  dot overlay when the auto-update store has surfaced a new version.

  The version string comes from `import.meta.env.VITE_APP_VERSION` which
  Vite inlines from `package.json` at build time (see `vite.config.js`).
  Under vitest the env var may be undefined, so we fall back to a
  sentinel `v0.0.0` rather than rendering nothing — the slot should
  always be visible so the user knows what build they're on.

  Clicking the slot invokes the parent's `onNavigate("updates")`. Until
  the Settings IA overhaul (MT-5) lands with actual sub-section deep-
  linking, the parent just routes to the Settings root.
-->
<script lang="ts">
  import { autoUpdateState } from "$lib/stores/autoUpdate";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Navigate to a Settings sub-section (parent wires real routing). */
    onNavigate: (section: string) => void;
  }

  const { onNavigate }: Props = $props();

  let versionString = $derived(
    `v${(import.meta.env.VITE_APP_VERSION as string | undefined) ?? "0.0.0"}`,
  );

  let updateAvailable = $derived($autoUpdateState.status === "available");
</script>

<button
  class="version-slot"
  class:update-available={updateAvailable}
  title={updateAvailable
    ? m.statusbar_update_available_tooltip()
    : m.statusbar_version_tooltip()}
  data-testid="statusbar-version-slot"
  data-update-available={updateAvailable ? "true" : "false"}
  onclick={() => onNavigate("updates")}
  type="button"
>
  <span class="version">{versionString}</span>
  {#if updateAvailable}
    <span class="update-dot" data-testid="statusbar-version-update-dot">•</span>
  {/if}
</button>

<style>
  .version-slot {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 100%;
    padding: 0 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    user-select: none;
    transition: color 0.15s;
    font-variant-numeric: tabular-nums;
  }

  .version-slot:hover {
    color: var(--text-primary);
  }

  .version-slot.update-available {
    color: var(--accent-blue);
  }

  .update-dot {
    color: var(--accent-blue);
    font-size: 16px;
    line-height: 0;
    margin-left: 1px;
  }
</style>
