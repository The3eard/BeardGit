<!--
  Native-brand provider icon.

  Renders the Anthropic / OpenAI / OpenCode glyph SVG as an `<img>` at the
  requested size with a transparent background. Falls back to a neutral
  tool glyph for providers not in the known set. No enclosing square —
  callers that want a coloured chip wrap this themselves.
-->
<script lang="ts">
  import type { AiProviderKind } from "$lib/types";
  import claudeIcon from "./icons/claude-code.svg";
  import codexIcon from "./icons/codex.svg";
  import opencodeIcon from "./icons/opencode.svg";
  import genericIcon from "./icons/generic.svg";

  interface Props {
    /** Provider kind. Anything unrecognised falls back to the generic glyph. */
    provider: AiProviderKind | string;
    /** Square side in px. Defaults to 20 to match the session-list slot. */
    size?: number;
    /** Optional extra classes for layout (e.g. margin). */
    className?: string;
  }

  const { provider, size = 20, className = "" }: Props = $props();

  const ASSETS: Record<string, string> = {
    claude_code: claudeIcon,
    codex: codexIcon,
    open_code: opencodeIcon,
  };

  const src = $derived(ASSETS[provider] ?? genericIcon);
  const alt = $derived(`${provider} icon`);
</script>

<img
  class={`provider-icon ${className}`}
  {src}
  {alt}
  width={size}
  height={size}
  draggable="false"
  data-testid={`provider-icon-${provider}`}
/>

<style>
  .provider-icon {
    display: inline-block;
    flex-shrink: 0;
    object-fit: contain;
    background: transparent;
  }
</style>
