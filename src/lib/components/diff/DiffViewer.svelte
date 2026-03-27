<script lang="ts">
  import type { FileDiff } from "../../types";
  import * as m from "$lib/paraglide/messages";

  let { diff, onApplyFile }: { diff: FileDiff | null; onApplyFile?: (path: string) => void } = $props();
</script>

{#if diff}
  <div class="diff-viewer">
    <div class="diff-header">
      <span class="diff-path">{diff.path}</span>
      <div class="diff-header-actions">
        {#if onApplyFile}
          <button class="apply-file-btn" onclick={() => onApplyFile(diff.path)} title="Apply this file">Apply</button>
        {/if}
        <span class="diff-stats">
          <span class="additions">+{diff.additions}</span>
          <span class="deletions">&#8722;{diff.deletions}</span>
        </span>
      </div>
    </div>
    <div class="diff-content">
      {#each diff.hunks as hunk}
        <div class="hunk-header">{hunk.header}</div>
        {#each hunk.lines as line}
          <div class="diff-line {line.content.startsWith('<<<<<<<') ? 'conflict-ours' : line.content.startsWith('=======') ? 'conflict-separator' : line.content.startsWith('>>>>>>>') ? 'conflict-theirs' : line.origin === '+' ? 'added' : line.origin === '-' ? 'removed' : 'context'}">
            <span class="line-no old">{line.old_lineno ?? ''}</span>
            <span class="line-no new">{line.new_lineno ?? ''}</span>
            <span class="line-origin">{line.origin}</span>
            <span class="line-content">{line.content}</span>
          </div>
        {/each}
      {/each}
    </div>
  </div>
{:else}
  <div class="no-diff">
    <p>{m.diff_empty()}</p>
  </div>
{/if}

<style>
  .diff-viewer { display: flex; flex-direction: column; height: 100%; overflow: hidden; }
  .diff-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }
  .diff-path { font-size: 12px; font-weight: 600; }
  .diff-header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .apply-file-btn {
    padding: 2px 8px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    font-size: 10px;
    cursor: pointer;
    transition: background 0.15s;
  }
  .apply-file-btn:hover {
    background: rgba(63, 185, 80, 0.15);
    border-color: var(--accent-green);
    color: var(--accent-green);
  }
  .diff-stats { font-size: 11px; }
  .additions { color: var(--accent-green); margin-right: 8px; }
  .deletions { color: #f85149; }
  .diff-content { flex: 1; overflow: auto; font-family: var(--font-mono); font-size: 12px; line-height: 1.5; }
  .hunk-header {
    padding: 4px 12px; background: rgba(88,166,255,0.08);
    color: var(--accent-blue); font-size: 11px; border-bottom: 1px solid var(--border);
  }
  .diff-line { display: flex; padding: 0 12px; white-space: pre; }
  .diff-line.added { background: rgba(63,185,80,0.1); }
  .diff-line.removed { background: rgba(248,81,73,0.1); }
  .line-no { width: 40px; text-align: right; color: var(--text-secondary); padding-right: 8px; user-select: none; font-size: 11px; }
  .line-origin { width: 12px; color: var(--text-secondary); user-select: none; }
  .line-content { flex: 1; }
  .diff-line.conflict-ours {
    background: rgba(88, 166, 255, 0.15);
    color: var(--accent-blue);
    font-weight: 600;
  }

  .diff-line.conflict-separator {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    font-weight: 600;
  }

  .diff-line.conflict-theirs {
    background: rgba(248, 81, 73, 0.15);
    color: #f85149;
    font-weight: 600;
  }

  .no-diff {
    display: flex; align-items: center; justify-content: center;
    height: 100%; color: var(--text-secondary); font-size: 13px;
  }
</style>
