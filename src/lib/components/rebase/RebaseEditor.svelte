<!--
  RebaseEditor — Full-screen overlay for interactive rebase.

  Displays a draggable list of commits between a base commit and HEAD.
  Each commit has an action dropdown (pick, squash, fixup, edit, drop)
  and can be reordered via HTML5 drag-and-drop. The list shows oldest
  commits at the top (same order as `git rebase -i`).
-->
<script lang="ts">
  import { getRebaseCommits, startInteractiveRebase } from "../../api/tauri";
  import type { RebaseCommit, RebaseAction } from "../../types";
  import * as m from "$lib/paraglide/messages";
  import { shortOid } from "../../utils/git";
  import Button from "$lib/components/ui/Button.svelte";

  interface Props {
    /** The base commit OID (exclusive — commits after this up to HEAD). */
    baseOid: string;
    /** Called after a successful rebase start. */
    onComplete: () => void;
    /** Called when the user cancels the editor. */
    onCancel: () => void;
  }

  let { baseOid, onComplete, onCancel }: Props = $props();

  type RebaseActionType = "pick" | "squash" | "fixup" | "edit" | "drop";

  interface RebaseEntry {
    commit: RebaseCommit;
    action: RebaseActionType;
  }

  const ACTION_OPTIONS: RebaseActionType[] = ["pick", "squash", "fixup", "edit", "drop"];

  let entries = $state<RebaseEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let submitting = $state(false);

  // Drag state
  let dragIndex = $state<number | null>(null);
  let dragOverIndex = $state<number | null>(null);

  /** Map action type to the i18n label. */
  function actionLabel(action: RebaseActionType): string {
    switch (action) {
      case "pick": return m.rebase_action_pick();
      case "squash": return m.rebase_action_squash();
      case "fixup": return m.rebase_action_fixup();
      case "edit": return m.rebase_action_edit();
      case "drop": return m.rebase_action_drop();
    }
  }

  /** Relative time string from ISO date. */
  function relativeTime(dateStr: string): string {
    const now = Date.now();
    const then = new Date(dateStr).getTime();
    const diffSec = Math.floor((now - then) / 1000);
    if (diffSec < 60) return `${diffSec}s ago`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    return `${diffDay}d ago`;
  }

  // Load commits on mount
  $effect(() => {
    const oid = baseOid;
    loading = true;
    error = null;

    getRebaseCommits(oid)
      .then((commits) => {
        entries = commits.map((c) => ({ commit: c, action: "pick" as RebaseActionType }));
        loading = false;
      })
      .catch((err) => {
        error = String(err);
        loading = false;
      });
  });

  function handleDragStart(e: DragEvent, index: number) {
    dragIndex = index;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(index));
    }
  }

  function handleDragOver(e: DragEvent, index: number) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "move";
    }
    dragOverIndex = index;
  }

  function handleDragLeave() {
    dragOverIndex = null;
  }

  function handleDrop(e: DragEvent, targetIndex: number) {
    e.preventDefault();
    if (dragIndex === null || dragIndex === targetIndex) {
      dragIndex = null;
      dragOverIndex = null;
      return;
    }

    const updated = [...entries];
    const [moved] = updated.splice(dragIndex, 1);
    updated.splice(targetIndex, 0, moved);
    entries = updated;

    dragIndex = null;
    dragOverIndex = null;
  }

  function handleDragEnd() {
    dragIndex = null;
    dragOverIndex = null;
  }

  function setAction(index: number, action: RebaseActionType) {
    entries = entries.map((e, i) => (i === index ? { ...e, action } : e));
  }

  async function handleStart() {
    submitting = true;
    try {
      const actions: RebaseAction[] = entries.map((e) => ({
        oid: e.commit.oid,
        action: e.action,
      }));
      await startInteractiveRebase(baseOid, actions);
      onComplete();
    } catch (err) {
      error = String(err);
    } finally {
      submitting = false;
    }
  }
</script>

