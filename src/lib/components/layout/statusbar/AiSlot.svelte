<!--
  AiSlot — shows the user's preferred AI provider via its brand icon, or
  a muted "AI" label when no provider is installed / preferred.

  Uses the shared `ProviderIcon` component at 14 px so the icon sits
  comfortably inside the 22 px statusbar. When `preferredAiProvider`
  is `null` we render a grey dot + "AI" label so the slot still offers
  an affordance — clicking jumps to the AI settings section (the actual
  deep-link lands with the Settings IA overhaul in MT-5).
-->
<script lang="ts">
  import { preferredAiProvider } from "$lib/stores/ai";
  import ProviderIcon from "$lib/components/ai-sessions/ProviderIcon.svelte";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Navigate to a Settings sub-section; parent wires actual routing. */
    onNavigate: (section: string) => void;
  }

  const { onNavigate }: Props = $props();

  let provider = $derived($preferredAiProvider);
</script>

<button
  class="ai-slot"
  title={m.statusbar_ai_tooltip()}
  data-testid="statusbar-ai-slot"
  data-has-provider={provider ? "true" : "false"}
  onclick={() => onNavigate("ai")}
  type="button"
>
  {#if provider}
    <ProviderIcon {provider} size={14} />
  {:else}
    <span class="dot" aria-hidden="true"></span>
    <span class="label">{m.statusbar_ai_label()}</span>
  {/if}
</button>

<style>
  .ai-slot {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 100%;
    padding: 0 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-xs);
    cursor: pointer;
    user-select: none;
    transition: color 0.15s;
  }

  .ai-slot:hover {
    color: var(--text-primary);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-secondary);
    opacity: 0.6;
    flex-shrink: 0;
  }

  .label {
    line-height: 1;
  }
</style>
