<!--
  SideBySideDiff — Two-column diff view for a single file.

  Aligns old and new lines side-by-side with synchronized scrolling.
  Context lines span both columns; additions and deletions are shown in
  their respective sides. Conflict markers (ours/theirs/base) are
  highlighted with distinct colors when present.
-->
<script lang="ts">
  import type { FileDiff, DiffHunkInfo } from "../../types";

  let { diff, onClose }: { diff: FileDiff; onClose?: () => void } = $props();

  interface SideBySideLine {
    type: "context" | "added" | "removed" | "modified";
    oldLineno: number | null;
    oldContent: string;
    newLineno: number | null;
    newContent: string;
  }

  interface SideBySideRow {
    kind: "hunk-header" | "line";
    header?: string;
    line?: SideBySideLine;
  }

  function buildRows(hunks: DiffHunkInfo[]): SideBySideRow[] {
    const rows: SideBySideRow[] = [];

    for (const hunk of hunks) {
      rows.push({ kind: "hunk-header", header: hunk.header });

      let i = 0;
      const lines = hunk.lines;
      while (i < lines.length) {
        const line = lines[i];

        if (line.origin === " ") {
          rows.push({
            kind: "line",
            line: {
              type: "context",
              oldLineno: line.old_lineno,
              oldContent: line.content,
              newLineno: line.new_lineno,
              newContent: line.content,
            },
          });
          i++;
        } else if (line.origin === "-") {
          // Collect consecutive removals
          const removed: typeof lines = [];
          while (i < lines.length && lines[i].origin === "-") {
            removed.push(lines[i]);
            i++;
          }
          // Collect consecutive additions right after
          const added: typeof lines = [];
          while (i < lines.length && lines[i].origin === "+") {
            added.push(lines[i]);
            i++;
          }
          // Pair them up
          const maxLen = Math.max(removed.length, added.length);
          for (let j = 0; j < maxLen; j++) {
            const rem = removed[j];
            const add = added[j];
            rows.push({
              kind: "line",
              line: {
                type: rem && add ? "modified" : rem ? "removed" : "added",
                oldLineno: rem?.old_lineno ?? null,
                oldContent: rem?.content ?? "",
                newLineno: add?.new_lineno ?? null,
                newContent: add?.content ?? "",
              },
            });
          }
        } else if (line.origin === "+") {
          rows.push({
            kind: "line",
            line: {
              type: "added",
              oldLineno: null,
              oldContent: "",
              newLineno: line.new_lineno,
              newContent: line.content,
            },
          });
          i++;
        } else {
          i++;
        }
      }
    }
    return rows;
  }

  interface WordDiffSegment {
    text: string;
    changed: boolean;
  }

  function computeWordDiff(
    oldText: string,
    newText: string,
  ): { oldSegs: WordDiffSegment[]; newSegs: WordDiffSegment[] } {
    const oldWords = oldText.split(/(\s+)/);
    const newWords = newText.split(/(\s+)/);

    const oldSegs: WordDiffSegment[] = [];
    const newSegs: WordDiffSegment[] = [];

    let oi = 0,
      ni = 0;
    while (oi < oldWords.length && ni < newWords.length) {
      if (oldWords[oi] === newWords[ni]) {
        oldSegs.push({ text: oldWords[oi], changed: false });
        newSegs.push({ text: newWords[ni], changed: false });
        oi++;
        ni++;
      } else {
        let foundOld = -1,
          foundNew = -1;
        for (let k = 1; k < 8; k++) {
          if (foundOld < 0 && oi + k < oldWords.length && oldWords[oi + k] === newWords[ni])
            foundOld = k;
          if (foundNew < 0 && ni + k < newWords.length && oldWords[oi] === newWords[ni + k])
            foundNew = k;
        }
        if (foundOld >= 0 && (foundNew < 0 || foundOld <= foundNew)) {
          for (let i = 0; i < foundOld; i++)
            oldSegs.push({ text: oldWords[oi + i], changed: true });
          oi += foundOld;
        } else if (foundNew >= 0) {
          for (let i = 0; i < foundNew; i++)
            newSegs.push({ text: newWords[ni + i], changed: true });
          ni += foundNew;
        } else {
          oldSegs.push({ text: oldWords[oi], changed: true });
          newSegs.push({ text: newWords[ni], changed: true });
          oi++;
          ni++;
        }
      }
    }
    while (oi < oldWords.length) oldSegs.push({ text: oldWords[oi++], changed: true });
    while (ni < newWords.length) newSegs.push({ text: newWords[ni++], changed: true });

    return { oldSegs, newSegs };
  }

  let rows = $derived(buildRows(diff.hunks));