<div class="rebase-overlay" role="dialog" aria-modal="true">
  <div class="rebase-editor">
    <div class="rebase-header">
      <h2 class="rebase-title">{m.rebase_editor_title()}</h2>
      {#if entries.length > 0}
        <span class="rebase-count">{m.rebase_commits_count({ count: entries.length })}</span>
      {/if}
    </div>

    <div class="rebase-body">
      {#if loading}
        <div class="rebase-loading">
          <div class="spinner"></div>
        </div>
      {:else if error}
        <div class="rebase-error">{error}</div>
      {:else if entries.length === 0}
        <div class="rebase-empty">No commits to rebase.</div>
      {:else}
        <div class="rebase-list">
          {#each entries as entry, i}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="rebase-row action-{entry.action}"
              class:dragging={dragIndex === i}
              class:drag-over={dragOverIndex === i}
              class:is-drop={entry.action === "drop"}
              draggable="true"
              ondragstart={(e) => handleDragStart(e, i)}
              ondragover={(e) => handleDragOver(e, i)}
              ondragleave={handleDragLeave}
              ondrop={(e) => handleDrop(e, i)}
              ondragend={handleDragEnd}
            >
              <span class="drag-handle">{"\u2630"}</span>

              <select
                class="action-select"
                value={entry.action}
                onchange={(e) => setAction(i, (e.target as HTMLSelectElement).value as RebaseActionType)}
              >
                {#each ACTION_OPTIONS as opt}
                  <option value={opt}>{actionLabel(opt)}</option>
                {/each}
              </select>

              <span class="commit-oid">{shortOid(entry.commit.oid)}</span>

              <span class="commit-message" class:strikethrough={entry.action === "drop"}>
                {entry.commit.message}
              </span>

              <span class="commit-meta">
                {entry.commit.author} · {relativeTime(entry.commit.date)}
              </span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="rebase-legend">
      <span class="legend-item"><span class="legend-dot pick"></span> pick — use commit as-is</span>
      <span class="legend-item"><span class="legend-dot squash"></span> squash — meld into previous</span>
      <span class="legend-item"><span class="legend-dot fixup"></span> fixup — meld, discard message</span>
      <span class="legend-item"><span class="legend-dot edit"></span> edit — pause to amend</span>
      <span class="legend-item"><span class="legend-dot drop"></span> drop — remove commit</span>
    </div>

    <div class="rebase-footer">
      <Button variant="neutral" onclick={onCancel} disabled={submitting}>
        {m.rebase_cancel()}
      </Button>
      <Button
        variant="primary"
        onclick={handleStart}
        disabled={submitting || entries.length === 0}
      >
        {#if submitting}
          <div class="spinner spinner--small"></div>
        {:else}
          {m.rebase_start()}
        {/if}
      </Button>
    </div>
  </div>
</div>

<style>
  .rebase-overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6); /* beardgit:allow-hex: modal backdrop neutral */
    backdrop-filter: blur(4px);
  }

  .rebase-editor {
    display: flex;
    flex-direction: column;
    width: min(700px, 90vw);
    max-height: 80vh;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4); /* beardgit:allow-hex: drop shadow neutral */
    overflow: hidden;
  }

  .rebase-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border);
  }

  .rebase-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .rebase-count {
    font-size: 11px;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    padding: 2px 8px;
    border-radius: 10px;
  }

  .rebase-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .rebase-loading,
  .rebase-empty,
  .rebase-error {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .rebase-error {
    color: var(--accent-orange);
  }

  .rebase-list {
    display: flex;
    flex-direction: column;
  }

  .rebase-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    border-left: 3px solid transparent;
    transition: background 0.1s, opacity 0.15s;
    cursor: grab;
    min-height: 36px;
  }

  .rebase-row:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .rebase-row.dragging {
    opacity: 0.3;
  }

  .rebase-row.drag-over {
    border-top: 2px solid var(--accent-blue);
  }

  /* Action-based left border colors */
  .rebase-row.action-pick { border-left-color: var(--accent-green); }
  .rebase-row.action-squash { border-left-color: var(--accent-orange); }
  .rebase-row.action-fixup { border-left-color: var(--accent-purple); }
  .rebase-row.action-edit { border-left-color: var(--accent-blue); }
  .rebase-row.action-drop { border-left-color: var(--accent-red); }

  .rebase-row.is-drop {
    opacity: 0.45;
  }

  .drag-handle {
    font-size: 14px;
    color: var(--text-secondary);
    cursor: grab;
    user-select: none;
    flex-shrink: 0;
    width: 16px;
    text-align: center;
  }

  .action-select {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 11px;
    padding: 2px 4px;
    cursor: pointer;
    flex-shrink: 0;
    width: 72px;
  }

  .commit-oid {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-purple);
    flex-shrink: 0;
    width: 56px;
  }

  .commit-message {
    flex: 1;
    font-size: 12px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .commit-message.strikethrough {
    text-decoration: line-through;
    color: var(--text-secondary);
  }

  .commit-meta {
    font-size: 11px;
    color: var(--text-secondary);
    flex-shrink: 0;
    white-space: nowrap;
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rebase-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 8px 20px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .legend-dot.pick { background: var(--accent-green); }
  .legend-dot.squash { background: var(--accent-orange); }
  .legend-dot.fixup { background: var(--accent-purple); }
  .legend-dot.edit { background: var(--accent-blue); }
  .legend-dot.drop { background: var(--accent-red); }

  .rebase-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
  }

</style>
