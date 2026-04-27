<!--
  Brand logo for one of the three supported AI providers, rendered from the
  shared PROVIDER_META path data. Size defaults to 16px and adapts colour
  from the provider metadata; callers can override `fill` for hover states.
-->
<script lang="ts">
  import type { AiProviderKind } from "$lib/types";
  import { PROVIDER_META } from "$lib/data/ai-providers";

  interface Props {
    /** Which provider's logo to render. */
    provider: AiProviderKind;
    /** Square side in px. Default 16. */
    size?: number;
    /** Override brand colour (defaults to provider's accent). */
    fill?: string;
  }

  const { provider, size = 16, fill }: Props = $props();

  const meta = $derived(PROVIDER_META[provider]);
  const colour = $derived(fill ?? meta.color);
</script>

{#if meta}
  <svg
    class="brand-icon"
    viewBox={meta.brandViewBox}
    width={size}
    height={size}
    fill={colour}
    aria-hidden="true"
  >
    <path d={meta.brandPath} />
  </svg>
{/if}

<style>
  .brand-icon {
    display: inline-block;
    flex-shrink: 0;
  }
</style>
