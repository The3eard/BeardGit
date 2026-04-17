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

    // Report initial dimensions to backend and focus
    const xtermInstance = terminalComponent?.getTerminal();
    if (xtermInstance) {
      terminalResize(terminal.sessionId, xtermInstance.cols, xtermInstance.rows);
      xtermInstance.focus();

      // Listen for resize events from xterm
      xtermInstance.onResize(({ cols, rows }) => {
        terminalResize(terminal.sessionId, cols, rows);
      });
    }
  });

  onDestroy(() => {
    offTerminalOutput(terminal.sessionId);
  });

  function handleClick() {
    terminalComponent?.getTerminal()?.focus();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="terminal-view"
  data-testid="terminal-view"
  onclick={handleClick}
  style:background={$activeTheme?.colors.background ?? 'var(--bg-primary)'}
>
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
    flex: 1;
    width: 100%;
    min-height: 0;
    overflow: hidden;
  }
</style>
