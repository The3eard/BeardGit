<!--
  PillRow — a row of pills (labels, reviewers) each with a `×` remove button,
  plus a trailing `+` button that triggers a user-supplied callback (typically
  opens a picker dialog).

  Used by MrPrDetail for both the labels and the reviewers sections.
-->
<script lang="ts">
  interface Props {
    /** Items to render as pills (one per entry). */
    items: string[];
    /** When true, the remove `×` and add `+` buttons are disabled. */
    disabled?: boolean;
    /** Called with the item text when the `×` on a pill is clicked. */
    onRemove: (item: string) => void;
    /** Called when the trailing `+` button is clicked. */
    onAddClick: () => void;
    /** Text shown when `items` is empty. */
    emptyLabel?: string;
    /** Additional CSS class applied to each pill (allows per-kind styling). */
    pillClass?: string;
    /** Optional tooltip provider, called per item. */
    tooltipFor?: (item: string) => string | undefined;
    /** Accessible label template for the remove buttons. Receives the item. */
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
    <span class="pill {pillClass}" title={tooltipFor?.(item) ?? item}>
      <span class="pill-label">{item}</span>
      <button
        class="pill-remove nf"
        type="button"
        {disabled}
        aria-label={removeAriaLabel ? removeAriaLabel(item) : `Remove ${item}`}
        onclick={() => onRemove(item)}>{"\uF00D"}</button>
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
    background: rgba(88, 166, 255, 0.15);
    color: var(--accent-blue);
    font-size: 11px;
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
  .pill-remove:hover { opacity: 1; background: rgba(255, 255, 255, 0.1); }
  .pill-remove:disabled { opacity: 0.3; cursor: not-allowed; }
  .pill-empty {
    color: var(--text-secondary);
    font-size: 11px;
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
  .pill-add:hover { color: var(--accent-blue); border-color: var(--accent-blue); }
  .pill-add:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
