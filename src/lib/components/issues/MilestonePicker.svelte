<!--
  MilestonePicker — popover dropdown for selecting one (or none) milestone.
  Lazily refreshes the milestones cache on mount.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    milestonesCache,
    milestonesCacheLoading,
    refreshMilestonesCache,
  } from "../../stores/issues";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Currently selected milestone id, or null for no milestone. */
    current: number | null;
    /** Fired with the new milestone id (or null) on confirm. */
    onConfirm: (id: number | null) => void;
    /** Fired when the user dismisses the dialog. */
    onCancel: () => void;
  }

  let { current, onConfirm, onCancel }: Props = $props();

  onMount(() => {
    if ($milestonesCache.length === 0) void refreshMilestonesCache();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
  }
</script>

<button class="overlay" type="button" aria-label={m.issues_cancel()} onclick={onCancel}></button>
<!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
<div class="picker" role="dialog" tabindex="-1" onkeydown={handleKeydown}>
  <h3 class="picker-title">{m.issues_milestone_picker_title()}</h3>
  {#if $milestonesCacheLoading && $milestonesCache.length === 0}
    <div class="picker-empty">{m.issues_loading()}</div>
  {:else}
    <div class="picker-list">
      <button
        class="picker-item"
        class:selected={current === null}
        type="button"
        onclick={() => onConfirm(null)}
      >
        <span class="picker-name"><em>{m.issues_milestone_picker_none()}</em></span>
      </button>
      {#each $milestonesCache as ms (ms.id)}
        <button
          class="picker-item"
          class:selected={current === ms.id}
          type="button"
          onclick={() => onConfirm(ms.id)}
        >
          <span class="picker-name">{ms.title}</span>
          {#if ms.state === "closed"}
            <span class="closed">{m.issues_milestone_picker_closed()}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4); /* beardgit:allow-hex: modal backdrop neutral */
    z-index: 99;
    border: none;
    cursor: pointer;
  }
  .picker {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 100;
    min-width: 320px;
    max-height: 60vh;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-shadow: var(--shadow-modal);
  }
  .picker-title {
    margin: 0;
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
  }
  .picker-list {
    overflow-y: auto;
    max-height: 40vh;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .picker-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 10px;
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--text-primary);
    font-size: var(--font-size-md);
    cursor: pointer;
    text-align: left;
  }
  .picker-item:hover {
    background: color-mix(in srgb, var(--text-primary) 5%, transparent);
  }
  .picker-item.selected {
    background: var(--overlay-accent-blue);
    color: var(--accent-primary);
    border-color: var(--accent-primary);
  }
  .picker-name {
    font-size: var(--font-size-md);
  }
  .closed {
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
    font-weight: 600;
  }
  .picker-empty {
    text-align: center;
    padding: 16px;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }
</style>
