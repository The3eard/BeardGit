<!--
  BlameGutter.svelte — Blame annotation panel.

  Shows per-line author, short OID, and relative date aligned with
  the code editor lines. Consecutive lines from the same commit are
  grouped visually with only the first line showing the full annotation.
-->
<script lang="ts">
  import type { BlameLine } from '$lib/types';
  import { formatRelativeTimeUnix } from '$lib/utils/time';
  import { shortOid } from '$lib/utils/git';

  interface Props {
    lines: BlameLine[];
    onOidClick?: (oid: string) => void;
  }

  let { lines, onOidClick }: Props = $props();

  /**
   * For each line decide whether it is the first line of a new commit
   * block (and thus should display the annotation).
   */
  let annotations = $derived(
    lines.map((line, i) => {
      const isFirst = i === 0 || lines[i - 1].oid !== line.oid;
      return { ...line, isFirst };
    })
  );

  /**
   * Assign alternating group indices so adjacent commit blocks get
   * different background tints.
   */
  let groupIndices = $derived(() => {
    let groupIdx = 0;
    return lines.map((line, i) => {
      if (i > 0 && lines[i - 1].oid !== line.oid) groupIdx++;
      return groupIdx;
    });
  });

  function truncateAuthor(author: string): string {
    return author.length > 12 ? author.slice(0, 11) + '\u2026' : author;
  }

  function handleOidClick(oid: string) {
    onOidClick?.(oid);
  }
</script>

<div class="blame-gutter">
  {#each annotations as ann, i}
    {@const groups = groupIndices()}
    <div
      class="gutter-line"
      class:even={groups[i] % 2 === 0}
      class:odd={groups[i] % 2 !== 0}
    >
      {#if ann.isFirst}
        <button
          class="oid-btn"
          title={ann.summary}
          onclick={() => handleOidClick(ann.oid)}
        >
          {shortOid(ann.oid)}
        </button>
        <span class="author" title={ann.author}>
          {truncateAuthor(ann.author)}
        </span>
        <span class="date">
          {formatRelativeTimeUnix(ann.timestamp)}
        </span>
      {:else}
        <span class="gutter-bar"></span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .blame-gutter {
    width: 220px;
    min-width: 220px;
    overflow-y: hidden;
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: 12px;
    line-height: 1.5;
    user-select: none;
    border-right: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .gutter-line {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    height: 18px;
    white-space: nowrap;
    overflow: hidden;
  }

  .gutter-line.even {
    background: var(--bg-secondary);
  }

  .gutter-line.odd {
    background: var(--bg-primary);
  }

  .oid-btn {
    background: none;
    border: none;
    color: var(--accent-blue);
    font-family: inherit;
    font-size: inherit;
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
  }

  .oid-btn:hover {
    text-decoration: underline;
  }

  .author {
    color: var(--text-secondary);
    flex-shrink: 0;
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .date {
    color: var(--text-secondary);
    opacity: 0.7;
    flex-shrink: 0;
    margin-left: auto;
  }

  .gutter-bar {
    display: block;
    width: 2px;
    height: 100%;
    background: var(--border);
    margin-left: 22px;
  }
</style>
