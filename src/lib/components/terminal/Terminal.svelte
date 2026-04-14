<script lang="ts">
  import { onMount } from "svelte";
  import { Terminal as XTerm } from "@xterm/xterm";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import { SearchAddon } from "@xterm/addon-search";
  import type { ITheme } from "@xterm/xterm";
  import type { ThemeData } from "../../types";

  interface Props {
    mode: "interactive" | "readonly";
    theme: ThemeData;
    onData?: (data: string) => void;
    fontSize?: number;
  }

  let { mode, theme, onData, fontSize = 13 }: Props = $props();

  let containerEl: HTMLDivElement | undefined = $state();
  let terminal: XTerm | undefined = $state();
  let fitAddon: FitAddon | undefined = $state();

  function toXtermTheme(t: ThemeData): ITheme {
    return {
      background: t.colors.background,
      foreground: t.colors.foreground,
      cursor: t.colors.blue,
      cursorAccent: t.colors.background,
      selectionBackground: t.derived.selection,
      black: t.colors.black,
      red: t.colors.red,
      green: t.colors.green,
      yellow: t.colors.yellow,
      blue: t.colors.blue,
      magenta: t.colors.magenta,
      cyan: t.colors.cyan,
      white: t.colors.white,
      brightBlack: t.colors.bright_black,
      brightRed: t.colors.bright_red,
      brightGreen: t.colors.bright_green,
      brightYellow: t.colors.bright_yellow,
      brightBlue: t.colors.bright_blue,
      brightMagenta: t.colors.bright_magenta,
      brightCyan: t.colors.bright_cyan,
      brightWhite: t.colors.bright_white,
    };
  }

  onMount(() => {
    if (!containerEl) return;

    terminal = new XTerm({
      theme: toXtermTheme(theme),
      fontFamily: "'Fira Code', 'NerdFontSymbols', monospace",
      fontSize,
      disableStdin: mode === "readonly",
      cursorBlink: mode === "interactive",
      scrollback: 10000,
      convertEol: true,
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());
    terminal.loadAddon(new SearchAddon());

    terminal.open(containerEl);

    // Load WebGL addon after open (needs canvas context)
    try {
      terminal.loadAddon(new WebglAddon());
    } catch {
      // WebGL not available — fallback to canvas renderer (automatic)
    }

    fitAddon.fit();

    if (mode === "interactive" && onData) {
      terminal.onData(onData);
    }

    // ResizeObserver for auto-fit
    const observer = new ResizeObserver(() => {
      requestAnimationFrame(() => fitAddon?.fit());
    });
    observer.observe(containerEl);

    return () => {
      observer.disconnect();
      terminal?.dispose();
    };
  });

  // React to theme changes
  $effect(() => {
    if (terminal && theme) {
      terminal.options.theme = toXtermTheme(theme);
    }
  });

  // Public methods exposed via bind:this
  export function write(data: string | Uint8Array): void {
    terminal?.write(data);
  }

  export function clear(): void {
    terminal?.clear();
    terminal?.reset();
  }

  export function dispose(): void {
    terminal?.dispose();
    terminal = undefined;
  }

  export function fit(): void {
    fitAddon?.fit();
  }

  export function getTerminal(): XTerm | undefined {
    return terminal;
  }
</script>

<div class="terminal-container" bind:this={containerEl} style:background={theme.colors.background}></div>

<style>
  .terminal-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .terminal-container :global(.xterm) {
    height: 100%;
  }

  .terminal-container :global(.xterm .xterm-screen),
  .terminal-container :global(.xterm .xterm-scrollable-element) {
    height: 100%;
  }
</style>
