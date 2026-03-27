<script lang="ts">
  import { onMount } from "svelte";
  import { jobLog, loadingJobLog, stopJobLogPolling, activeProvider } from "../../stores/provider";
  import { preprocessJobLog } from "../../api/tauri";
  import { ansiToHtml } from "../../utils/ansi";
  import * as m from "$lib/paraglide/messages";

  let logContainer: HTMLDivElement | undefined = $state();

  onMount(() => {
    return () => stopJobLogPolling();
  });

  let preprocessedLog = $state<string | null>(null);

  $effect(() => {
    const raw = $jobLog;
    const provider = $activeProvider;
    if (raw && provider) {
      preprocessJobLog(raw, provider.kind).then((cleaned) => {
        preprocessedLog = cleaned;
      });
    } else {
      preprocessedLog = null;
    }
  });

  let htmlLog = $derived(preprocessedLog ? ansiToHtml(preprocessedLog) : null);

  $effect(() => {
    if (htmlLog && logContainer) {
      requestAnimationFrame(() => {
        if (logContainer) {
          logContainer.scrollTop = logContainer.scrollHeight;
        }
      });
    }
  });

  function scrollToTop() {
    if (logContainer) {
      logContainer.scrollTop = 0;
    }
  }

  function scrollToBottom() {
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  }
</script>

<div class="job-log">
  {#if $loadingJobLog}
    <div class="log-loading">
      <div class="spinner"></div>
      <span>{m.joblog_loading()}</span>
    </div>
  {:else if htmlLog}
    <div class="log-toolbar">
      <button class="log-nav-btn" onclick={scrollToTop} title={m.joblog_top_title()}>
        <span class="nf">{"\uF062"}</span> {m.joblog_top()}
      </button>
      <button class="log-nav-btn" onclick={scrollToBottom} title={m.joblog_bottom_title()}>
        <span class="nf">{"\uF063"}</span> {m.joblog_bottom()}
      </button>
    </div>
    <div class="log-content" bind:this={logContainer}>{@html htmlLog}</div>
  {:else}
    <div class="log-empty">{m.joblog_empty()}</div>
  {/if}
</div>

<style>
  .job-log {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .log-toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .log-nav-btn {
    background: var(--overlay-hover);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .log-nav-btn:hover {
    background: var(--overlay-active);
    color: var(--text-primary);
  }

  .log-content {
    flex: 1;
    padding: 12px 16px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-y: auto;
    margin: 0;
    -webkit-user-select: text;
    user-select: text;
  }

  .log-loading {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .log-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 13px;
    font-style: italic;
  }
</style>
