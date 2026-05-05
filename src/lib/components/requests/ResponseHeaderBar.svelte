<!--
  Status bar shown at the top of the Response viewer. Reflects the
  current run lifecycle: a running pill while a request is in flight,
  the error message when the last run failed, and the status code +
  duration + body size + truncation flag for a successful response.

  Pill colours reuse the `--overlay-accent-*` token family that the
  rest of the app's status badges (issue states, MR states, pipeline
  status) draw from, so the pill vocabulary stays consistent.
-->
<script lang="ts">
  import { lastResponse, lastResponseError, runState } from "./stores";

  /** Pick a colour register based on the HTTP status code. */
  function statusKind(s: number): "ok" | "warn" | "err" {
    if (s >= 500) return "err";
    if (s >= 400) return "err";
    if (s >= 300) return "warn";
    return "ok";
  }
</script>

<div class="status">
  {#if $runState === "running"}
    <span class="pill pill--running">Running…</span>
  {:else if $lastResponseError}
    <span class="pill pill--err">Error</span>
    <span class="msg">{$lastResponseError}</span>
  {:else if $lastResponse}
    {@const kind = statusKind($lastResponse.status)}
    <span class="pill pill--{kind}">{$lastResponse.status}</span>
    <span class="meta">
      {$lastResponse.durationMs} ms · {$lastResponse.body.byteLength} B
    </span>
    {#if $lastResponse.truncated}
      <span class="warn">truncated to 5 MB</span>
    {/if}
  {:else}
    <span class="pill pill--idle">No response yet</span>
  {/if}
</div>

<style>
  .status {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }

  .pill {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    letter-spacing: 0.2px;
  }

  .pill--ok {
    background: var(--overlay-accent-green);
    color: var(--accent-green);
  }
  .pill--warn {
    background: var(--overlay-accent-orange);
    color: var(--accent-orange);
  }
  .pill--err {
    background: var(--overlay-accent-red);
    color: var(--accent-red);
  }
  .pill--running {
    background: var(--overlay-accent-blue);
    color: var(--accent-blue);
  }
  .pill--idle {
    background: var(--overlay-accent-muted);
    color: var(--text-secondary);
    font-family: inherit;
    font-weight: 500;
  }

  .meta,
  .msg {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .warn {
    color: var(--accent-orange);
    font-size: 11px;
  }
</style>
