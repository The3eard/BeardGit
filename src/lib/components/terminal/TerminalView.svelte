<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Terminal from "./Terminal.svelte";
  import { activeTheme } from "$lib/stores/theme";
  import { terminalWrite, terminalResize } from "$lib/api/tauri";
  import { onTerminalOutput, offTerminalOutput } from "$lib/stores/terminal";
  import type { TerminalTabInfo } from "$lib/types";

  interface Props {
    terminal: TerminalTabInfo;
  }

  let { terminal }: Props = $props();

  let terminalComponent = $state<Terminal | undefined>();

  function handleData(data: string) {
    // Encode keyboard input as base64 and send to PTY
    const encoded = btoa(data);
    terminalWrite(terminal.sessionId, encoded);
  }

  function handleOutput(data: Uint8Array) {
    terminalComponent?.write(data);
  }

  onMount(() => {
    onTerminalOutput(terminal.sessionId, handleOutput);

    // Report initial dimensions to backend
    const xtermInstance = terminalComponent?.getTerminal();
    if (xtermInstance) {
      terminalResize(terminal.sessionId, xtermInstance.cols, xtermInstance.rows);

      // Listen for resize events from xterm
      xtermInstance.onResize(({ cols, rows }) => {
        terminalResize(terminal.sessionId, cols, rows);
      });
    }
  });

  onDestroy(() => {
    offTerminalOutput(terminal.sessionId);
  });
</script>

<div class="terminal-view">
  {#if $activeTheme}
    <Terminal
      bind:this={terminalComponent}
      mode="interactive"
      theme={$activeTheme}
      onData={handleData}
    />
  {/if}
</div>

<style>
  .terminal-view {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
  }
</style>
