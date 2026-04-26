<script lang="ts">
  /**
   * Scrollable transcript panel for an AI background run.
   *
   * Shows all captured output lines (after stripping ANSI escapes). Auto
   * scrolls to the bottom when new lines arrive unless the user has
   * scrolled up manually. Includes a copy-all button.
   *
   * Lines that parse as JSON are pretty-printed with 2-space indent —
   * Claude Code's `--output-format stream-json` emits one event per
   * line as a dense single-line object (assistant text, tool inputs,
   * hook payloads…), and reading that as a literal wall of escaped
   * commas is essentially impossible. Non-JSON lines (errors, plain
   * text from Codex / OpenCode) pass through unchanged.
   */
  import { onMount } from "svelte";
  import { stripAnsi } from "$lib/utils/strip-ansi";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    lines: string[];
  }

  let { lines }: Props = $props();

  /**
   * Best-effort pretty-print: if the trimmed line parses as JSON,
   * reserialise with `JSON.stringify(_, null, 2)` so nested objects
   * land on their own lines. Otherwise return the text as-is. We test
   * the first character before calling `JSON.parse` so a bare word
   * like `"hello world"` (technically valid JSON) doesn't get quoted
   * and re-rendered — only object / array lines are treated as
   * stream-json events worth expanding.
   */
  function prettyPrintLine(text: string): string {
    const trimmed = text.trim();
    if (!trimmed) return text;
    const first = trimmed[0];
    if (first !== "{" && first !== "[") return text;
    try {
      const parsed = JSON.parse(trimmed);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return text;
    }
  }

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
    // Match what the user sees on screen — pretty-printed JSON for
    // stream-json events, plain text for everything else. Saves a
    // round-trip through `jq` when sharing a transcript snippet.
    const text = plainLines.join("\n\n");
    try {
      await navigator.clipboard.writeText(text);
      justCopied = true;
      setTimeout(() => (justCopied = false), 1400);
    } catch {
      /* clipboard API may be unavailable in some webviews */
    }
  }

  let plainLines = $derived(lines.map((l) => prettyPrintLine(stripAnsi(l))));

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
    /* Pretty-printed JSON spans multiple rendered rows; a small
       trailing margin separates events so the user can tell where
       one ends and the next begins. Single-line plain output stays
       compact because the margin is collapsing-friendly. */
    margin-bottom: 6px;
  }

  .line:last-child {
    margin-bottom: 0;
  }

  .empty {
    color: var(--text-secondary);
    font-style: italic;
    font-family: inherit;
  }
</style>
