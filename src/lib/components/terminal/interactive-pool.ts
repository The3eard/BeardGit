/**
 * Pool for interactive xterm.js terminal instances.
 *
 * Mirrors the read-only pool in `pool.ts` but creates terminals with
 * `disableStdin: false` and `cursorBlink: true`. Reuses instances on
 * acquire/release to avoid WebGL context churn and GC pressure.
 *
 * Max pool size: 3 (2 visible + 1 warm spare).
 */

import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { SearchAddon } from "@xterm/addon-search";
import type { ITheme } from "@xterm/xterm";
import type { ThemeData } from "../../types";
import { get } from "svelte/store";
import { activeTheme } from "../../stores/theme";

export interface InteractivePooledInstance {
  terminal: XTerm;
  fitAddon: FitAddon;
}

/** Maximum interactive instances alive at once (2 visible + 1 warm). */
const MAX_POOL_SIZE = 3;

let warmInstance: InteractivePooledInstance | null = null;
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

function createInstance(): InteractivePooledInstance {
  const theme = get(activeTheme);
  const terminal = new XTerm({
    theme: theme ? toXtermTheme(theme) : undefined,
    fontFamily: "'Fira Code', 'NerdFontSymbols', monospace",
    fontSize: 13,
    disableStdin: false,
    cursorBlink: true,
    scrollback: 10000,
    convertEol: true,
  });

  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(new WebLinksAddon());
  terminal.loadAddon(new SearchAddon());

  return { terminal, fitAddon };
}

/**
 * Acquire an interactive terminal instance.
 *
 * Returns a warm spare if available, otherwise creates a new one.
 * After acquisition, queues creation of a new warm spare if under the
 * pool size limit.
 */
export function acquireInteractive(): InteractivePooledInstance {
  let instance: InteractivePooledInstance;

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

/**
 * Release an interactive terminal instance back to the pool.
 *
 * If the warm slot is empty, clears the terminal and keeps it for reuse.
 * Otherwise, disposes the instance to stay within the pool size limit.
 */
export function releaseInteractive(instance: InteractivePooledInstance): void {
  activeCount--;

  if (!warmInstance) {
    instance.terminal.clear();
    instance.terminal.reset();
    warmInstance = instance;
  } else {
    instance.terminal.dispose();
  }
}

/** Update theme on the warm pooled instance, if any. */
export function updateInteractivePoolTheme(theme: ThemeData): void {
  const xtermTheme = toXtermTheme(theme);
  if (warmInstance) {
    warmInstance.terminal.options.theme = xtermTheme;
  }
}

/** Reset the pool — dispose warm instance and reset counts. For testing. */
export function resetInteractivePool(): void {
  if (warmInstance) {
    warmInstance.terminal.dispose();
    warmInstance = null;
  }
  activeCount = 0;
}

/** Return pool stats for testing and debugging. */
export function getInteractivePoolStats(): { activeCount: number; warmCount: number } {
  return {
    activeCount,
    warmCount: warmInstance ? 1 : 0,
  };
}
