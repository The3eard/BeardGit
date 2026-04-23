<script lang="ts">
  import { selectedCiRun, loadingDetail, loadJobLog, retryCiRun, retryCiFailedJobs, cancelCiRun, retryCiJob } from "../../stores/provider";
  import type { CiJob } from "../../types";
  import * as m from "$lib/paraglide/messages";
  import { ciStatusColor } from "../../utils/status";

  let { onSelectJob }: { onSelectJob?: (jobId: number) => void } = $props();

  let selectedJobId = $state<number | null>(null);
  let loadingJobId = $state<number | null>(null);
  let busy = $state(false);
  let actionError = $state<string | null>(null);

  async function doRetry() {
    if (!$selectedCiRun || busy) return;
    busy = true; actionError = null;
    try { await retryCiRun($selectedCiRun.run.id); }
    catch (e) { actionError = m.pipeline_retry_error({ error: String(e) }); }
    finally { busy = false; }
  }
  async function doRetryFailed() {
    if (!$selectedCiRun || busy) return;
    busy = true; actionError = null;
    try { await retryCiFailedJobs($selectedCiRun.run.id); }
    catch (e) { actionError = m.pipeline_retry_error({ error: String(e) }); }
    finally { busy = false; }
  }
  async function doCancel() {
    if (!$selectedCiRun || busy) return;
    busy = true; actionError = null;
    try { await cancelCiRun($selectedCiRun.run.id); }
    catch (e) { actionError = m.pipeline_cancel_error({ error: String(e) }); }
    finally { busy = false; }
  }
  async function doRetryJob(jobId: number) {
    if (busy) return;
    busy = true; actionError = null;
    try { await retryCiJob(jobId); }
    catch (e) { actionError = m.pipeline_retry_error({ error: String(e) }); }
    finally { busy = false; }
  }

  let runStatus = $derived($selectedCiRun?.run.status ?? "");
  let isActive = $derived(
    runStatus === "running" || runStatus === "pending" || runStatus === "queued"
  );
  let hasFailedJob = $derived(
    $selectedCiRun?.stages.some(s =>
      s.jobs.some(j => j.status === "failed" || j.status === "timed_out")
    ) ?? false
  );
  let isCompleted = $derived(
    runStatus === "success" || runStatus === "failed" ||
    runStatus === "canceled" || runStatus === "timed_out"
  );

  function statusIcon(status: string): string {
    switch (status) {
      case "success": return "\uF00C";   // nf-fa-check
      case "failed": return "\uF00D";    // nf-fa-times
      case "timed_out": return "\uF00D"; // nf-fa-times
      case "running": return "\uF04B";   // nf-fa-play
      case "pending": return "\uF04D";   // nf-fa-stop (hollow look via color)
      case "queued": return "\uF017";    // nf-fa-clock_o
      case "canceled": return "\uF05E";  // nf-fa-ban
      case "skipped": return "\uF051";   // nf-fa-step_forward
      case "manual": return "\uF144";    // nf-fa-play_circle
      default: return "\uF017";          // nf-fa-clock_o
    }
  }

  function formatDuration(seconds: number | null): string {
    if (seconds == null) return "";
    if (seconds < 60) return `${seconds}s`;
    const min = Math.floor(seconds / 60);
    const sec = seconds % 60;
    if (min < 60) return `${min}m ${sec}s`;
    const hr = Math.floor(min / 60);
    const remMin = min % 60;
    return `${hr}h ${remMin}m`;
  }

  async function handleJobClick(job: CiJob) {
    selectedJobId = job.id;
    loadingJobId = job.id;
    onSelectJob?.(job.id);
    try {
      await loadJobLog(job.id, job.status);
    } finally {
      loadingJobId = null;
    }
  }
</script>