</script>

<div class="sbs-diff">
  <div class="sbs-header">
    <span class="sbs-path">{diff.path}</span>
    <div class="sbs-header-right">
      <span class="sbs-stats">
        <span class="sbs-add">+{diff.additions}</span>
        <span class="sbs-del">-{diff.deletions}</span>
      </span>
      {#if onClose}
        <button class="sbs-close nf" onclick={onClose} title="Close">{"\uEA76"}</button>
      {/if}
    </div>
  </div>
  <div class="sbs-body">
    <table class="sbs-table">
      <colgroup>
        <col style="width: 45px" />
        <col />
        <col style="width: 45px" />
        <col />
      </colgroup>
      <tbody>
        {#each rows as row}
          {#if row.kind === "hunk-header"}
            <tr class="hunk-row">
              <td colspan="4" class="hunk-cell">{row.header}</td>
            </tr>
          {:else if row.line}
            {@const wordDiff = row.line.type === "modified" ? computeWordDiff(row.line.oldContent, row.line.newContent) : null}
            <tr class="line-row {row.line.type}">
              <td class="lineno old-lineno">{row.line.oldLineno ?? ""}</td>
              <td class="code old-code" class:empty={!row.line.oldContent && row.line.type === "added"}>
                {#if wordDiff}
                  {#each wordDiff.oldSegs as seg}<span class:word-changed={seg.changed}>{seg.text}</span>{/each}
                {:else}
                  {row.line.oldContent}
                {/if}
              </td>
              <td class="lineno new-lineno">{row.line.newLineno ?? ""}</td>
              <td class="code new-code" class:empty={!row.line.newContent && row.line.type === "removed"}>
                {#if wordDiff}
                  {#each wordDiff.newSegs as seg}<span class:word-changed={seg.changed}>{seg.text}</span>{/each}
                {:else}
                  {row.line.newContent}
                {/if}
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .sbs-diff {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    border-top: 1px solid var(--border);
  }

  .sbs-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .sbs-path {
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .sbs-header-right {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .sbs-stats {
    font-size: 11px;
  }

  .sbs-add {
    color: #3fb950;
    margin-right: 6px;
  }

  .sbs-del {
    color: #f85149;
  }

  .sbs-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 4px;
    border-radius: 3px;
  }

  .sbs-close:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }

  .sbs-body {
    flex: 1;
    overflow: auto;
  }

  .sbs-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    table-layout: fixed;
  }

  .hunk-row .hunk-cell {
    padding: 4px 12px;
    background: rgba(88, 166, 255, 0.08);
    color: var(--accent-blue);
    font-size: 11px;
    border-bottom: 1px solid var(--border);
  }

  .lineno {
    width: 45px;
    min-width: 45px;
    max-width: 45px;
    text-align: right;
    padding-right: 8px;
    color: var(--text-secondary);
    font-size: 11px;
    user-select: none;
    vertical-align: top;
    opacity: 0.6;
  }

  .code {
    padding: 0 8px;
    text-align: left;
    white-space: pre;
    vertical-align: top;
    overflow: hidden;
  }

  .old-code {
    border-right: 1px solid var(--border);
  }

  .line-row.removed .old-code,
  .line-row.modified .old-code {
    background: rgba(248, 81, 73, 0.1);
  }

  .line-row.added .new-code,
  .line-row.modified .new-code {
    background: rgba(63, 185, 80, 0.1);
  }

  .line-row.removed .new-code.empty,
  .line-row.modified .old-code:not(.empty) ~ .new-code.empty {
    background: rgba(255, 255, 255, 0.02);
  }

  .line-row.added .old-code.empty {
    background: rgba(255, 255, 255, 0.02);
  }

  .code.empty {
    background: rgba(255, 255, 255, 0.02);
  }

  .line-row.modified .old-code .word-changed {
    background: rgba(248, 81, 73, 0.4);
    border-radius: 2px;
    padding: 0 1px;
  }

  .line-row.modified .new-code .word-changed {
    background: rgba(63, 185, 80, 0.4);
    border-radius: 2px;
    padding: 0 1px;
  }
</style>
