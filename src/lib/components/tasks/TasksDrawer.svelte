<!--
  TasksDrawer — the unified tasks surface for BeardGit.

  Slides up from the bottom of the viewport (max 50% height, internal
  scroll on overflow) and renders every in-flight or recently-finished
  task emitted by the aggregator store. Three sources feed the list:
  Rust `task://update` events, headless AI runs, and the auto-update
  lifecycle — all flattened into `TaskEntry` rows.

  Layout:
    Header — title + "Clear finished" action + X close.
    Body   — Active section (running) + Recently finished (≤ 5 min).
    Footer — "View log" link opening the system log directory.

  Interaction:
    - `Esc` closes the drawer.
    - Clicking the backdrop closes the drawer.
    - Opening the drawer marks unseen errors as seen (clears the red dot).

  Keyboard:
    - `Esc` closes the drawer.
    - `↑` / `↓` move focus between rows (active first, then recently-
      finished), with wraparound.
    - `Enter` on the focused row invokes that row's primary (first)
      action. For rows without actions it's a no-op.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { openLogDirectory } from "$lib/api/tauri";
  import * as m from "$lib/paraglide/messages";
  import {
    tasksStore,
    activeTaskCount,
    recentlyFinishedTasks,
    hasUnseenError,
    markSeen,
    clearFinished,
    cancelTaskById,
  } from "$lib/stores/tasks";
  import type { TaskEntry, TaskAction } from "$lib/types/tasks";
  import TaskEntryRow from "./TaskEntryRow.svelte";

  interface Props {
    /** Whether the drawer is open. Parent-controlled and bindable. */
    open: boolean;
    /** Called when the drawer should close (Esc, backdrop, X). */
    onClose: () => void;
  }

  let { open = $bindable(false), onClose }: Props = $props();

  /** Active entries = running only, sorted newest-first. */
  const active = $derived(
    $tasksStore
      .filter((t) => t.status === "running")
      .slice()
      .sort((a, b) => b.startedAt - a.startedAt),
  );

  /** Recently finished = terminal-state entries within the 5 min window. */
  const finished = $derived(
    $recentlyFinishedTasks
      .slice()
      .sort((a, b) => (b.finishedAt ?? 0) - (a.finishedAt ?? 0)),
  );

  /** Flattened navigation order: active rows followed by finished rows. */
  const navOrder = $derived([...active, ...finished]);

  /**
   * Currently-focused entry index inside `navOrder`, or `-1` when no
   * row has claimed keyboard focus yet. Clamped back into range
   * whenever the list shrinks (task finished, cleared, etc.).
   */
  let focusedIndex = $state(-1);

  $effect(() => {
    if (navOrder.length === 0) {
      focusedIndex = -1;
    } else if (focusedIndex >= navOrder.length) {
      focusedIndex = navOrder.length - 1;
    }
  });

  // Mark unseen errors as seen whenever the drawer transitions to open —
  // clears the red dot on the statusbar tasks slot.
  $effect(() => {
    if (open) markSeen();
  });

  /** Route a row action back to the store or drawer-local handlers. */
  async function handleAction(entry: TaskEntry, id: TaskAction["id"]) {
    if (id === "cancel") {
      await cancelTaskById(entry.id);
    } else if (id === "dismiss") {
      // Dismissing a finished entry removes it from the in-memory map.
      // For running entries the UI hides the dismiss button so this
      // branch is only reachable from a terminal row.
      clearFinished();
    }
    // retry / open_output are wired up by kind-specific consumers in a
    // follow-up slice (retry needs per-producer hooks; open_output
    // needs an AI session viewer route).
  }

  /**
   * Move keyboard focus inside the drawer. `delta` of `+1` goes down,
   * `-1` goes up; both wrap. Initializes to the first row when focus
   * has not been claimed yet.
   */
  function moveFocus(delta: 1 | -1) {
    if (navOrder.length === 0) {
      focusedIndex = -1;
      return;
    }
    if (focusedIndex < 0) {
      focusedIndex = delta > 0 ? 0 : navOrder.length - 1;
      return;
    }
    const next = (focusedIndex + delta + navOrder.length) % navOrder.length;
    focusedIndex = next;
  }

  /** Invoke the primary action of the focused row, if any. */
  function invokePrimaryOnFocused() {
    const entry = navOrder[focusedIndex];
    if (!entry || entry.actions.length === 0) return;
    void handleAction(entry, entry.actions[0].id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return;
    switch (e.key) {
      case "Escape":
        e.preventDefault();
        onClose();
        return;
      case "ArrowDown":
        e.preventDefault();
        moveFocus(1);
        return;
      case "ArrowUp":
        e.preventDefault();
        moveFocus(-1);
        return;
      case "Enter":
        if (focusedIndex >= 0) {
          e.preventDefault();
          invokePrimaryOnFocused();
        }
        return;
    }
  }

  /**
   * Stable data-testid suffix for keyboard-navigation tests: open the
   * log directory via the existing Tauri command. Failures are swallowed
   * — the user can always open the directory manually if this fails.
   */
  async function openLog() {
    try {
      await openLogDirectory();
    } catch {
      // Non-fatal: the footer link is a convenience, not a critical path.
    }
  }

  onMount(() => {
    // No-op: the $effect above handles the initial markSeen when open.
    // Kept as a hook so future wiring (e.g. analytics) has a home.
  });

  void activeTaskCount;
  void hasUnseenError;
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="drawer-backdrop"
    data-testid="tasks-drawer-backdrop"
    onclick={onClose}
    onkeydown={handleKeydown}
    role="presentation"
  ></div>

  <div
    class="tasks-drawer"
    role="dialog"
    aria-modal="true"
    aria-label={m.tasks_title()}
    data-testid="tasks-drawer"
  >
    <header class="tasks-drawer__header">
      <h2 class="tasks-drawer__title" data-testid="tasks-drawer-title">
        {m.tasks_title()}
      </h2>
      <div class="tasks-drawer__header-actions">
        <button
          type="button"
          class="btn btn-cancel tasks-drawer__clear"
          data-testid="tasks-drawer-clear"
          onclick={clearFinished}
          disabled={finished.length === 0}
        >
          {m.tasks_clear_finished()}
        </button>
        <button
          type="button"
          class="btn-icon tasks-drawer__close"
          aria-label={m.tasks_close()}
          data-testid="tasks-drawer-close"
          onclick={onClose}
        >
          ×
        </button>
      </div>
    </header>

    <div class="tasks-drawer__body" data-testid="tasks-drawer-body">
      {#if active.length === 0 && finished.length === 0}
        <p class="tasks-drawer__empty" data-testid="tasks-drawer-empty">
          {m.tasks_empty()}
        </p>
      {/if}

      {#if active.length > 0}
        <section
          class="tasks-drawer__section"
          data-testid="tasks-drawer-active-section"
        >
          <h3 class="tasks-drawer__section-title">
            {m.tasks_active_heading()}
          </h3>
          <ul class="tasks-drawer__list">
            {#each active as entry, i (entry.id)}
              <li
                class="tasks-drawer__list-item"
                data-testid="tasks-drawer-row-wrapper"
                data-focused={focusedIndex === i ? "true" : "false"}
                data-nav-index={i}
              >
                <TaskEntryRow
                  {entry}
                  onAction={(id) => handleAction(entry, id)}
                />
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if finished.length > 0}
        <section
          class="tasks-drawer__section"
          data-testid="tasks-drawer-finished-section"
        >
          <h3 class="tasks-drawer__section-title">
            {m.tasks_finished_heading()}
          </h3>
          <ul class="tasks-drawer__list">
            {#each finished as entry, i (entry.id)}
              <li
                class="tasks-drawer__list-item"
                data-testid="tasks-drawer-row-wrapper"
                data-focused={focusedIndex === active.length + i
                  ? "true"
                  : "false"}
                data-nav-index={active.length + i}
              >
                <TaskEntryRow
                  {entry}
                  onAction={(id) => handleAction(entry, id)}
                />
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>

    <footer class="tasks-drawer__footer">
      <button
        type="button"
        class="tasks-drawer__log-link"
        data-testid="tasks-drawer-view-log"
        onclick={openLog}
      >
        {m.tasks_view_log()}
      </button>
    </footer>
  </div>
{/if}

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    z-index: 900;
  }

  .tasks-drawer {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    max-height: 50vh;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    box-shadow: 0 -8px 24px rgba(0, 0, 0, 0.35);
    z-index: 901;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tasks-drawer__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    flex: 0 0 auto;
  }

  .tasks-drawer__title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .tasks-drawer__header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tasks-drawer__clear {
    padding: 3px 10px;
    font-size: 11px;
  }

  .tasks-drawer__body {
    overflow-y: auto;
    padding: 8px 16px;
    flex: 1 1 auto;
  }

  .tasks-drawer__section {
    margin-bottom: 12px;
  }

  .tasks-drawer__section-title {
    margin: 0 0 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .tasks-drawer__list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tasks-drawer__list-item {
    border-radius: 6px;
    transition: box-shadow 0.12s ease;
  }

  .tasks-drawer__list-item[data-focused="true"] {
    box-shadow: 0 0 0 2px var(--accent-blue);
  }

  .tasks-drawer__empty {
    margin: 16px 0;
    text-align: center;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .tasks-drawer__footer {
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    flex: 0 0 auto;
    display: flex;
    justify-content: flex-end;
  }

  .tasks-drawer__log-link {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    text-decoration: underline;
  }

  .tasks-drawer__log-link:hover {
    color: var(--text-primary);
  }
</style>
