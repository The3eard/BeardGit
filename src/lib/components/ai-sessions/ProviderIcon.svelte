<!--
  Native-brand provider icon.

  Renders the Anthropic / OpenAI / OpenCode glyph SVG as an `<img>` at the
  requested size with a transparent background. Falls back to a neutral
  tool glyph for providers not in the known set. No enclosing square —
  callers that want a coloured chip wrap this themselves.

  OpenAI (Codex) and OpenCode logos ship in two-tone form, so we keep
  light + dark assets per brand and pick the right one off the active
  theme's `meta.mode`. `<img>` cannot resolve `currentColor` from the
  parent document, so a single asset per brand would either flatten the
  two-tone OpenCode mark or paint OpenAI's monoblossom in only one mode.
  Anthropic's Claude mark is monochrome and theme-agnostic, so it ships
  as a single asset.
-->
<script lang="ts">
  import type { AiProviderKind } from "$lib/types";
  import { activeTheme } from "$lib/stores/theme";
  import claudeIcon from "./icons/claude-code.svg";
  import openaiBlack from "./icons/openai-black.svg";
  import openaiWhite from "./icons/openai-white.svg";
  import opencodeLight from "./icons/opencode-light.svg";
  import opencodeDark from "./icons/opencode-dark.svg";
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

  /* `meta.mode` is "dark" or "light"; default to dark when no theme is
     loaded yet (matches the app's bootstrap default). */
  const isDark = $derived(($activeTheme?.meta.mode ?? "dark") === "dark");

  const src = $derived.by(() => {
    switch (provider) {
      case "claude_code":
        return claudeIcon;
      case "codex":
        return isDark ? openaiWhite : openaiBlack;
      case "open_code":
        return isDark ? opencodeDark : opencodeLight;
      default:
        return genericIcon;
    }
  });
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
