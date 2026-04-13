import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { SearchAddon } from "@xterm/addon-search";
import type { ITheme } from "@xterm/xterm";
import type { ThemeData } from "../../types";
import { get } from "svelte/store";
import { activeTheme } from "../../stores/theme";

export interface PooledInstance {
  terminal: XTerm;
  fitAddon: FitAddon;
}

/** Maximum read-only instances alive at once (2 visible + 1 warm). */
const MAX_POOL_SIZE = 3;

let warmInstance: PooledInstance | null = null;
let activeCount = 0;

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

function createInstance(): PooledInstance {
  const theme = get(activeTheme);
  const terminal = new XTerm({
    theme: theme ? toXtermTheme(theme) : undefined,
    fontFamily: "'Fira Code', monospace",
    fontSize: 13,
    disableStdin: true,
    cursorBlink: false,
    scrollback: 10000,
    convertEol: true,
  });

  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(new WebLinksAddon());
  terminal.loadAddon(new SearchAddon());

  return { terminal, fitAddon };
}

/** Acquire a read-only terminal instance. Grabs warm instance if available. */
export function acquire(): PooledInstance {
  let instance: PooledInstance;

  if (warmInstance) {
    instance = warmInstance;
    warmInstance = null;
  } else {
    instance = createInstance();
  }

  activeCount++;

  // Queue warm replacement after first use
  if (!warmInstance && activeCount + 1 <= MAX_POOL_SIZE) {
    requestAnimationFrame(() => {
      if (!warmInstance) {
        warmInstance = createInstance();
      }
    });
  }

  return instance;
}

/** Release a read-only terminal instance back to the pool. */
export function release(instance: PooledInstance): void {
  activeCount--;

  if (!warmInstance) {
    instance.terminal.clear();
    instance.terminal.reset();
    warmInstance = instance;
  } else {
    instance.terminal.dispose();
  }
}

/** Update theme on all pooled instances. */
export function updatePoolTheme(theme: ThemeData): void {
  const xtermTheme = toXtermTheme(theme);
  if (warmInstance) {
    warmInstance.terminal.options.theme = xtermTheme;
  }
}
