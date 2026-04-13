<script lang="ts">
  import { onMount } from "svelte";
  import { sortedTasks, selectedOutput, selectedTask, collapsePanel, closePanel } from "../../stores/tasks";
  import { activeTheme } from "../../stores/theme";
  import { acquire, release, updatePoolTheme } from "../terminal/pool";
  import type { PooledInstance } from "../terminal/pool";
  import { WebglAddon } from "@xterm/addon-webgl";
  import TaskList from "./TaskList.svelte";
  import * as m from "$lib/paraglide/messages";

  let outputContainer: HTMLDivElement | undefined = $state();
  let pooledInstance: PooledInstance | null = $state(null);
  let lastWrittenLength = $state(0);

  let taskCommand = $derived($selectedTask?.command ?? null);

  onMount(() => {
    pooledInstance = acquire();
    if (outputContainer && pooledInstance) {
      pooledInstance.terminal.open(outputContainer);
      try {
        pooledInstance.terminal.loadAddon(new WebglAddon());
      } catch {
        // WebGL not available — fallback to canvas renderer
      }
      pooledInstance.fitAddon.fit();
    }

    return () => {
      if (pooledInstance) {
        release(pooledInstance);
        pooledInstance = null;
      }
    };
  });

  // Write output to terminal when selected task changes or new output arrives
  $effect(() => {
    const output = $selectedOutput;
    if (!pooledInstance) return;

    if (output.length === 0) {
      pooledInstance.terminal.clear();
      pooledInstance.terminal.reset();
      lastWrittenLength = 0;
      return;
    }

    // Full rewrite on task switch (selectedTask changed)
    if (lastWrittenLength > output.length) {
      pooledInstance.terminal.clear();
      pooledInstance.terminal.reset();
      lastWrittenLength = 0;
    }

    // Write only new lines (incremental)
    const newLines = output.slice(lastWrittenLength);
    if (newLines.length > 0) {
      const text = newLines.map((l) => l.text).join("\r\n") + "\r\n";
      pooledInstance.terminal.write(text);
      lastWrittenLength = output.length;
    }
  });

  // Update theme
  $effect(() => {
    const theme = $activeTheme;
    if (theme) updatePoolTheme(theme);
  });

  // Auto-fit on container resize
  $effect(() => {
    if (!outputContainer || !pooledInstance) return;
    const observer = new ResizeObserver(() => {
      requestAnimationFrame(() => pooledInstance?.fitAddon.fit());
    });
    observer.observe(outputContainer);
    return () => observer.disconnect();
  });
</script>

<div class="task-panel">
  <div class="panel-sidebar">
    <div class="panel-header">
      <span class="header-title">{m.tasks_title()}</span>
      <div class="header-actions">
        <button class="panel-icon-btn" onclick={collapsePanel} title={m.tasks_collapse_tooltip()}>
          <span class="nf">{"\uF066"}</span>
        </button>
        <button class="panel-icon-btn" onclick={closePanel} title="Close">
          <span class="nf">{"\uF00D"}</span>
        </button>
      </div>
    </div>
    <div class="panel-list">
      <TaskList tasks={$sortedTasks} />
    </div>
  </div>

  <div class="panel-output">
    <div class="output-header">
      <span class="output-label">{m.tasks_output()}</span>
    </div>
    {#if taskCommand}
      <div class="output-command">
        <span class="command-prompt">$</span> {taskCommand}
      </div>
    {/if}
    {#if $selectedTask}
      <div class="output-terminal" bind:this={outputContainer}></div>
    {:else}
      <div class="output-empty">{m.tasks_select_task()}</div>
    {/if}
  </div>
</div>

<style>
  .task-panel {
    display: flex;
    height: 100%;
    overflow: hidden;
    background: var(--bg-secondary);
  }

  .panel-sidebar {
    width: clamp(160px, 15vw, 220px);
    min-width: 0;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .header-title {
    font-weight: 600;
    font-size: 11px;
    color: var(--text-primary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .panel-icon-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
  }

  .panel-icon-btn:hover {
    color: var(--text-primary);
  }

  .panel-list {
    flex: 1;
    overflow-y: auto;
  }

  .panel-output {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .output-header {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .output-label {
    color: var(--text-secondary);
    font-size: 11px;
  }

  .output-command {
    padding: 6px 8px;
    background: var(--bg-primary);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-blue);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: all;
  }

  .command-prompt {
    color: var(--text-secondary);
    margin-right: 4px;
  }

  .output-terminal {
    flex: 1;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .output-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 12px;
    font-style: italic;
  }
</style>
