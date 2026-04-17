<script lang="ts">
  /**
   * Inline status pill for an AI background run. Maps a discriminated
   * AiBackgroundRunStatus into a coloured badge with an icon + localised
   * label. Reused by AiSessionList (inline) and AiSessionDetail (header).
   */
  import type { AiBackgroundRunStatus } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    status: AiBackgroundRunStatus;
    /** When true, render the spinner-style icon for running/queued states. */
    compact?: boolean;
  }

  let { status, compact = false }: Props = $props();

  let label = $derived.by(() => {
    switch (status.state) {
      case "queued":
        return m.ai_background_status_queued();
      case "running":
        return m.ai_background_status_running();
      case "completed":
        return m.ai_background_status_completed();
      case "failed":
        return m.ai_background_status_failed();
      case "cancelled":
        return m.ai_background_status_cancelled();
    }
  });

  let klass = $derived.by(() => {
    switch (status.state) {
      case "queued":
        return "badge badge-queued";
      case "running":
        return "badge badge-running";
      case "completed":
        return "badge badge-completed";
      case "failed":
        return "badge badge-failed";
      case "cancelled":
        return "badge badge-cancelled";
    }
  });
</script>

<span class={klass} class:compact>
  {#if status.state === "running"}
    <span class="spinner" aria-hidden="true"></span>
  {/if}
  <span class="label">{label}</span>
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    line-height: 1.4;
    white-space: nowrap;
  }

  .badge.compact {
    padding: 1px 6px;
    font-size: 9px;
  }

  .badge-queued {
    background: color-mix(in srgb, var(--text-secondary) 18%, transparent);
    color: var(--text-secondary);
  }

  .badge-running {
    background: color-mix(in srgb, var(--accent-blue) 18%, transparent);
    color: var(--accent-blue);
  }

  .badge-completed {
    background: color-mix(in srgb, var(--accent-green, #2ea043) 18%, transparent);
    color: var(--accent-green, #2ea043);
  }

  .badge-failed {
    background: color-mix(in srgb, #f85149 18%, transparent);
    color: #f85149;
  }

  .badge-cancelled {
    background: color-mix(in srgb, var(--text-secondary) 12%, transparent);
    color: var(--text-secondary);
  }

  .spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
