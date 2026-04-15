<script lang="ts">
  /**
   * StagingDiffEditor — interactive hunk/line staging with per-line checkboxes.
   *
   * Renders a structured diff as a scrollable list of hunks with checkboxes
   * for granular staging, unstaging, and discarding of individual hunks or lines.
   */
  import type { FileDiff, HunkSelection } from "$lib/types";
  import { stageHunks, unstageHunks, discardHunks } from "$lib/api/tauri";
  import { refreshStatuses, refreshDiffs } from "$lib/stores/changes";
  import ConfirmDialog from "$lib/components/common/ConfirmDialog.svelte";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    diff: FileDiff;
    isStaged: boolean;
    filename: string;
    onClose?: () => void;
  }

  let { diff, isStaged, filename, onClose }: Props = $props();

  /** Track which lines are selected per hunk: Map<hunkIndex, Set<lineIndex>> */
  let selectedLines = $state(new Map<number, Set<number>>());

  /** Whether a discard confirmation dialog is open. */
  let showDiscardConfirm = $state(false);

  /** Whether an action is currently in progress. */
  let actionInProgress = $state(false);

  /** Count of hunks where ALL changed lines are selected. */
  let selectedHunkCount = $derived.by(() => {
    let count = 0;
    for (const [hunkIdx, lineSet] of selectedLines) {
      const hunk = diff.hunks[hunkIdx];
      if (!hunk) continue;
      const changedCount = hunk.lines.filter(l => l.origin !== " ").length;
      if (changedCount > 0 && lineSet.size === changedCount) {
        count++;
      }
    }
    return count;
  });

  /** Total selected lines across all hunks. */
  let selectedLineCount = $derived.by(() => {
    let count = 0;
    for (const lineSet of selectedLines.values()) {
      count += lineSet.size;
    }
    return count;
  });

  /** Returns indices of changed (non-context) lines within a hunk. */
  function getChangedLineIndices(hunkIdx: number): number[] {
    const hunk = diff.hunks[hunkIdx];
    if (!hunk) return [];
    return hunk.lines
      .map((l, i) => (l.origin !== " " ? i : -1))
      .filter(i => i >= 0);
  }

  /** Check state for a hunk: true = all selected, false = none, 'indeterminate' = partial. */
  function hunkCheckState(hunkIdx: number): boolean | "indeterminate" {
    const changed = getChangedLineIndices(hunkIdx);
    if (changed.length === 0) return false;
    const set = selectedLines.get(hunkIdx);
    if (!set || set.size === 0) return false;
    if (set.size === changed.length) return true;
    return "indeterminate";
  }

  /** Toggle all changed lines in a hunk. */
  function toggleHunk(hunkIdx: number) {
    const changed = getChangedLineIndices(hunkIdx);
    if (changed.length === 0) return;
    const next = new Map(selectedLines);
    const state = hunkCheckState(hunkIdx);
    if (state === true) {
      // Deselect all
      next.delete(hunkIdx);
    } else {
      // Select all changed lines
      next.set(hunkIdx, new Set(changed));
    }
    selectedLines = next;
  }

  /** Toggle a single line. */
  function toggleLine(hunkIdx: number, lineIdx: number) {
    const next = new Map(selectedLines);
    const set = new Set(next.get(hunkIdx) ?? []);
    if (set.has(lineIdx)) {
      set.delete(lineIdx);
    } else {
      set.add(lineIdx);
    }
    if (set.size === 0) {
      next.delete(hunkIdx);
    } else {
      next.set(hunkIdx, set);
    }
    selectedLines = next;
  }

  /** Select all changed lines across all hunks. */
  function selectAll() {
    const next = new Map<number, Set<number>>();
    for (let i = 0; i < diff.hunks.length; i++) {
      const changed = getChangedLineIndices(i);
      if (changed.length > 0) {
        next.set(i, new Set(changed));
      }
    }
    selectedLines = next;
  }

  /** Deselect everything. */
  function deselectAll() {
    selectedLines = new Map();
  }

  /** Convert selectedLines to HunkSelection[] for IPC. */
  function buildSelections(): HunkSelection[] {
    const result: HunkSelection[] = [];
    for (const [hunkIdx, lineIdxSet] of selectedLines) {
      const hunk = diff.hunks[hunkIdx];
      if (!hunk) continue;
      const changedCount = hunk.lines.filter(l => l.origin !== " ").length;
      if (lineIdxSet.size === changedCount) {
        // All changed lines selected — send entire hunk
        result.push({ hunk_index: hunkIdx, line_ranges: null });
      } else {
        // Partial — build contiguous line ranges
        const sorted = [...lineIdxSet].sort((a, b) => a - b);
        const ranges: [number, number][] = [];
        let start = sorted[0];
        let end = sorted[0];
        for (let i = 1; i < sorted.length; i++) {
          if (sorted[i] === end + 1) {
            end = sorted[i];
          } else {
            ranges.push([start, end]);
            start = sorted[i];
            end = sorted[i];
          }
        }
        ranges.push([start, end]);
        result.push({ hunk_index: hunkIdx, line_ranges: ranges });
      }
    }
    return result;
  }

  /** After a successful action, refresh store data and clear selection. */
  async function afterAction() {
    selectedLines = new Map();
    await Promise.all([refreshStatuses(), refreshDiffs()]);
  }

  async function handleStage() {
    if (actionInProgress || selectedLineCount === 0) return;
    actionInProgress = true;
    try {
      await stageHunks(filename, buildSelections());
      await afterAction();
    } finally {
      actionInProgress = false;
    }
  }

  async function handleUnstage() {
    if (actionInProgress || selectedLineCount === 0) return;
    actionInProgress = true;
    try {
      await unstageHunks(filename, buildSelections());
      await afterAction();
    } finally {
      actionInProgress = false;
    }
  }

  async function handleDiscard() {
    if (actionInProgress || selectedLineCount === 0) return;
    showDiscardConfirm = true;
  }

  async function confirmDiscard() {
    showDiscardConfirm = false;
    actionInProgress = true;
    try {
      await discardHunks(filename, buildSelections());
      await afterAction();
    } finally {
      actionInProgress = false;
    }
  }
