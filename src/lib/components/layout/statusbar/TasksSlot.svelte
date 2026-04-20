<!--
  TasksSlot — the leftmost slot of the lean statusbar.

  Renders a Nerd-font checklist glyph + the count of *currently running*
  tasks as reported by the aggregator store (`activeTaskCount`). Two
  overlays layer on top of the glyph:

    - **Pulse animation** — fires briefly whenever the count grows, so
      the user gets peripheral-vision feedback that new work is starting.
    - **Red dot** — shown while `hasUnseenError` is true; cleared when the
      drawer opens (`markSeen()` in the drawer).

  Clicking the slot calls the parent-provided `onOpen` handler, which
  flips the `tasksDrawerOpen` store in the root layout.

  The slot is always visible. When `activeTaskCount === 0`, only the
  icon renders (no count badge), so the statusbar stays quiet but the
  control is still clickable.
-->
<script lang="ts">
  import { activeTaskCount, hasUnseenError } from "$lib/stores/tasks";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Called when the user clicks the slot. Parent opens the drawer. */
    onOpen: () => void;
  }

  const { onOpen }: Props = $props();

  let count = $derived($activeTaskCount);
  let unseenError = $derived($hasUnseenError);

  /**
   * Previous count snapshot used to detect increases and trigger the
   * pulse animation. `hasPrevious` distinguishes "first render, skip
   * pulse" from "real increase". Both are plain mutable — the $effect
   * only reads `count` reactively.
   */
  let previousCount = 0;
  let hasPrevious = false;
  let pulsing = $state(false);
  let pulseTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (hasPrevious && count > previousCount) {
      pulsing = true;
      if (pulseTimer) clearTimeout(pulseTimer);
      pulseTimer = setTimeout(() => {
        pulsing = false;
      }, 600);
    }
    previousCount = count;
    hasPrevious = true;
  });
</script>

<button
  class="tasks-slot"
  class:pulsing
  class:has-error={unseenError}
  onclick={onOpen}
  title={m.statusbar_tasks_tooltip()}
  data-testid="statusbar-tasks-slot"
  data-count={count}
  type="button"
>
  <span class="icon-wrap">
    <span class="nf" aria-hidden="true">{"\uF46A"}</span>
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
    font-size: 11px;
    cursor: pointer;
    user-select: none;
    transition: color 0.15s;
  }

  .tasks-slot:hover {
    color: var(--text-primary);
  }

  .tasks-slot.has-error {
    color: var(--accent-red);
  }

  .tasks-slot.pulsing .icon-wrap {
    animation: tasks-pulse 600ms ease-out;
  }

  .icon-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
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

  @keyframes tasks-pulse {
    0%   { transform: scale(1);   opacity: 1;   }
    30%  { transform: scale(1.25); opacity: 0.85; }
    100% { transform: scale(1);   opacity: 1;   }
  }
</style>
