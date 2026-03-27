<!--
  ConflictToolbar — Banner displayed during merge/rebase/cherry-pick/revert conflicts.

  Shows the current operation state, lists conflicted files, and provides
  Abort and Continue buttons. Continue is disabled until all conflicts are
  resolved. Automatically hidden when no operation is in progress.
-->
<script lang="ts">
  import {
    conflictStatus,
    isInConflict,
    abortOperation,
    continueOperation,
  } from "../../stores/conflict";
  import * as m from "$lib/paraglide/messages";

  function getAbortLabel(state: string): string {
    switch (state) {
      case "merging":
        return m.conflict_abort_merge();
      case "rebasing":
        return m.conflict_abort_rebase();
      case "cherry_picking":
        return m.conflict_abort_cherry_pick();
      case "reverting":
        return m.conflict_abort_revert();
      default:
        return "";
    }
  }

  function getContinueLabel(state: string): string {
    switch (state) {
      case "merging":
        return m.conflict_continue_merge();
      case "rebasing":
        return m.conflict_continue_rebase();
      case "cherry_picking":
        return m.conflict_continue_cherry_pick();
      case "reverting":
        return m.conflict_continue_revert();
      default:
        return "";
    }
  }

  function getStateLabel(state: string): string {
    switch (state) {
      case "merging":
        return m.conflict_state_merging();
      case "rebasing":
        return m.conflict_state_rebasing();
      case "cherry_picking":
        return m.conflict_state_cherry_picking();
      case "reverting":
        return m.conflict_state_reverting();
      default:
        return "";
    }
  }

  let fileCountText = $derived(
    $conflictStatus.conflicted_files.length === 0
      ? m.conflict_resolved_all()
      : $conflictStatus.conflicted_files.length === 1
        ? m.conflict_toolbar_files_one({ count: 1 })
        : m.conflict_toolbar_files({ count: $conflictStatus.conflicted_files.length }),
  );
</script>

{#if $isInConflict}
  <div class="conflict-toolbar">
    <div class="conflict-left">
      <span class="conflict-icon">{"\uF071"}</span>
      <span class="conflict-state">{getStateLabel($conflictStatus.state)}</span>
      <span class="conflict-separator">—</span>
      <span class="conflict-files" class:resolved={$conflictStatus.can_continue}>
        {fileCountText}
      </span>
    </div>
    <div class="conflict-right">
      <button class="btn btn-abort" onclick={abortOperation}>
        {getAbortLabel($conflictStatus.state)}
      </button>
      <button
        class="btn btn-continue"
        disabled={!$conflictStatus.can_continue}
        title={$conflictStatus.can_continue ? "" : m.conflict_continue_disabled_tooltip()}
        onclick={continueOperation}
      >
        {getContinueLabel($conflictStatus.state)}
      </button>
    </div>
  </div>
{/if}

<style>
  .conflict-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background: rgba(210, 153, 34, 0.1);
    border-bottom: 1px solid rgba(210, 153, 34, 0.3);
    flex-shrink: 0;
  }

  .conflict-left {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .conflict-icon {
    font-family: var(--font-icons);
    color: #d29922;
    font-size: 14px;
  }

  .conflict-state {
    font-weight: 700;
    color: #d29922;
    font-size: 11px;
    letter-spacing: 0.5px;
  }

  .conflict-separator {
    color: var(--text-secondary);
  }

  .conflict-files {
    color: var(--text-primary);
  }

  .conflict-files.resolved {
    color: var(--accent-green);
  }

  .conflict-right {
    display: flex;
    gap: 8px;
  }

  .btn {
    padding: 4px 12px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    border: 1px solid var(--border);
    transition: background 0.15s;
  }

  .btn-abort {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .btn-abort:hover {
    background: rgba(248, 81, 73, 0.15);
    border-color: #f85149;
    color: #f85149;
  }

  .btn-continue {
    background: var(--accent-blue);
    color: #fff;
    border-color: var(--accent-blue);
  }

  .btn-continue:hover {
    opacity: 0.9;
  }

  .btn-continue:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