<div class="pipeline-detail">
  {#if $loadingDetail}
    <div class="detail-loading">
      <div class="spinner"></div>
      <span>{m.pipeline_loading_detail()}</span>
    </div>
  {:else if $selectedCiRun}
    <div class="detail-header">
      <div class="detail-title">
        <span
          class="detail-status"
          style="color: {ciStatusColor($selectedCiRun.run.status)}"
        >
          {statusIcon($selectedCiRun.run.status)}
        </span>
        <span>{m.pipeline_run_title({ id: String($selectedCiRun.run.display_id) })}</span>
      </div>
      <div class="detail-meta">
        <span class="meta-ref">{$selectedCiRun.run.ref_name}</span>
        <span class="meta-sha">{$selectedCiRun.run.sha.substring(0, 8)}</span>
        {#if $selectedCiRun.duration != null}
          <span class="meta-duration">
            {formatDuration($selectedCiRun.duration)}
          </span>
        {/if}
        {#if $selectedCiRun.run.status === 'running' || $selectedCiRun.run.status === 'pending' || $selectedCiRun.run.status === 'queued'}
          <span class="auto-refresh-label">{m.pipeline_auto_refresh()}</span>
        {/if}
      </div>

      <div class="detail-actions">
        {#if isCompleted}
          <button onclick={doRetry} disabled={busy}>{m.pipeline_action_retry()}</button>
          {#if hasFailedJob}
            <button onclick={doRetryFailed} disabled={busy}>
              {m.pipeline_action_retry_failed()}
            </button>
          {/if}
        {/if}
        {#if isActive}
          <button onclick={doCancel} disabled={busy}>{m.pipeline_action_cancel()}</button>
        {/if}
      </div>

      {#if actionError}<div class="action-error">{actionError}</div>{/if}
    </div>

    <div class="stages-flow">
      {#each $selectedCiRun.stages as stage}
        <div class="stage-card">
          <div class="stage-name">{stage.name}</div>
          <div class="stage-jobs">
            {#each stage.jobs as job (job.id)}
              <div class="job-row-wrapper">
                <button
                  class="job-row"
                  class:selected={selectedJobId === job.id}
                  onclick={() => handleJobClick(job)}
                >
                  {#if loadingJobId === job.id}
                    <div class="spinner spinner--job"></div>
                  {:else}
                    <span
                      class="job-status-icon"
                      style="color: {ciStatusColor(job.status)}"
                    >
                      {statusIcon(job.status)}
                    </span>
                  {/if}
                  <span class="job-name">{job.name}</span>
                  {#if job.duration != null}
                    <span class="job-duration">{formatDuration(job.duration)}</span>
                  {/if}
                </button>
                {#if job.status === "failed" || job.status === "timed_out"}
                  <button
                    class="job-retry-btn"
                    onclick={(e) => { e.stopPropagation(); doRetryJob(job.id); }}
                    disabled={busy}
                    title={m.pipeline_action_retry_job()}
                  >{m.pipeline_action_retry_job()}</button>
                {/if}
              </div>
            {/each}
          </div>
        </div>
        {#if $selectedCiRun.stages.indexOf(stage) < $selectedCiRun.stages.length - 1}
          <div class="stage-arrow">{"\uF061"}</div>
        {/if}
      {/each}
    </div>
  {:else}
    <div class="detail-empty">{m.pipeline_select_run()}</div>
  {/if}
</div>

<style>
  .pipeline-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .detail-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .detail-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .detail-status {
    font-size: 16px;
    font-family: var(--font-icons);
  }

  .detail-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .meta-ref {
    color: var(--accent-blue);
    font-weight: 500;
  }

  .meta-sha {
    font-family: "SF Mono", "Fira Code", monospace;
  }

  .meta-duration {
    margin-left: auto;
  }

  .auto-refresh-label {
    font-size: 11px;
    color: var(--accent-blue);
    opacity: 0.7;
    font-style: italic;
    margin-left: auto;
  }

  .stages-flow {
    display: flex;
    align-items: flex-start;
    gap: 0;
    padding: 16px;
    overflow-x: auto;
    flex: 1;
  }

  .stage-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    min-width: 180px;
    max-width: 240px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .stage-name {
    padding: 8px 12px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--text-primary) 2%, transparent);
  }

  .stage-jobs {
    display: flex;
    flex-direction: column;
  }

  .stage-arrow {
    display: flex;
    align-items: center;
    padding: 0 8px;
    color: var(--text-secondary);
    font-size: 18px;
    font-family: var(--font-icons);
    flex-shrink: 0;
    padding-top: 28px;
  }

  .job-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.15s;
  }

  .job-row:last-child {
    border-bottom: none;
  }

  .job-row:hover {
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  }

  .job-row.selected {
    background: var(--selection);
  }

  .job-status-icon {
    font-size: 12px;
    font-family: var(--font-icons);
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }

  .spinner--job {
    width: 12px;
    height: 12px;
    border-width: 1.5px;
    flex-shrink: 0;
  }

  .job-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .job-duration {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .detail-empty {
    gap: 8px;
  }

  .detail-actions { display: flex; gap: 6px; margin-top: 6px; flex-wrap: wrap; }
  .detail-actions button {
    background: var(--bg-secondary); color: var(--text-primary);
    border: 1px solid var(--border); border-radius: 4px;
    padding: 4px 10px; font-size: 11px; cursor: pointer;
  }
  .detail-actions button:hover:not(:disabled) { border-color: var(--accent-blue); color: var(--accent-blue); }
  .detail-actions button:disabled { opacity: 0.5; cursor: not-allowed; }
  .action-error { color: var(--accent-red); font-size: 11px; margin-top: 6px; }

  .job-row-wrapper { display: flex; align-items: center; gap: 4px; border-bottom: 1px solid var(--border); }
  .job-row-wrapper:last-child { border-bottom: none; }
  .job-row-wrapper .job-row { border-bottom: none; flex: 1; }
  .job-retry-btn {
    background: none; border: 1px solid var(--border); color: var(--text-secondary);
    padding: 2px 6px; font-size: 10px; border-radius: 3px; cursor: pointer; margin-right: 8px;
  }
  .job-retry-btn:hover:not(:disabled) { border-color: var(--accent-orange); color: var(--accent-orange); }
  .job-retry-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
