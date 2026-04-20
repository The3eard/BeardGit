<!--
  ForgeSlot — renders one pill per currently-configured forge provider
  (GitHub / GitLab) read from the `providerStatus` store.

  Each pill:
    - Green dot  → provider is authed and active.
    - Yellow dot → token expired or otherwise in a recoverable bad state.
    - Grey dot   → registered but currently unavailable.

  Hidden entirely when no forge provider is configured — the 1 px divider
  that follows is the parent's concern, so we simply return an empty
  fragment in that case.

  The component is provider-agnostic: the list of pills is derived from
  `providerStatus.providers` and the active index. Clicking any pill
  invokes the parent-provided `onNavigate("integrations")` — until the
  Settings IA overhaul (MT-5) supports deep-linking to a sub-section,
  that call just routes to Settings.
-->
<script lang="ts">
  import { providerStatus } from "$lib/stores/provider";
  import * as m from "$lib/paraglide/messages";
  import type { ConnectedProvider } from "$lib/types";

  interface Props {
    /** Navigate to a specific Settings sub-section (or Settings root). */
    onNavigate: (section: string) => void;
  }

  const { onNavigate }: Props = $props();

  interface ForgePill {
    key: string;
    kind: "github" | "gitlab";
    label: string;
    /** "authed" | "expired" | "unavailable" */
    state: "authed" | "expired" | "unavailable";
    title: string;
  }

  function providerLabel(p: ConnectedProvider): string {
    return p.kind === "github" ? m.provider_github() : m.provider_gitlab();
  }

  function providerState(
    p: ConnectedProvider,
  ): "authed" | "expired" | "unavailable" {
    // The backend doesn't yet expose an `expired` flag — treat every
    // connected provider as `authed`. Reserved for future work.
    if (!p.user || !p.user.username) return "unavailable";
    return "authed";
  }

  let pills = $derived<ForgePill[]>(
    $providerStatus.providers.map((p, i) => {
      const state = providerState(p);
      const suffix =
        state === "authed"
          ? m.statusbar_forge_authed()
          : state === "expired"
            ? m.statusbar_forge_expired()
            : m.statusbar_forge_unavailable();
      return {
        key: `${p.kind}-${p.instance_url || "default"}-${i}`,
        kind: p.kind,
        label: providerLabel(p),
        state,
        title: `${providerLabel(p)} — ${suffix}`,
      };
    }),
  );
</script>

{#if pills.length > 0}
  <div class="forge-slot" data-testid="statusbar-forge-slot">
    {#each pills as pill (pill.key)}
      <button
        class="forge-pill"
        class:authed={pill.state === "authed"}
        class:expired={pill.state === "expired"}
        class:unavailable={pill.state === "unavailable"}
        title={pill.title}
        data-testid="statusbar-forge-pill"
        data-kind={pill.kind}
        data-state={pill.state}
        onclick={() => onNavigate("integrations")}
        type="button"
      >
        <span class="dot" aria-hidden="true"></span>
        <span class="label">{pill.label}</span>
      </button>
    {/each}
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

  .forge-pill.expired .dot {
    background: var(--accent-orange);
  }

  .forge-pill.unavailable .dot {
    background: var(--text-secondary);
    opacity: 0.6;
  }

  .label {
    line-height: 1;
  }
</style>
