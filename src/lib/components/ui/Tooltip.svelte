<!--
  Tooltip — styled, fast tooltip primitive.

  Replaces the native `title=` attribute, which has a ~700 ms delay,
  no styling, no shortcut hint, and dismisses unpredictably. The
  audit flagged this as a top discoverability blocker — every
  IconButton in the app delegated its label to the OS tooltip.

  Usage:
    <Tooltip text="Refresh repository" shortcut="Cmd+R">
      <IconButton icon={...} description="Refresh" onclick={...} />
    </Tooltip>

  Visual: dark popover, monospaced shortcut chip, 250 ms show delay,
  no exit animation (snap-out keeps the cursor responsive).
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** Tooltip body. Required. */
    text: string;
    /** Optional keyboard shortcut hint, rendered as a `<kbd>` chip. */
    shortcut?: string;
    /** Slot for the trigger element (button, icon, etc.). */
    children?: Snippet;
    /** Override the show delay, in milliseconds. Defaults to 250 ms. */
    delayMs?: number;
    /** Vertical placement preference: `top` (default) or `bottom`. */
    placement?: "top" | "bottom";
  }

  const { text, shortcut, children, delayMs = 250, placement = "top" }: Props = $props();

  let visible = $state(false);
  let triggerEl: HTMLSpanElement | undefined = $state();
  let timer: ReturnType<typeof setTimeout> | undefined;

  function schedule() {
    clearTimeout(timer);
    timer = setTimeout(() => { visible = true; }, delayMs);
  }

  function cancel() {
    clearTimeout(timer);
    visible = false;
  }
</script>

<span
  class="tooltip-trigger"
  bind:this={triggerEl}
  onmouseenter={schedule}
  onmouseleave={cancel}
  onfocusin={schedule}
  onfocusout={cancel}
>
  {@render children?.()}
  {#if visible}
    <span class="tooltip tooltip--{placement}" role="tooltip" aria-live="polite">
      <span class="tooltip__text">{text}</span>
      {#if shortcut}
        <kbd class="tooltip__shortcut">{shortcut}</kbd>
      {/if}
    </span>
  {/if}
</span>

<style>
  .tooltip-trigger {
    position: relative;
    display: inline-flex;
  }

  .tooltip {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    z-index: 1000;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: var(--bg-toolbar);
    color: var(--text-primary);
    font-size: 11px;
    line-height: 1.4;
    white-space: nowrap;
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 2px 8px var(--overlay-shadow);
    pointer-events: none;
    user-select: none;
  }

  .tooltip--top {
    bottom: calc(100% + 6px);
  }

  .tooltip--bottom {
    top: calc(100% + 6px);
  }

  .tooltip__shortcut {
    padding: 1px 5px;
    background: var(--overlay-active);
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
  }
</style>
