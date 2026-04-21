<!--
  TasksPopover — anchored popover for the statusbar tasks icon.

  Restores the pre-cluster-0.3 interaction where clicking the task icon
  opens a compact list of the most recent tasks, with a click-to-drill
  affordance that expands into a detail view. Replaces the bottom
  drawer the 0.3 refactor introduced.

  Layout has two modes:

    - **List mode** — up to 8 most recent entries (active first, then
      finished), each one rendered via `TaskEntryRow` plus an "open
      detail" chevron. Header carries a "Clear finished" button.
    - **Detail mode** — header with back arrow + title, body shows the
      selected task's metadata; the full console output stream is wired
      in by the follow-up slice (see `TaskDetailPanel.svelte`).

  Anchoring is via `position: fixed`, tucked above the statusbar
  (`bottom: 28px`). Close triggers:
    - `Esc` key.
    - Click outside the popover.
    - Clicking the ✕ in the header.
    - Clicking the statusbar icon again (handled by the parent toggle).
-->
<script lang="ts">
  import { tick } from "svelte";
  import {
    tasksStore,
    recentlyFinishedTasks,
    clearFinished,
    cancelTaskById,
    markSeen,
  } from "$lib/stores/tasks";
  import { closeTasksPopover } from "$lib/stores/tasksPopover";
  import type { TaskEntry, TaskAction } from "$lib/types/tasks";
  import TaskEntryRow from "./TaskEntryRow.svelte";
  import TaskDetailPanel from "./TaskDetailPanel.svelte";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Whether the popover is open. Parent-controlled. */
    open: boolean;
    /** Called when the popover should close. */
    onClose: () => void;
  }

  let { open, onClose }: Props = $props();

  /**
   * Maximum entries the compact list shows. Older rows live in the
   * aggregator's `tasksStore` but are hidden from the popover — the
   * user scrolls through Settings → Logs for the full history.
   */
  const MAX_ROWS = 8;

  /** Active entries = currently running, newest-first. */
  const active = $derived(
    $tasksStore
      .filter((t) => t.status === "running")
      .slice()
      .sort((a, b) => b.startedAt - a.startedAt),
  );

  /** Recently finished entries, newest-first (≤ 5 min window). */
  const finished = $derived(
    $recentlyFinishedTasks
      .slice()
      .sort((a, b) => (b.finishedAt ?? 0) - (a.finishedAt ?? 0)),
  );

  /** Compact list: active first, then finished, trimmed to MAX_ROWS. */
  const rows = $derived([...active, ...finished].slice(0, MAX_ROWS));

  /**
   * Id of the entry whose detail view is currently open, or `null` in
   * list mode. Reset to `null` whenever the popover closes so the next
   * open always starts in list mode.
   */
  let detailId = $state<string | null>(null);
  const detailEntry = $derived(
    detailId !== null
      ? ($tasksStore.find((t) => t.id === detailId) ?? null)
      : null,
  );

  /** DOM root for outside-click detection. */
  let popoverEl: HTMLDivElement | undefined = $state();
  /**
   * Rising-edge guard for the opening click.
   *
   * The statusbar `TasksSlot` button flips `tasksPopoverOpen` to `true`
   * on click; the same click event then bubbles up to our
   * `<svelte:window onclick>` handler. By that point Svelte's render
   * pass has already committed, so `popoverEl` is bound and the click
   * target (the statusbar button) is outside the popover — without this
   * guard, the click-outside handler would close the popover on the
   * very frame it opened.
   *
   * We flip `ready` to `false` synchronously when `open` transitions
   * `false → true`, then to `true` on the next microtask (`tick()`),
   * after the opening click has finished bubbling. `handleClickOutside`
   * short-circuits while `ready` is `false`.
   */
  let ready = $state(false);

  // React to `open` transitions: mark seen on open, reset detail on
  // close, and arm the click-outside guard for exactly one frame so the
  // opening click can't close the popover.
  $effect(() => {
    if (open) {
      markSeen();
      ready = false;
      void tick().then(() => {
        // Only flip `ready` true if the popover is still open — a
        // close-then-open dance within a single tick would otherwise
        // leave `ready` true for a future open.
        if (open) ready = true;
      });
    } else {
      detailId = null;
      ready = false;
    }
  });

  function handleClickOutside(event: MouseEvent) {
    if (!open || !ready) return;
    if (popoverEl && !popoverEl.contains(event.target as Node)) {
      onClose();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === "Escape") {
      event.preventDefault();
      // Escape from detail view goes back to the list first; a second
      // Escape closes the popover entirely.
      if (detailId !== null) {
        detailId = null;
      } else {
        onClose();
      }
    }
  }

  /** Route a row action back to the aggregator store. */
  async function handleAction(entry: TaskEntry, id: TaskAction["id"]) {
    if (id === "cancel") {
      await cancelTaskById(entry.id);
    } else if (id === "dismiss") {
      clearFinished();
    }
    // retry / open_output are wired up by kind-specific consumers.
  }

  function openDetail(entry: TaskEntry) {
    detailId = entry.id;
  }

  function backToList() {
    detailId = null;
  }

  /** Close the popover via the shared store helper (used by X button). */
  function closeX() {
    closeTasksPopover();
    onClose();
  }
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="tasks-popover"
    role="dialog"
    tabindex="-1"
    aria-modal="false"
    aria-label={m.tasks_title()}
    data-testid="tasks-popover"
    bind:this={popoverEl}
    onclick={(e) => e.stopPropagation()}
  >
    {#if detailEntry}
      <header class="popover-header" data-testid="tasks-popover-detail-header">
        <button
          type="button"
          class="btn-icon back-btn"
          aria-label={m.tasks_collapse_tooltip()}
          data-testid="tasks-popover-back"
          onclick={backToList}
        >
          <span class="nf" aria-hidden="true">{"\uF060"}</span>
        </button>
        <span class="header-title header-title--detail" title={detailEntry.title}
          >{detailEntry.title}</span
        >
        <button
          type="button"
          class="btn-icon close-btn"
          aria-label={m.tasks_close()}
          data-testid="tasks-popover-close"
          onclick={closeX}
        >
          <span class="nf" aria-hidden="true">{"\uF00D"}</span>
        </button>
      </header>
      <div class="popover-body popover-body--detail">
        <TaskDetailPanel entry={detailEntry} />
      </div>
    {:else}
      <header class="popover-header">
        <div class="header-left">
          <span class="header-title">{m.tasks_title()}</span>
          <span class="header-badge" data-testid="tasks-popover-badge"
            >{rows.length}</span
          >
        </div>
        <div class="header-actions">
          <button
            type="button"
            class="btn-cancel clear-btn"
            data-testid="tasks-popover-clear"
            disabled={finished.length === 0}
            onclick={clearFinished}
          >
            {m.tasks_clear_finished()}
          </button>
          <button
            type="button"
            class="btn-icon close-btn"
            aria-label={m.tasks_close()}
            data-testid="tasks-popover-close"
            onclick={closeX}
          >
            <span class="nf" aria-hidden="true">{"\uF00D"}</span>
          </button>
        </div>
      </header>

      <div class="popover-body" data-testid="tasks-popover-body">
        {#if rows.length === 0}
          <p class="empty" data-testid="tasks-popover-empty">
            {m.tasks_empty()}
          </p>
        {:else}
          <ul class="popover-list">
            {#each rows as entry (entry.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <li
                class="popover-item"
                data-testid="tasks-popover-item"
                data-task-id={entry.id}
                onclick={() => openDetail(entry)}
              >
                <div class="popover-item__row">
                  <TaskEntryRow
                    {entry}
                    onAction={(id) => handleAction(entry, id)}
                  />
                </div>
                <span class="popover-item__chevron nf" aria-hidden="true"
                  >{"\uF054"}</span
                >
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .tasks-popover {
    position: fixed;
    bottom: 28px;
    left: 8px;
    width: min(380px, 92vw);
    max-height: min(440px, 55vh);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 -4px 24px var(--overlay-shadow, rgba(0, 0, 0, 0.35));
    z-index: 1000;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .popover-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 8px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .header-title {
    font-weight: 600;
    font-size: 12px;
    color: var(--text-primary);
  }

  .header-title--detail {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .header-badge {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    padding: 0 6px;
    border-radius: 8px;
    font-size: 10px;
    min-width: 16px;
    text-align: center;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .clear-btn {
    padding: 3px 10px;
    font-size: 11px;
  }

  .btn-icon {
    font-family: inherit;
    padding: 2px 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 4px;
  }

  .btn-icon:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .popover-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .popover-body--detail {
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .empty {
    text-align: center;
    margin: 16px 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .popover-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .popover-item {
    position: relative;
    display: flex;
    align-items: stretch;
    cursor: pointer;
    border-radius: 6px;
  }

  .popover-item__row {
    flex: 1;
    min-width: 0;
  }

  .popover-item__chevron {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-secondary);
    font-size: 10px;
    opacity: 0;
    transition: opacity 0.12s;
    pointer-events: none;
  }

  .popover-item:hover .popover-item__chevron {
    opacity: 1;
  }
</style>
