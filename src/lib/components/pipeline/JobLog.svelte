<script lang="ts">
  import { onMount } from "svelte";
  import { jobLog, loadingJobLog, stopJobLogPolling, activeProvider } from "../../stores/provider";
  import { preprocessJobLog } from "../../api/tauri";
  import { activeTheme } from "../../stores/theme";
  import { acquire, release, updatePoolTheme } from "../terminal/pool";
  import type { PooledInstance } from "../terminal/pool";
  import { WebglAddon } from "@xterm/addon-webgl";
  import * as m from "$lib/paraglide/messages";

  let terminalContainer: HTMLDivElement | undefined = $state();
  let pooledInstance: PooledInstance | null = $state(null);
  let terminalOpened = $state(false);

  onMount(() => {
    pooledInstance = acquire();

    return () => {
      stopJobLogPolling();
      if (pooledInstance) {
        release(pooledInstance);
        pooledInstance = null;
        terminalOpened = false;
      }
    };
  });

  // Open terminal into container when the DOM element becomes available
  $effect(() => {
    if (terminalContainer && pooledInstance && !terminalOpened) {
      pooledInstance.terminal.open(terminalContainer);
      try {
        pooledInstance.terminal.loadAddon(new WebglAddon());
      } catch {
        // WebGL not available — fallback to canvas renderer
      }
      pooledInstance.fitAddon.fit();
      terminalOpened = true;
    }
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

  // Write log to terminal when preprocessed log changes
  $effect(() => {
    if (!pooledInstance) return;
    if (preprocessedLog) {
      pooledInstance.terminal.clear();
      pooledInstance.terminal.reset();
      pooledInstance.terminal.write(preprocessedLog);
    } else {
      pooledInstance.terminal.clear();
      pooledInstance.terminal.reset();
    }
  });

  // Update theme
  $effect(() => {
    const theme = $activeTheme;
    if (theme) updatePoolTheme(theme);
  });

  // Auto-fit on container resize
  $effect(() => {
    if (!terminalContainer || !pooledInstance) return;
    const observer = new ResizeObserver(() => {
      requestAnimationFrame(() => pooledInstance?.fitAddon.fit());
    });
    observer.observe(terminalContainer);
    return () => observer.disconnect();
  });

  function scrollToTop() {
    pooledInstance?.terminal.scrollToTop();
  }

  function scrollToBottom() {
    pooledInstance?.terminal.scrollToBottom();
  }
</script>

<div class="job-log">
  {#if $loadingJobLog}
    <div class="log-loading">
      <div class="spinner"></div>
      <span>{m.joblog_loading()}</span>
    </div>
  {:else if preprocessedLog}
    <div class="log-toolbar">
      <button class="log-nav-btn" onclick={scrollToTop} title={m.joblog_top_title()}>
        <span class="nf">{"\uF062"}</span> {m.joblog_top()}
      </button>
      <button class="log-nav-btn" onclick={scrollToBottom} title={m.joblog_bottom_title()}>
        <span class="nf">{"\uF063"}</span> {m.joblog_bottom()}
      </button>
    </div>
    <div class="log-terminal" bind:this={terminalContainer}></div>
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

  .log-terminal {
    flex: 1;
    overflow: hidden;
    background: var(--bg-primary);
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
