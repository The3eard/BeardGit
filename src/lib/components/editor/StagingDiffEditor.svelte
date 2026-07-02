<script lang="ts">
  /**
   * StagingDiffEditor — interactive hunk/line staging with per-line checkboxes.
   *
   * Renders a structured diff as a scrollable list of hunks with checkboxes
   * for granular staging, unstaging, and discarding of individual hunks or lines.
   */
  import type { FileDiff, HunkSelection } from "$lib/types";
  import { stageHunks, unstageHunks, discardHunks } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import ConfirmDialog from "$lib/components/common/ConfirmDialog.svelte";
  import EmptyState from "$lib/components/common/EmptyState.svelte";
  import { Button, Checkbox, IconButton } from "$lib/components/ui";
  import { diffLineWrapping } from "$lib/stores/diffSettings";
  import { loadLineParser, highlightLineHtml } from "./line-highlight";
  import type { Parser } from "@lezer/common";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    diff: FileDiff;
    isStaged: boolean;
    filename: string;
    onClose?: () => void;
  }

  let { diff, isStaged, filename, onClose }: Props = $props();

  /** Lezer parser for per-line syntax highlighting (null = plain text). */
  let lineParser = $state<Parser | null>(null);
  $effect(() => {
    const file = filename;
    let cancelled = false;
    loadLineParser(file).then((p) => {
      if (!cancelled) lineParser = p;
    });
    return () => {
      cancelled = true;
    };
  });

  /** Track which lines are selected per hunk: Map<hunkIndex, Set<lineIndex>> */
  let selectedLines = $state(new Map<number, Set<number>>());

  // Reset the selection whenever the diff identity changes. The selection is
  // keyed by positional indices into diff.hunks[i].lines[j]; the page
  // re-resolves `diff` from the stores after a mutation WITHOUT remounting
  // this component, so a stale selection would stage/discard the wrong lines.
  $effect(() => {
    void diff;
    selectedLines = new Map();
  });

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

  /** After a successful action, clear selection. Refresh is event-driven. */
  function afterAction() {
    selectedLines = new Map();
  }

  async function handleStage() {
    if (actionInProgress || selectedLineCount === 0) return;
    actionInProgress = true;
    try {
      await runMutation({
        kind: "stage",
        invoke: () => stageHunks(filename, buildSelections()),
        failureToastPrefix: "Stage failed",
      });
      afterAction();
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      actionInProgress = false;
    }
  }

  async function handleUnstage() {
    if (actionInProgress || selectedLineCount === 0) return;
    actionInProgress = true;
    try {
      await runMutation({
        kind: "unstage",
        invoke: () => unstageHunks(filename, buildSelections()),
        failureToastPrefix: "Unstage failed",
      });
      afterAction();
    } catch {
      // runMutation already surfaced the toast.
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
      await runMutation({
        kind: "discard",
        invoke: () => discardHunks(filename, buildSelections()),
        failureToastPrefix: "Discard failed",
      });
      afterAction();
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      actionInProgress = false;
    }
  }
</script>

