<!--
  PillRow — a row of pills (labels, reviewers) each with a `×` remove button,
  plus a trailing `+` button that triggers a user-supplied callback (typically
  opens a picker dialog).

  Used by MrPrDetail for both the labels and the reviewers sections.
-->
<script lang="ts">
  interface Props {
    /**
     * Items to render as pills. Plain strings are treated as names with no
     * color; label objects carry an optional `color` (hex without `#`) that
     * is used to tint the pill background and text.
     */
    items: (string | { name: string; color?: string | null })[];
    /** When true, the remove `×` and add `+` buttons are disabled. */
    disabled?: boolean;
    /** Called with the item name when the `×` on a pill is clicked. */
    onRemove: (item: string) => void;
    /** Called when the trailing `+` button is clicked. */
    onAddClick: () => void;
    /** Text shown when `items` is empty. */
    emptyLabel?: string;
    /** Additional CSS class applied to each pill (allows per-kind styling). */
    pillClass?: string;
    /** Optional tooltip provider, called per item name. */
    tooltipFor?: (item: string) => string | undefined;
    /** Accessible label template for the remove buttons. Receives the name. */
    removeAriaLabel?: (item: string) => string;
    /** Accessible label for the trailing add button. */
    addAriaLabel?: string;
  }

  let {
    items,
    disabled = false,
    onRemove,
    onAddClick,
    emptyLabel = "",
    pillClass = "",
    tooltipFor,
    removeAriaLabel,
    addAriaLabel,
  }: Props = $props();
</script>

<div class="pill-row">
  {#each items as item}
    {@const label = typeof item === "string" ? { name: item, color: null } : item}
    <span
      class="pill {pillClass}"
      style:background={label.color ? `#${label.color}20` : undefined}
      style:color={label.color ? `#${label.color}` : undefined}
      title={tooltipFor?.(label.name) ?? label.name}
    >
      <span class="pill-label">{label.name}</span>
      <button
        class="pill-remove nf"
        type="button"
        {disabled}
        aria-label={removeAriaLabel ? removeAriaLabel(label.name) : `Remove ${label.name}`}
        onclick={() => onRemove(label.name)}>{"\uF00D"}</button>
    </span>
  {/each}
  {#if items.length === 0 && emptyLabel}
    <span class="pill-empty">{emptyLabel}</span>
  {/if}
  <button
    class="pill-add nf"
    type="button"
    {disabled}
    aria-label={addAriaLabel ?? "Add"}
    onclick={onAddClick}>{"\uF067"}</button>
</div>

<style>
  .pill-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 2px 8px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
    color: var(--accent-primary);
    font-size: var(--font-size-xs);
  }
  .pill-label {
    line-height: 1.2;
  }
  .pill-remove {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: inherit;
    font-family: var(--font-icons);
    font-size: 9px;
    cursor: pointer;
    opacity: 0.7;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .pill-remove:hover { opacity: 1; background: color-mix(in srgb, var(--text-primary) 10%, transparent); }
  .pill-remove:disabled { opacity: 0.3; cursor: not-allowed; }
  .pill-empty {
    color: var(--text-secondary);
    font-size: var(--font-size-xs);
    font-style: italic;
  }
  .pill-add {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px dashed var(--border);
    background: none;
    color: var(--text-secondary);
    font-family: var(--font-icons);
    font-size: 9px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .pill-add:hover { color: var(--accent-primary); border-color: var(--accent-primary); }
  .pill-add:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
