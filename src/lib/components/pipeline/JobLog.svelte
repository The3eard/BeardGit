<script lang="ts">
  import { onMount } from "svelte";
  import { jobLog, loadingJobLog, jobLogUnavailable, selectedJobSteps, stopJobLogPolling, activeProvider } from "../../stores/provider";
  import { preprocessJobLog } from "../../api/tauri";
  import { activeTheme } from "../../stores/theme";
  import { acquire, release, updatePoolTheme } from "../terminal/pool";
  import type { PooledInstance } from "../terminal/pool";
  import { WebglAddon } from "@xterm/addon-webgl";
  import JobSteps from "./JobSteps.svelte";
  import * as m from "$lib/paraglide/messages";
  import EmptyState from "../common/EmptyState.svelte";
  import { IconButton } from "$lib/components/ui";

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

  // Open terminal into container once (container is always in DOM)
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

  // Determine which overlay to show (null = show terminal)
  let overlayState = $derived.by(() => {
    if ($loadingJobLog) return "loading";
    if (preprocessedLog) return null; // show terminal
    if ($jobLogUnavailable && $selectedJobSteps && $selectedJobSteps.length > 0) return "steps";
    if ($jobLogUnavailable) return "unavailable";
    return "empty";
  });

  function scrollToTop() {
    pooledInstance?.terminal.scrollToTop();
  }

  function scrollToBottom() {
    pooledInstance?.terminal.scrollToBottom();
  }
</script>

<div class="job-log">
  {#if overlayState === null}
    <div class="log-toolbar">
      <IconButton tone="default" icon={"\uF062"} description={m.joblog_top_title()} onclick={scrollToTop} />
      <IconButton tone="default" icon={"\uF063"} description={m.joblog_bottom_title()} onclick={scrollToBottom} />
    </div>
  {/if}

  <div class="log-body">
    <!-- Terminal container is ALWAYS in the DOM — never destroyed/recreated -->
    <div
      class="log-terminal"
      class:hidden={overlayState !== null}
      bind:this={terminalContainer}
    ></div>

    {#if overlayState === "loading"}
      <div class="log-overlay">
        <div class="spinner"></div>
        <span>{m.joblog_loading()}</span>
      </div>
    {:else if overlayState === "steps"}
      <div class="log-overlay-fill">
        <JobSteps steps={$selectedJobSteps ?? []} />
      </div>
    {:else if overlayState === "unavailable"}
      <div class="log-overlay">
        <span class="nf">{"\uF017"}</span>
        <span>{m.joblog_unavailable()}</span>
      </div>
    {:else if overlayState === "empty"}
      <div class="log-overlay log-overlay--empty">
        <EmptyState fill icon={"\uF120"} title={m.joblog_empty()} />
      </div>
    {/if}
  </div>
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

  .log-body {
    flex: 1;
    position: relative;
    overflow: hidden;
  }

  .log-terminal {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .log-terminal.hidden {
    visibility: hidden;
  }

  .log-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-secondary);
    font-size: var(--font-size-md);
    font-style: italic;
    background: var(--bg-primary);
  }

  .log-overlay--empty {
    font-style: normal;
  }

  .log-overlay-fill {
    position: absolute;
    inset: 0;
    background: var(--bg-primary);
    overflow: hidden;
  }
</style>
