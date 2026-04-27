<!--
  Button.svelte — shared button primitive for the MT-5 settings IA overhaul.

  Replaces the grab-bag of `.refresh-btn` / `.action-btn` / `.add-btn` /
  `.btn-primary` style classes that sprouted across the settings tree. All
  colours come from existing CSS tokens in `src/app.css`; no new theme
  variables are introduced.

  Consumers pass a `variant`, optional `size`, optional `icon` (a
  NerdFont glyph like `"\uF021"`), and a label slot:

  ```svelte
  <Button variant="primary" icon="\uF021" onclick={refresh}>Refresh</Button>
  ```

  The `loading` prop swaps the icon for a spinner and disables click
  handling. `disabled` also disables click handling. Both are surfaced via
  the HTML `disabled` attribute so assistive tech and browsers treat the
  element as inert.
-->
<script lang="ts">
  interface Props {
    /**
     * Visual variant — picks colour/border tokens. Default `'neutral'`.
     *
     * - `primary`: loud, accent-blue fill. Tonal-at-rest, solid-on-hover.
     *   Use for the main affirmative action (Save, Confirm, Create, Open,
     *   Submit, Connect, Checkout, Continue, Publish, Reopen).
     * - `success`: loud, accent-green fill. Tonal-at-rest, solid-on-hover.
     *   Use for git-state-changing affirmatives (Merge, Apply stash,
     *   Pop stash, Approve PR, Resolve conflict).
     * - `danger`: loud, accent-red fill. Tonal-at-rest, solid-on-hover.
     *   Use for destructive actions (Discard changes, Delete, Remove,
     *   Force-delete, Reset, Abort merge).
     * - `neutral`: canonical non-accent button. Tonal baseline fill,
     *   transparent border. Use for everything else (Cancel, Close,
     *   Refresh, Edit, Manage, Retry, Load more, Dismiss, Show in graph).
     */
    variant?: "primary" | "success" | "danger" | "neutral";
    /** Vertical rhythm/padding scale. Default `'md'`. */
    size?: "sm" | "md" | "lg";
    /** When true, swap the icon for a spinner and suppress clicks. */
    loading?: boolean;
    /** When true, renders disabled and suppresses clicks. */
    disabled?: boolean;
    /** Optional leading icon — a NerdFont glyph codepoint. */
    icon?: string;
    /** Button form semantics. Default `'button'`. */
    type?: "button" | "submit";
    /** Click handler; skipped while `loading` or `disabled`. */
    onclick?: (event: MouseEvent) => void;
    /** Optional `aria-label` for icon-only buttons (a11y). */
    ariaLabel?: string;
    /**
     * Optional hover description. When set, surfaces as the native
     * browser `title` tooltip. Also used as the fallback `aria-label`
     * when `ariaLabel` is not provided, so a button stays accessible
     * for screen readers without callers having to repeat themselves.
     */
    description?: string;
    /**
     * Toggle / "selected" state. When true on `variant="neutral"`, the
     * button takes on `primary`'s tonal-at-rest styling (blue tint
     * background, blue text, blue-tinted border) so segmented controls,
     * dropdown triggers, and toggle buttons can read as "this is
     * pressed/expanded/selected" without changing variant.
     */
    active?: boolean;
    /** `aria-haspopup` attribute for dropdown triggers. */
    ariaHaspopup?: "menu" | "true" | "false";
    /** `aria-expanded` attribute for dropdown triggers. */
    ariaExpanded?: boolean;
    /** Optional `data-testid` forwarded to the underlying `<button>`. */
    testid?: string;
    /** Slot for label text/children. */
    children?: import("svelte").Snippet;
  }

  let {
    variant = "neutral",
    size = "md",
    loading = false,
    disabled = false,
    icon,
    type = "button",
    onclick,
    ariaLabel,
    description,
    active = false,
    ariaHaspopup,
    ariaExpanded,
    testid,
    children,
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
  {type}
  class="bg-btn bg-btn--{variant} bg-btn--{size}"
  class:bg-btn--loading={loading}
  class:bg-btn--active={active}
  disabled={disabled || loading}
  aria-busy={loading ? "true" : undefined}
  aria-label={ariaLabel ?? description}
  aria-haspopup={ariaHaspopup}
  aria-expanded={ariaExpanded}
  title={description}
  data-variant={variant}
  data-size={size}
  data-testid={testid}
  onclick={handleClick}
>
  {#if loading}
    <span class="bg-btn__spinner" aria-hidden="true"></span>
  {:else if icon}
    <span class="bg-btn__icon nf" aria-hidden="true">{icon}</span>
  {/if}
  {#if children}
    <span class="bg-btn__label">{@render children()}</span>
  {/if}
</button>

<style>
  .bg-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: 6px;
    border: 1px solid var(--border);
    cursor: pointer;
    font-family: inherit;
    line-height: 1;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease,
      opacity 0.15s ease;
    color: var(--text-primary);
    background: var(--overlay-hover);
  }

  .bg-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .bg-btn--loading {
    cursor: progress;
  }

  /* Sizes */
  .bg-btn--sm {
    padding: 3px 10px;
    font-size: 11px;
  }
  .bg-btn--md {
    padding: 6px 14px;
    font-size: 12px;
  }
  .bg-btn--lg {
    padding: 8px 18px;
    font-size: 13px;
  }

  /* Variants
     -----------------------------------------------------------------
     `primary`, `success`, and `danger` share a tonal-at-rest,
     solid-on-hover pattern: a translucent tint of the accent at
     rest so the button reads as the meaningful CTA without looking
     pre-selected, then ramping up to the full accent on hover so
     the interactive feedback is unmistakable. Earlier rules used
     a solid fill at rest with `opacity: 0.9` on hover, which made
     every primary or destructive button look "highlighted" before
     the user even touched it. */
  .bg-btn--primary {
    background: color-mix(in srgb, var(--accent-blue) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent-blue) 60%, transparent);
    color: var(--accent-blue);
  }
  .bg-btn--primary:hover:not(:disabled) {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    color: var(--text-primary);
  }

  .bg-btn--success {
    background: color-mix(in srgb, var(--accent-green) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent-green) 60%, transparent);
    color: var(--accent-green);
  }
  .bg-btn--success:hover:not(:disabled) {
    background: var(--accent-green);
    border-color: var(--accent-green);
    color: var(--text-primary);
  }

  /* `neutral` is the canonical non-accent button. Tonal baseline fill,
     transparent border. Use for Cancel, Close, Refresh, Edit, Manage,
     Retry, Load more, Dismiss, and any other non-accent text action. */
  .bg-btn--neutral {
    background: var(--bg-secondary);
    border-color: transparent;
    color: var(--text-primary);
  }
  .bg-btn--neutral:hover:not(:disabled) {
    background: var(--overlay-hover);
  }

  .bg-btn--danger {
    background: color-mix(in srgb, var(--accent-red) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent-red) 60%, transparent);
    color: var(--accent-red);
  }
  .bg-btn--danger:hover:not(:disabled) {
    background: var(--accent-red);
    border-color: var(--accent-red);
    color: var(--text-primary);
  }

  .bg-btn__icon {
    font-family: var(--font-icons);
    font-size: 1em;
    line-height: 1;
  }

  .bg-btn__label {
    display: inline-flex;
    align-items: center;
  }

  /* Active / pressed / selected state for the neutral variant.
     Mirrors `primary`'s tonal-at-rest values so a toggle pill, a
     dropdown trigger, or a segmented-control selection reads as
     "this is the chosen one" while staying part of the neutral
     family at rest. Hover stays at neutral hover so the user gets
     clear feedback that re-clicking does something. */
  .bg-btn--neutral.bg-btn--active {
    background: color-mix(in srgb, var(--accent-blue) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent-blue) 60%, transparent);
    color: var(--accent-blue);
  }
  .bg-btn--neutral.bg-btn--active:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent-blue) 28%, transparent);
  }

  .bg-btn__spinner {
    width: 12px;
    height: 12px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    display: inline-block;
    animation: bg-btn-spin 0.6s linear infinite;
    opacity: 0.85;
  }

  @keyframes bg-btn-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
