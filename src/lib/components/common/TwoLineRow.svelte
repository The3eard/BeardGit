<!--
  TwoLineRow — shared two-line row primitive for list panels.

  Line 1: [leadIcon] [keyLabel?] [title                              ] [trailingDate?]
  Line 2: meta (flex-wrap, aligned under the title — NOT the icon).

  Every slot except `selected` is a Svelte 5 snippet the caller supplies;
  the row is purely presentational — keyboard focus, click handlers, and
  `role="option"` stay on the `<div class="list-row">` that wraps it in
  `List.svelte`.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** Column 1: status glyph or dot. */
    leadIcon: Snippet;
    /** Column 2: monospace identifier (e.g. `#42`, `running`). Omittable. */
    keyLabel?: Snippet;
    /** Column 3 (1fr): headline text, single-line, ellipsed. */
    title: Snippet;
    /** Column 4: right-aligned relative date/time. Omittable. */
    trailingDate?: Snippet;
    /** Line 2: free-flowing metadata chips (flex-wrap, gap 8). */
    meta: Snippet;
    /** Adds the `.two-line-row--selected` modifier. */
    selected: boolean;
  }

  let { leadIcon, keyLabel, title, trailingDate, meta, selected }: Props = $props();
</script>

<div class="two-line-row" class:two-line-row--selected={selected}>
  <div class="two-line-row__line1">
    <span class="two-line-row__lead">{@render leadIcon()}</span>
    {#if keyLabel}
      <span class="two-line-row__key">{@render keyLabel()}</span>
    {/if}
    <span class="two-line-row__title">{@render title()}</span>
    {#if trailingDate}
      <span class="two-line-row__date">{@render trailingDate()}</span>
    {/if}
  </div>
  <div class="two-line-row__meta">{@render meta()}</div>
</div>

<style>
  .two-line-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    width: 100%;
    min-width: 0;
    color: var(--text-primary);
  }
  /* .two-line-row--selected: selection bg is painted by .list-row.selected in list.css */

  .two-line-row__line1 {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .two-line-row__lead {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 16px;
  }
  .two-line-row__key {
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
  }
  .two-line-row__title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 500;
  }
  .two-line-row__date {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .two-line-row__meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    /* Align meta under the title: lead (16) + gap (8) + key typical (~32) + gap (8) ≈ 64.
       We pick 28 (lead + one gap) so short keyLabels don't leave a visual gutter. */
    padding-left: 28px;
    font-size: 11px;
    color: var(--text-secondary);
    min-width: 0;
  }
</style>
