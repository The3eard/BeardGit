<script lang="ts">
  /**
   * Scrollable transcript panel for an AI background run.
   *
   * Shows all captured output lines (after stripping ANSI escapes). Auto
   * scrolls to the bottom when new lines arrive unless the user has
   * scrolled up manually. Includes a copy-all button.
   */
  import { onMount } from "svelte";
  import { stripAnsi } from "$lib/utils/strip-ansi";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    lines: string[];
  }

  let { lines }: Props = $props();

  let container: HTMLDivElement | undefined = $state();
  let userScrolledUp = $state(false);
  let justCopied = $state(false);

  function onScroll() {
    if (!container) return;
    const distanceFromBottom =
      container.scrollHeight - container.scrollTop - container.clientHeight;
    userScrolledUp = distanceFromBottom > 24;
  }

  $effect(() => {
    void lines.length;
    if (!container || userScrolledUp) return;
    queueMicrotask(() => {
      if (container) container.scrollTop = container.scrollHeight;
    });
  });

  async function handleCopy() {
    const text = lines.map((l) => stripAnsi(l)).join("\n");
    try {
      await navigator.clipboard.writeText(text);
      justCopied = true;
      setTimeout(() => (justCopied = false), 1400);
    } catch {
      /* clipboard API may be unavailable in some webviews */
    }
  }

  let plainLines = $derived(lines.map(stripAnsi));

  onMount(() => {
    if (container) container.scrollTop = container.scrollHeight;
  });
</script>

<div class="wrap">
  <div class="actions">
    <button class="btn-copy" onclick={handleCopy} disabled={lines.length === 0}>
      {justCopied ? "✓" : m.ai_background_transcript_copy()}
    </button>
  </div>
  <div
    class="transcript"
    bind:this={container}
    onscroll={onScroll}
    role="log"
    aria-live="polite"
  >
    {#if plainLines.length === 0}
      <div class="empty">{m.ai_background_transcript_empty()}</div>
    {:else}
      {#each plainLines as line, i (i)}
        <div class="line">{line}</div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    padding: 4px 6px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .btn-copy {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn-copy:hover:not(:disabled) {
    color: var(--text-primary);
    border-color: var(--accent-blue);
  }

  .btn-copy:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .transcript {
    flex: 1;
    overflow-y: auto;
    padding: 8px 10px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .line {
    min-height: 1.5em;
  }

  .empty {
    color: var(--text-secondary);
    font-style: italic;
    font-family: inherit;
  }
</style>
