<!--
  TasksSlot — the leftmost slot of the lean statusbar.

  Renders a Nerd-font checklist glyph that mirrors the state of the most
  recent background task. Four signals layer on top:

    - **Spinner ring** — replaces the glyph while any task is `running`
      (`anyRunning`), so in-flight work is obvious without watching the
      count. At rest the checklist glyph returns (the old rotating sync
      glyph read as a "refresh" button).
    - **State colour** — the glyph takes on the accent/green/red/muted
      colour for the latest task's status. Running wins over terminal
      states so a freshly-started task doesn't inherit the previous
      failure's red.
    - **Count badge** — shown when `activeTaskCount > 0` so the user
      sees at-a-glance how many tasks are in flight.
    - **Red dot** — shown while `hasUnseenError` is true; cleared when
      the popover opens (`markSeen()` in the popover).

  Clicking the slot calls the parent-provided `onOpen` handler, which
  flips the `tasksPopoverOpen` store in the root layout. A second click
  while the popover is already open closes it (handled by the parent
  toggle helper).
-->
<script lang="ts">
  import {
    activeTaskCount,
    anyRunning,
    hasUnseenError,
    latestEntry,
  } from "$lib/stores/tasks";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Called when the user clicks the slot. Parent opens the popover. */
    onOpen: () => void;
  }

  const { onOpen }: Props = $props();

  let count = $derived($activeTaskCount);
  let unseenError = $derived($hasUnseenError);
  let spinning = $derived($anyRunning);

  /**
   * Modifier that drives the CSS colour palette for the icon glyph.
   *
   * - `running` — accent blue/orange (matches the old spinner tint).
   * - `error`   — red (takes priority over unseen-error dot).
   * - `success` — green.
   * - `cancelled` — muted secondary text.
   * - `idle`    — default secondary text (no history, or nothing to
   *               emphasise yet).
   */
  let stateClass = $derived.by<
    "running" | "error" | "success" | "cancelled" | "idle"
  >(() => {
    const entry = $latestEntry;
    if (!entry) return "idle";
    switch (entry.status) {
      case "running":
        return "running";
      case "error":
        return "error";
      case "success":
        return "success";
      case "cancelled":
        return "cancelled";
      default:
        return "idle";
    }
  });
</script>

<button
  class="tasks-slot state-{stateClass}"
  class:spinning
  class:has-error={unseenError}
  onclick={onOpen}
  title={m.statusbar_tasks_tooltip()}
  data-testid="statusbar-tasks-slot"
  data-count={count}
  data-state={stateClass}
  type="button"
>
  <span class="icon-wrap">
    {#if spinning}
      <!-- A real spinner ring while work is in flight \u2014 the old
           rotating sync glyph read as a "refresh" button. -->
      <span class="slot-spinner" aria-hidden="true"></span>
    {:else}
      <span class="nf glyph" aria-hidden="true">{"\uF0AE"}</span>
    {/if}
    {#if unseenError}
      <span class="error-dot" data-testid="statusbar-tasks-error-dot"></span>
    {/if}
  </span>
  {#if count > 0}
    <span class="count" data-testid="statusbar-tasks-count">{count}</span>
  {/if}
</button>

<style>
  .tasks-slot {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 100%;
    padding: 0 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-xs);
    cursor: pointer;
    user-select: none;
    transition: color 0.15s;
  }

  .tasks-slot:hover {
    color: var(--text-primary);
  }

  /* State-driven glyph colour. Running wins over terminal states so a
     freshly-started task doesn't inherit a previous failure's red. */
  .tasks-slot.state-running {
    color: var(--accent-orange);
  }
  .tasks-slot.state-error {
    color: var(--accent-red);
  }
  .tasks-slot.state-success {
    color: var(--accent-green);
  }
  .tasks-slot.state-cancelled {
    color: var(--text-secondary);
  }

  /* Unseen-error dot keeps priority for attention even when the last
     task recovered — the dot itself stays red and the base colour
     bumps to red to match. */
  .tasks-slot.has-error {
    color: var(--accent-red);
  }

  .icon-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  .glyph {
    display: inline-block;
  }

  .slot-spinner {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1.5px solid color-mix(in srgb, currentColor 30%, transparent);
    border-top-color: currentColor;
    animation: tasks-spin 0.8s linear infinite;
  }

  .error-dot {
    position: absolute;
    top: -1px;
    right: -3px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-red);
    box-shadow: 0 0 0 1px var(--bg-toolbar);
  }

  .count {
    font-variant-numeric: tabular-nums;
    min-width: 8px;
    text-align: center;
  }

  @keyframes tasks-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .slot-spinner {
      animation: none;
    }
  }
</style>
