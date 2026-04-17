import { describe, it, expect, vi, beforeEach } from 'vitest';

// Stub requestAnimationFrame for node environment (no-op; warm replacement
// scheduling is exercised separately via explicit pool state assertions).
vi.stubGlobal(
  'requestAnimationFrame',
  (_cb: FrameRequestCallback): number => 0,
);

// ── Mock xterm.js and addons ──
const mockTerminalInstances: Array<{
  options: Record<string, unknown>;
  loadAddon: ReturnType<typeof vi.fn>;
  clear: ReturnType<typeof vi.fn>;
  reset: ReturnType<typeof vi.fn>;
  dispose: ReturnType<typeof vi.fn>;
  open: ReturnType<typeof vi.fn>;
}> = [];

vi.mock('@xterm/xterm', () => {
  class Terminal {
    options: Record<string, unknown>;
    loadAddon = vi.fn();
    clear = vi.fn();
    reset = vi.fn();
    dispose = vi.fn();
    open = vi.fn();
    onData = vi.fn();
    constructor(opts: Record<string, unknown>) {
      this.options = { ...opts };
      mockTerminalInstances.push(this as unknown as (typeof mockTerminalInstances)[number]);
    }
  }
  return { Terminal };
});

vi.mock('@xterm/addon-webgl', () => {
  class WebglAddon {}
  return { WebglAddon };
});

vi.mock('@xterm/addon-fit', () => {
  class FitAddon {
    fit = vi.fn();
    proposeDimensions = vi.fn();
  }
  return { FitAddon };
});

vi.mock('@xterm/addon-web-links', () => {
  class WebLinksAddon {}
  return { WebLinksAddon };
});

vi.mock('@xterm/addon-search', () => {
  class SearchAddon {}
  return { SearchAddon };
});

// Must mock svelte/store's `get` since the pool reads activeTheme.
// Use importOriginal so writable/derived/readable still work for dependents.
vi.mock('svelte/store', async (importOriginal) => {
  const actual = await importOriginal<typeof import('svelte/store')>();
  return {
    ...actual,
    get: vi.fn(() => null),
  };
});

// Import pool functions AFTER mocks are registered
import {
  acquireInteractive,
  releaseInteractive,
  updateInteractivePoolTheme,
  resetInteractivePool,
  getInteractivePoolStats,
} from './interactive-pool';
import type { ThemeData } from '../../types';

describe('interactive terminal pool', () => {
  beforeEach(() => {
    mockTerminalInstances.length = 0;
    resetInteractivePool();
  });

  it('acquire returns a new instance when pool is empty', () => {
    const inst = acquireInteractive();
    expect(inst).toBeDefined();
    expect(inst.terminal).toBeDefined();
    expect(inst.fitAddon).toBeDefined();
  });

  it('acquire creates instance with interactive settings', () => {
    acquireInteractive();
    const created = mockTerminalInstances[0];
    expect(created.options.disableStdin).toBe(false);
    expect(created.options.cursorBlink).toBe(true);
  });

  it('release + acquire returns a recycled instance', () => {
    const first = acquireInteractive();
    releaseInteractive(first);

    const second = acquireInteractive();
    // The recycled terminal should be the same mock object
    expect(second.terminal).toBe(first.terminal);
    // clear() and reset() should have been called during release
    expect(first.terminal.clear).toHaveBeenCalled();
    expect(first.terminal.reset).toHaveBeenCalled();
  });

  it('pool respects max size (3)', () => {
    // Acquire 4 instances, release all 4
    const instances = Array.from({ length: 4 }, () => acquireInteractive());
    instances.forEach(inst => releaseInteractive(inst));

    // Pool should hold at most 1 warm + discard the rest beyond pool capacity
    // With 4 released: first becomes warm, rest disposed
    const stats = getInteractivePoolStats();
    expect(stats.warmCount).toBeLessThanOrEqual(1);
    // 3 of the 4 should have been disposed
    const disposeCount = instances.filter(
      inst => (inst.terminal.dispose as ReturnType<typeof vi.fn>).mock.calls.length > 0
    ).length;
    expect(disposeCount).toBe(3);
  });

  it('release disposes when pool already has a warm instance', () => {
    const first = acquireInteractive();
    const second = acquireInteractive();

    releaseInteractive(first);  // first becomes warm
    releaseInteractive(second); // second is disposed (warm slot taken)

    expect(second.terminal.dispose).toHaveBeenCalled();
  });

  it('updateInteractivePoolTheme updates warm instance theme', () => {
    const inst = acquireInteractive();
    releaseInteractive(inst);

    const theme = {
      colors: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        blue: '#569cd6',
        black: '#000',
        red: '#f44747',
        green: '#6a9955',
        yellow: '#d7ba7d',
        magenta: '#c586c0',
        cyan: '#4ec9b0',
        white: '#d4d4d4',
        bright_black: '#808080',
        bright_red: '#f44747',
        bright_green: '#6a9955',
        bright_yellow: '#d7ba7d',
        bright_blue: '#569cd6',
        bright_magenta: '#c586c0',
        bright_cyan: '#4ec9b0',
        bright_white: '#ffffff',
      },
      derived: { selection: '#264f78' },
    } as unknown as ThemeData;

    updateInteractivePoolTheme(theme);
    expect(inst.terminal.options.theme).toBeDefined();
  });

  it('getInteractivePoolStats returns correct counts', () => {
    expect(getInteractivePoolStats()).toEqual({ activeCount: 0, warmCount: 0 });

    const a = acquireInteractive();
    expect(getInteractivePoolStats()).toEqual({ activeCount: 1, warmCount: 0 });

    const b = acquireInteractive();
    expect(getInteractivePoolStats()).toEqual({ activeCount: 2, warmCount: 0 });

    releaseInteractive(a);
    expect(getInteractivePoolStats()).toEqual({ activeCount: 1, warmCount: 1 });

    releaseInteractive(b);
    expect(getInteractivePoolStats()).toEqual({ activeCount: 0, warmCount: 1 });
  });
});
