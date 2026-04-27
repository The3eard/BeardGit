<script lang="ts">
  import type { CiJobStep } from "../../types";
  import { ciStatusColor } from "../../utils/status";

  interface Props {
    steps: CiJobStep[];
  }

  let { steps }: Props = $props();

  function statusIcon(status: string): string {
    switch (status) {
      case "success": return "\uF00C";   // nf-fa-check
      case "failed": return "\uF00D";    // nf-fa-times
      case "timed_out": return "\uF00D";
      case "running": return "\uF04B";   // nf-fa-play
      case "pending": return "\uF10C";   // nf-fa-circle_o
      case "queued": return "\uF10C";
      case "canceled": return "\uF05E";  // nf-fa-ban
      case "skipped": return "\uF051";   // nf-fa-step_forward
      default: return "\uF10C";
    }
  }

  function formatDuration(seconds: number | null): string {
    if (seconds == null) return "";
    if (seconds < 1) return "<1s";
    if (seconds < 60) return `${Math.round(seconds)}s`;
    const min = Math.floor(seconds / 60);
    const sec = Math.round(seconds % 60);
    return `${min}m ${sec}s`;
  }
</script>

<div class="job-steps">
  <div class="steps-header">Step progress</div>
  <div class="steps-list">
    {#each steps as step (step.number)}
      <div class="step-row" class:running={step.status === "running"}>
        <span
          class="step-icon nf"
          class:spinning={step.status === "running"}
          style="color: {ciStatusColor(step.status)}"
        >{statusIcon(step.status)}</span>
        <span class="step-name">{step.name}</span>
        <span class="step-duration">{formatDuration(step.duration)}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .job-steps {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .steps-header {
    padding: 8px 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
  }

  .steps-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .step-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    font-size: 12px;
    color: var(--text-primary);
  }

  .step-row.running {
    background: var(--overlay-hover);
  }

  .step-icon {
    font-size: 11px;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }

  .step-icon.spinning {
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .step-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .step-duration {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
    font-family: var(--font-mono);
  }
</style>