<div class="staging-diff-editor" class:wrap={$diffLineWrapping}>
  <!-- Header bar -->
  <div class="header">
    <div class="header-left">
      {#if onClose}
        <IconButton icon={"\uF00D"} description={m.tooltip_close()} onclick={onClose} />
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
      <Button variant="neutral" size="sm" onclick={selectAll} description={m.staging_select_all()}>
        {m.staging_select_all()}
      </Button>
      <Button
        variant="neutral"
        size="sm"
        onclick={deselectAll}
        disabled={selectedLineCount === 0}
        description={m.staging_deselect_all()}
      >
        {m.staging_deselect_all()}
      </Button>
      {#if !isStaged}
        <Button
          variant="primary"
          size="sm"
          onclick={handleStage}
          disabled={selectedLineCount === 0 || actionInProgress}
        >
          {m.staging_stage_selected()}
        </Button>
        <Button
          variant="danger"
          size="sm"
          onclick={handleDiscard}
          disabled={selectedLineCount === 0 || actionInProgress}
        >
          {m.staging_discard_selected()}
        </Button>
      {:else}
        <Button
          variant="neutral"
          size="sm"
          onclick={handleUnstage}
          disabled={selectedLineCount === 0 || actionInProgress}
        >
          {m.staging_unstage_selected()}
        </Button>
      {/if}
    </div>
  </div>

  <!-- Hunk list -->
  <div class="hunk-list">
    {#each diff.hunks as hunk, hunkIdx}
      {@const checkState = hunkCheckState(hunkIdx)}
      <div class="hunk">
        <div class="hunk-header">
          <span class="hunk-checkbox-label">
            <Checkbox
              id="hunk-toggle-{isStaged ? 'staged' : 'unstaged'}-{hunkIdx}"
              checked={checkState === true}
              indeterminate={checkState === "indeterminate"}
              onchange={() => toggleHunk(hunkIdx)}
            />
            <label
              class="hunk-header-text"
              for="hunk-toggle-{isStaged ? 'staged' : 'unstaged'}-{hunkIdx}"
            >{hunk.header}</label>
          </span>
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
                  <Checkbox
                    checked={isSelected}
                    ariaLabel={line.content}
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
              <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlightLineHtml escapes all source text -->
              <span class="line-content">{@html highlightLineHtml(lineParser, line.content)}</span>
            </div>
          {/each}
        </div>
      </div>
    {/each}

    {#if diff.binary}
      <EmptyState fill icon={"\uF440"} title={m.diff_binary_file()} />
    {:else if diff.truncated}
      <EmptyState fill icon={"\uF440"} title={m.diff_too_large()} />
    {:else if diff.hunks.length === 0}
      <EmptyState fill icon={"\uF440"} title={m.diff_empty()} />
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

  .filename {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .stats {
    display: flex;
    gap: 4px;
    font-size: var(--font-size-xs);
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
    font-size: var(--font-size-xs);
    color: var(--accent-primary);
    background: var(--overlay-accent-blue);
    padding: 2px 8px;
    border-radius: 4px;
    white-space: nowrap;
  }

  /* ── Hunk list ──────────────────────────────────────────── */

  .hunk-list {
    flex: 1;
    overflow-y: auto;
    /* Default: horizontal scroll. The wrap variant below switches it
       off so wrapping does the work instead. */
    overflow-x: auto;
  }

  .staging-diff-editor.wrap .hunk-list {
    overflow-x: hidden;
  }

  /* Scroll mode: rows extend beyond the viewport so the parent can
     scroll horizontally. `width: max-content` lets each row be exactly
     as wide as its longest line; `min-width: 100%` keeps short rows
     spanning the viewport (so backgrounds/borders look right when the
     content is shorter than the visible area). The intermediate
     containers (`hunk`, `hunk-lines`) get the same treatment so their
     `border-bottom` / background span the full scroll width rather
     than clipping at the original viewport edge. */
  .staging-diff-editor:not(.wrap) .hunk,
  .staging-diff-editor:not(.wrap) .hunk-header,
  .staging-diff-editor:not(.wrap) .hunk-lines,
  .staging-diff-editor:not(.wrap) .line {
    width: max-content;
    min-width: 100%;
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

  .hunk-header-text {
    cursor: pointer;
    font-size: var(--font-size-xs);
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

  /* Same added/removed backgrounds as the CodeMirror diff views — both
     renderers read the theme's [editor] palette via these tokens. */
  .line-added {
    background: var(--diff-added-bg);
  }

  .line-removed {
    background: var(--diff-removed-bg);
  }

  .line-context {
    background: transparent;
  }

  .line-selected {
    border-left-color: var(--accent-primary);
  }

  .line-selected.line-added {
    background: color-mix(in srgb, var(--accent-green) 18%, transparent);
  }

  .line-selected.line-removed {
    background: color-mix(in srgb, var(--accent-red) 18%, transparent);
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

  .line-number {
    color: var(--text-secondary);
    font-size: var(--font-size-xs);
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
    font-size: var(--font-size-sm);
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
    font-size: var(--font-size-sm);
    white-space: pre;
    padding: 0 8px 0 4px;
    color: var(--text-primary);
    /* Block, NOT flex: highlighted lines are a list of inline spans and
       a flex container would swallow the whitespace text nodes between
       them. Vertical centring comes from the parent .line row. */
    display: block;
    min-width: 0;
  }

  /* Wrap mode: let long lines break across multiple visual rows so the
     whole content stays on screen without horizontal scroll. The line
     numbers + origin column align to the top so they sit next to the
     first wrapped line rather than centring on the wrapped block. */
  .staging-diff-editor.wrap .line {
    align-items: flex-start;
  }

  .staging-diff-editor.wrap .line-content {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    align-items: flex-start;
    padding-top: 2px;
    padding-bottom: 2px;
    /* Take all remaining row width so wrapping has a stable target
       width (otherwise `flex: 0 1 auto` would only let the cell shrink,
       not grow, and short context lines would leave the wrap target
       cramped). */
    flex: 1;
  }

  .staging-diff-editor.wrap .line-number,
  .staging-diff-editor.wrap .origin {
    align-items: flex-start;
    padding-top: 2px;
  }

</style>
