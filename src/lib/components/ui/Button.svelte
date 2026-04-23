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
     * Visual variant — picks colour/border tokens. Default `'secondary'`.
     *
     * - `primary`: loud, accent-blue fill. Use for the single most
     *   important action in a row (Connect, Save, Submit).
     * - `secondary`: tonal baseline fill, borderless, softer than
     *   `subtle`. The default. Use when you want "this is a button" but
     *   the row already has another button carrying the action weight.
     * - `subtle`: tonal fill with a visible border — reads as
     *   actionable but not primary. Use for Manage / Edit / secondary
     *   toggles where `ghost` would read as disabled and `secondary`
     *   is too flat.
     * - `ghost`: transparent baseline, inherits the row colour. Use
     *   for inline icon buttons (close, dismiss, row-chevron) where a
     *   fill would feel heavy.
     * - `danger`: loud, accent-red fill. Use for destructive actions
     *   (Disconnect, Delete).
     */
    variant?: "primary" | "secondary" | "subtle" | "ghost" | "danger";
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
    /** Optional `data-testid` forwarded to the underlying `<button>`. */
    testid?: string;
    /** Slot for label text/children. */
    children?: import("svelte").Snippet;
  }

  let {
    variant = "secondary",
    size = "md",
    loading = false,
    disabled = false,
    icon,
    type = "button",
    onclick,
    ariaLabel,
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
  disabled={disabled || loading}
  aria-busy={loading ? "true" : undefined}
  aria-label={ariaLabel}
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

  /* Variants */
  .bg-btn--primary {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    color: var(--text-primary);
  }
  .bg-btn--primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .bg-btn--secondary {
    background: var(--bg-secondary);
    border-color: transparent;
    color: var(--text-primary);
  }
  .bg-btn--secondary:hover:not(:disabled) {
    background: var(--overlay-hover);
  }

  .bg-btn--subtle {
    background: var(--bg-secondary);
    border-color: var(--border);
    color: var(--text-primary);
  }
  .bg-btn--subtle:hover:not(:disabled) {
    background: var(--overlay-hover);
    border-color: var(--accent-blue);
  }

  .bg-btn--ghost {
    background: transparent;
    border-color: transparent;
    color: var(--text-secondary);
  }
  .bg-btn--ghost:hover:not(:disabled) {
    color: var(--text-primary);
    background: var(--overlay-hover);
  }

  .bg-btn--danger {
    background: var(--accent-red);
    border-color: var(--accent-red);
    color: var(--text-primary);
  }
  .bg-btn--danger:hover:not(:disabled) {
    opacity: 0.9;
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
