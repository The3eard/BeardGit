<!--
  ForgeSlot — renders a single pill for the active project's forge
  provider, or nothing when the project has no resolvable provider.

  Single-pill output (never both GitHub and GitLab simultaneously) comes
  from the `projectProvider` derived store, which consults (in order):

    1. `repoConfig.provider` explicit override.
    2. `activeProject.remotes[origin]` URL heuristic.
    3. Otherwise `null` (component collapses).

  Clicking the pill invokes the parent-provided
  `onNavigate("integrations")`, which the StatusBar maps to the
  Connection Settings sub-section.
-->
<script lang="ts">
  import { projectProvider } from "$lib/stores/provider";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Navigate to a specific Settings sub-section (or Settings root). */
    onNavigate: (section: string) => void;
  }

  const { onNavigate }: Props = $props();
</script>

{#if $projectProvider}
  {@const p = $projectProvider}
  <div class="forge-slot" data-testid="statusbar-forge-slot">
    <button
      class="forge-pill authed"
      data-testid="statusbar-forge-pill"
      data-kind={p.kind}
      data-state="authed"
      onclick={() => onNavigate("integrations")}
      type="button"
      title={p.kind === "github" ? m.provider_github() : m.provider_gitlab()}
    >
      <span class="dot" aria-hidden="true"></span>
      <span class="label"
        >{p.kind === "github" ? m.provider_github() : m.provider_gitlab()}</span
      >
    </button>
  </div>
{/if}

<style>
  .forge-slot {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 100%;
    padding: 0 8px;
  }

  .forge-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: none;
    padding: 0;
    color: var(--text-secondary);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    user-select: none;
    transition: color 0.15s;
  }

  .forge-pill:hover {
    color: var(--text-primary);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-secondary);
    flex-shrink: 0;
  }

  .forge-pill.authed .dot {
    background: var(--accent-green);
  }

  .label {
    line-height: 1;
  }
</style>