</script>

<div class="staging-diff-editor">
  <!-- Header bar -->
  <div class="header">
    <div class="header-left">
      {#if onClose}
        <button class="close-btn" onclick={onClose} title="Close">\uDB80\uDD99</button>
      {/if}
      <span class="filename" title={filename}>{filename}</span>
      <span class="stats">
        {#if diff.additions > 0}
          <span class="stat-add">+{diff.additions}</span>
        {/if}
        {#if diff.deletions > 0}
          <span class="stat-del">-{diff.deletions}</span>
        {/if}
      </span>
    </div>
    <div class="header-right">
      {#if selectedLineCount > 0}
        <span class="selection-info">
          {m.staging_lines_selected({ count: String(selectedLineCount) })}
        </span>
      {/if}
      <button class="header-btn" onclick={selectAll} title={m.staging_select_all()}>
        {m.staging_select_all()}
      </button>
      <button
        class="header-btn"
        onclick={deselectAll}
        disabled={selectedLineCount === 0}
        title={m.staging_deselect_all()}
      >
        {m.staging_deselect_all()}
      </button>
      {#if !isStaged}
        <button
          class="action-btn stage-btn"
          onclick={handleStage}
          disabled={selectedLineCount === 0 || actionInProgress}
        >
          {m.staging_stage_selected()}
        </button>
        <button
          class="action-btn discard-btn"
          onclick={handleDiscard}
          disabled={selectedLineCount === 0 || actionInProgress}
        >
          {m.staging_discard_selected()}
        </button>
      {:else}
        <button
          class="action-btn unstage-btn"
          onclick={handleUnstage}
          disabled={selectedLineCount === 0 || actionInProgress}
        >
          {m.staging_unstage_selected()}
        </button>
      {/if}
    </div>
  </div>

  <!-- Hunk list -->
  <div class="hunk-list">
    {#each diff.hunks as hunk, hunkIdx}
      {@const checkState = hunkCheckState(hunkIdx)}
      <div class="hunk">
        <div class="hunk-header">
          <label class="hunk-checkbox-label">
            <input
              type="checkbox"
              checked={checkState === true}
              indeterminate={checkState === "indeterminate"}
              onchange={() => toggleHunk(hunkIdx)}
            />
            <span class="hunk-header-text">{hunk.header}</span>
          </label>
        </div>
        <div class="hunk-lines">
          {#each hunk.lines as line, lineIdx}
            {@const isChanged = line.origin !== " "}
            {@const isSelected = selectedLines.get(hunkIdx)?.has(lineIdx) ?? false}
            <div
              class="line"
              class:line-added={line.origin === "+"}
              class:line-removed={line.origin === "-"}
              class:line-context={line.origin === " "}
              class:line-selected={isSelected}
            >
              <div class="line-checkbox-cell">
                {#if isChanged}
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onchange={() => toggleLine(hunkIdx, lineIdx)}
                  />
                {/if}
              </div>
              <span class="line-number">{line.old_lineno ?? ""}</span>
              <span class="line-number">{line.new_lineno ?? ""}</span>
              <span
                class="origin"
                class:origin-add={line.origin === "+"}
                class:origin-remove={line.origin === "-"}
              >{line.origin}</span>
              <span class="line-content">{line.content}</span>
            </div>
          {/each}
        </div>
      </div>
    {/each}

    {#if diff.hunks.length === 0}
      <div class="empty-diff">
        <p>{m.diff_empty()}</p>
      </div>
    {/if}
  </div>
</div>

{#if showDiscardConfirm}
  <ConfirmDialog
    title={m.staging_discard_selected()}
    message={m.staging_discard_confirm()}
    destructive={true}
    onConfirm={confirmDiscard}
    onCancel={() => { showDiscardConfirm = false; }}
  />
{/if}

<style>
  .staging-diff-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
  }

  /* ── Header ─────────────────────────────────────────────── */

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    gap: 8px;
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-family: var(--font-icons);
    font-size: 14px;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    transition: color 0.15s ease;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .filename {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .stats {
    display: flex;
    gap: 4px;
    font-size: 11px;
    font-family: 'Fira Code', var(--font-mono), monospace;
  }

  .stat-add {
    color: var(--accent-green);
    background: var(--overlay-accent-green);
    padding: 1px 6px;
    border-radius: 4px;
  }

  .stat-del {
    color: var(--accent-red);
    background: var(--overlay-accent-red);
    padding: 1px 6px;
    border-radius: 4px;
  }

  .selection-info {
    font-size: 11px;
    color: var(--accent-blue);
    background: var(--overlay-accent-blue);
    padding: 2px 8px;
    border-radius: 4px;
    white-space: nowrap;
  }

  .header-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .header-btn:hover:not(:disabled) {
    background: var(--overlay-hover);
    color: var(--text-primary);
  }

  .header-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .action-btn {
    border: none;
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    white-space: nowrap;
    transition: opacity 0.15s ease, transform 0.1s ease;
  }

  .action-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .action-btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .stage-btn {
    background: var(--accent-green);
    color: #fff;
  }

  .stage-btn:hover:not(:disabled) { opacity: 0.85; }

  .unstage-btn {
    background: var(--accent-orange);
    color: #fff;
  }

  .unstage-btn:hover:not(:disabled) { opacity: 0.85; }

  .discard-btn {
    background: var(--accent-red);
    color: #fff;
  }

  .discard-btn:hover:not(:disabled) { opacity: 0.85; }

  /* ── Hunk list ──────────────────────────────────────────── */

  .hunk-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .hunk {
    border-bottom: 1px solid var(--border);
  }

  .hunk-header {
    display: flex;
    align-items: center;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .hunk-checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    min-width: 0;
  }

  .hunk-checkbox-label input[type="checkbox"] {
    margin: 0;
    accent-color: var(--accent-blue);
    flex-shrink: 0;
  }

  .hunk-header-text {
    font-size: 11px;
    font-family: 'Fira Code', var(--font-mono), monospace;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Lines ──────────────────────────────────────────────── */

  .hunk-lines {
    display: flex;
    flex-direction: column;
  }

  .line {
    display: flex;
    align-items: stretch;
    min-height: 20px;
    border-left: 3px solid transparent;
    transition: border-color 0.1s ease;
  }

  .line-added {
    background: var(--diff-added-bg, rgba(63, 185, 80, 0.1));
  }

  .line-removed {
    background: var(--diff-removed-bg, rgba(248, 81, 73, 0.1));
  }

  .line-context {
    background: transparent;
  }

  .line-selected {
    border-left-color: var(--accent-blue);
  }

  .line-selected.line-added {
    background: rgba(63, 185, 80, 0.18);
  }

  .line-selected.line-removed {
    background: rgba(248, 81, 73, 0.18);
  }

  .line-selected.line-context {
    background: var(--overlay-accent-blue);
  }

  .line-checkbox-cell {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    flex-shrink: 0;
  }

  .line-checkbox-cell input[type="checkbox"] {
    margin: 0;
    accent-color: var(--accent-blue);
  }

  .line-number {
    color: var(--text-secondary);
    font-size: 11px;
    font-family: 'Fira Code', var(--font-mono), monospace;
    min-width: 40px;
    text-align: right;
    padding: 0 6px;
    user-select: none;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    opacity: 0.6;
  }

  .origin {
    width: 16px;
    text-align: center;
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    user-select: none;
  }

  .origin-add { color: var(--accent-green); }
  .origin-remove { color: var(--accent-red); }

  .line-content {
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: 12px;
    white-space: pre;
    padding: 0 8px 0 4px;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .empty-diff {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
