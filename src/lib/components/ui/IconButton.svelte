<!--
  IconButton.svelte — shared primitive for buttons that show only a
  Nerd Font glyph (no text label). Replaces the per-component
  `.btn-icon`, `.header-btn` (icon-only variants), `.refresh-btn` and
  `.icon-btn` classes that were duplicated across the tree.

  Visual contract: transparent at rest, on hover only the glyph color
  changes — no rectangular background fill. Icon-only buttons should
  read as inline glyphs that brighten on hover, not as boxed buttons.

  Accessibility contract: `description` is required and drives both
  the native browser `title` (hover tooltip) and the underlying
  `aria-label`, so an icon-only button is never silent to screen
  readers. Callers pass localized strings via paraglide (e.g.
  `description={m.tooltip_close()}`), which is why the prop is
  required even though the visual hover is the immediate motivation.

  Usage:

  ```svelte
  <IconButton
    icon={""}
    description={m.tooltip_close()}
    onclick={onClose}
  />
  ```

  `loading` swaps the glyph for a Nerd-Font spinner and disables the
  click. `disabled` greys the glyph and disables the click. `tone`
  picks a colour register: `default` (text-secondary → text-primary)
  or `danger` (text-secondary → accent-red on hover) for destructive
  actions.
-->
<script lang="ts">
  interface Props {
    /** Nerd Font glyph codepoint, e.g. `""` for the close `×`. */
    icon: string;
    /**
     * Hover description; required because icon-only buttons MUST have
     * an accessible label. Drives the native `title` and the underlying
     * `aria-label`. Pass a localized paraglide message.
     */
    description: string;
    /** Vertical rhythm. `md` matches `Button`'s md (~24px tall). */
    size?: "sm" | "md" | "lg";
    /** Colour register. `danger` highlights destructive actions on hover. */
    tone?: "default" | "danger";
    /** When true, swap the glyph for a spinner and suppress clicks. */
    loading?: boolean;
    /** When true, render disabled and suppress clicks. */
    disabled?: boolean;
    /** Click handler; skipped while loading or disabled. */
    onclick?: (event: MouseEvent) => void;
    /** Optional `data-testid` forwarded to the underlying `<button>`. */
    testid?: string;
    /** Override `aria-label`. Defaults to `description` when omitted. */
    ariaLabel?: string;
    /** Toggle / "selected" state — paints the glyph in accent-blue. */
    active?: boolean;
    /** `aria-haspopup` attribute for dropdown triggers. */
    ariaHaspopup?: "menu" | "true" | "false";
    /** `aria-expanded` attribute for dropdown triggers. */
    ariaExpanded?: boolean;
  }

  let {
    icon,
    description,
    size = "md",
    tone = "default",
    loading = false,
    disabled = false,
    onclick,
    testid,
    ariaLabel,
    active = false,
    ariaHaspopup,
    ariaExpanded,
  }: Props = $props();

  function handleClick(event: MouseEvent) {
    if (loading || disabled) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    onclick?.(event);
  }
</script>

<button
  type="button"
  class="ic-btn ic-btn--{size} ic-btn--{tone}"
  class:ic-btn--loading={loading}
  class:ic-btn--active={active}
  disabled={disabled || loading}
  aria-busy={loading ? "true" : undefined}
  aria-label={ariaLabel ?? description}
  aria-haspopup={ariaHaspopup}
  aria-expanded={ariaExpanded}
  title={description}
  data-testid={testid}
  onclick={handleClick}
>
  {#if loading}
    <span class="ic-btn__glyph nf" aria-hidden="true">{""}</span>
  {:else}
    <span class="ic-btn__glyph nf" aria-hidden="true">{icon}</span>
  {/if}
</button>

<style>
  .ic-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    line-height: 1;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: color 0.12s ease, opacity 0.12s ease;
  }

  .ic-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .ic-btn--loading {
    cursor: progress;
  }

  /* Sizes — purely about padding + glyph size. */
  .ic-btn--sm {
    padding: 2px 4px;
    font-size: 12px;
  }
  .ic-btn--md {
    padding: 4px 6px;
    font-size: 14px;
  }
  .ic-btn--lg {
    padding: 6px 8px;
    font-size: 16px;
  }

  /* Tones — the only colour change happens on the glyph. The button
     background stays transparent at every state, which is the whole
     point of this component vs the previous `.btn-icon` style. */
  .ic-btn--default:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .ic-btn--danger:hover:not(:disabled) {
    color: var(--accent-red);
  }

  /* Active / "this toggle is on" state — glyph paints in accent-blue
     regardless of hover. Used by the line-numbers toggle in
     MergeEditor, the columns toggle in GitGraph, the draft toggle
     in MrPrDetail, etc. */
  .ic-btn--active {
    color: var(--accent-blue);
  }
  .ic-btn--active:hover:not(:disabled) {
    color: var(--accent-blue);
  }

  .ic-btn__glyph {
    font-family: var(--font-icons);
    font-size: 1em;
    line-height: 1;
  }

  /* Spinner: spin the loading glyph in place. */
  .ic-btn--loading .ic-btn__glyph {
    animation: ic-btn-spin 0.9s linear infinite;
  }

  @keyframes ic-btn-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
