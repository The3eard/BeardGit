<!--
  SessionRow — shared single-line row primitive for the AI Sessions list.

  Used by `ConversationRow` and `ActiveRow` to enforce one consistent
  shape across the two sections: `[provider icon]  {title}  {relative
  date}`. All action buttons, provider-name spans, cwd, forked badges and
  status badges live in `AiSessionDetail.svelte` instead.

  Selection state is driven top-down via the `selected` prop — the row
  itself is purely presentational. `onSelect` fires on click and on
  Enter/Space so the row behaves as a button for keyboard users.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** Leading icon — typically `<ProviderIcon />` rendered via a snippet. */
    icon: Snippet;
    /** Row title; receives `white-space: nowrap` + ellipsis. */
    title: string;
    /** Trailing relative-time label; renders `—` when null. */
    date: string | null;
    /** When true, applies the `.selected` accent-bar + tint. */
    selected: boolean;
    /** Called on click or Enter/Space. */
    onSelect: () => void;
  }

  let { icon, title, date, selected, onSelect }: Props = $props();

  /** Keyboard activation — Enter and Space both select. */
  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onSelect();
    }
  }
</script>

<div
  class="session-row"
  class:selected
  role="button"
  tabindex="0"
  onclick={onSelect}
  onkeydown={onKey}
>
  <span class="row-icon">{@render icon()}</span>
  <span class="row-title" title={title}>{title}</span>
  <span class="row-date">{date ?? "—"}</span>
</div>

<style>
  .session-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    box-sizing: border-box;
    cursor: pointer;
    transition: background 0.1s;
    border-left: 2px solid transparent;
  }

  .session-row:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: -2px;
  }

  .session-row.selected {
    background: color-mix(in srgb, var(--accent-blue) 10%, transparent);
    border-left-color: var(--accent-blue);
  }

  .row-icon {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
  }

  .row-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-primary);
  }

  .row-date {
    flex-shrink: 0;
    font-size: 10px;
    color: var(--text-secondary);
    white-space: nowrap;
  }
</style>
