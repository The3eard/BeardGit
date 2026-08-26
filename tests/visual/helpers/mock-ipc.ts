/**
 * Playwright-side stub of `window.__TAURI_INTERNALS__`.
 *
 * The visual suite runs the Svelte UI under `npm run dev` (Vite +
 * Chromium), where no Tauri backend exists. Without a stub, every
 * `invoke()` call from `@tauri-apps/api/core` throws because
 * `window.__TAURI_INTERNALS__` is undefined.
 *
 * `installMockIPC` injects an init script that defines the stub
 * BEFORE the app bundle loads, so the SDK finds it and resolves
 * commands against a JS-side response map instead of the Rust IPC
 * channel.
 *
 * Surface mirrored from `node_modules/@tauri-apps/api/{core,event}.js`:
 *   __TAURI_INTERNALS__.invoke(cmd, args, options)
 *   __TAURI_INTERNALS__.transformCallback(cb, once)
 *   __TAURI_INTERNALS__.unregisterCallback(id)
 *   __TAURI_INTERNALS__.convertFileSrc(path, protocol)
 *   __TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener(event, id)
 */

import type { Page } from "@playwright/test";

import type { IpcCommandName } from "./ipc-commands.generated";

/**
 * Map of Tauri IPC command name (snake_case, exactly as passed to
 * `invoke()`) → response value. Values must be JSON-serialisable; if
 * a test needs different data per call, swap the map at runtime via
 * {@link setMockResponses} or {@link patchMockResponses}.
 *
 * Unregistered commands resolve to `undefined` (not throw), matching
 * the vitest setup at `src/test/setup.ts:43`.
 *
 * Keyed by the generated union of real commands rather than by `string`.
 * A misspelled key used to be indistinguishable from a command the view
 * never calls — it just answered nothing — and six of them had
 * accumulated in `routes.spec.ts`. One (`get_remote_repo_config`, really
 * `load_remote_repo_config`) made the Repo settings view render an error
 * card, which was then recorded as its baseline and passed forever.
 */
export type IpcResponses = Partial<Record<IpcCommandName, unknown>>;

/**
 * A response that depends on one of the call's arguments.
 *
 * Most commands answer the same thing however they are called, and a flat
 * map covers them. `list_workdir_tree` does not: the editor tree lists one
 * directory at a time, so a single canned array would hand the same
 * children to every folder and draw a tree that repeats itself. Wrap the
 * value in this to key it on an argument instead.
 *
 *     list_workdir_tree: byArg("prefix", { src: srcChildren }, rootEntries)
 *
 * Serialisable on purpose — the response map is passed through
 * `addInitScript`, so a callback would not survive the trip.
 */
export interface ByArgResponse {
  __byArg: {
    /** Argument name to switch on. `null` / `undefined` reads as `""`. */
    key: string;
    /** Value per argument. */
    cases: Record<string, unknown>;
    /** Used when the argument matches no case. */
    fallback: unknown;
  };
}

/** Build a {@link ByArgResponse}. See its docs for when that is needed. */
export function byArg(
  key: string,
  cases: Record<string, unknown>,
  fallback: unknown,
): ByArgResponse {
  return { __byArg: { key, cases, fallback } };
}

/** A captured IPC call recorded by the in-page mock. */
export interface MockCall {
  cmd: string;
  args: unknown;
  /** 1-based call index in invocation order. */
  index: number;
}

/** Internal shape exposed at `window.__beardgitMockIPC` for runtime hooks. */
interface MockState {
  responses: IpcResponses;
  calls: MockCall[];
  callbacks: Map<number, (payload: unknown) => void>;
  nextCallbackId: number;
}

declare global {
  interface Window {
    __beardgitMockIPC?: MockState;
    __TAURI_INTERNALS__?: {
      invoke: (cmd: string, args?: unknown, options?: unknown) => Promise<unknown>;
      transformCallback: (callback: (payload: unknown) => void, once?: boolean) => number;
      unregisterCallback: (id: number) => void;
      convertFileSrc: (path: string, protocol?: string) => string;
      metadata?: { currentWindow?: { label?: string } };
    };
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: (event: string, eventId: number) => void;
    };
  }
}

/**
 * Install the Tauri IPC stub on the next navigation. Call once per
 * `page` (typically in `test.beforeEach`) before `page.goto()`.
 */
export async function installMockIPC(
  page: Page,
  responses: IpcResponses = {},
): Promise<void> {
  await page.addInitScript((initial: IpcResponses) => {
    const state: MockState = {
      responses: { ...initial },
      calls: [],
      callbacks: new Map(),
      nextCallbackId: 1,
    };
    window.__beardgitMockIPC = state;

    const internals = {
      invoke(cmd: string, args: unknown): Promise<unknown> {
        state.calls.push({ cmd, args, index: state.calls.length + 1 });
        const canned = (state.responses as Record<string, unknown>)[cmd];
        const spec = (canned as ByArgResponse | undefined)?.__byArg;
        if (spec) {
          const argv = (args ?? {}) as Record<string, unknown>;
          const key = String(argv[spec.key] ?? "");
          return Promise.resolve(
            Object.prototype.hasOwnProperty.call(spec.cases, key)
              ? spec.cases[key]
              : spec.fallback,
          );
        }
        return Promise.resolve(canned);
      },
      transformCallback(callback: (payload: unknown) => void): number {
        const id = state.nextCallbackId++;
        state.callbacks.set(id, callback);
        return id;
      },
      unregisterCallback(id: number): void {
        state.callbacks.delete(id);
      },
      convertFileSrc(path: string): string {
        return path;
      },
      metadata: { currentWindow: { label: "main" } },
    };

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: internals,
      writable: true,
      configurable: true,
    });
    Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
      value: { unregisterListener(): void {} },
      writable: true,
      configurable: true,
    });
  }, responses);
}

/** Replace the entire response map at runtime. */
export async function setMockResponses(
  page: Page,
  responses: IpcResponses,
): Promise<void> {
  await page.evaluate((next: IpcResponses) => {
    if (window.__beardgitMockIPC) {
      window.__beardgitMockIPC.responses = { ...next };
    }
  }, responses);
}

/** Merge new entries into the existing response map. */
export async function patchMockResponses(
  page: Page,
  responses: IpcResponses,
): Promise<void> {
  await page.evaluate((next: IpcResponses) => {
    if (window.__beardgitMockIPC) {
      window.__beardgitMockIPC.responses = {
        ...window.__beardgitMockIPC.responses,
        ...next,
      };
    }
  }, responses);
}

/** Fire an event from the mock side as if Rust had emitted it. */
export async function emitMockEvent(
  page: Page,
  event: string,
  payload: unknown,
): Promise<void> {
  await page.evaluate(
    ({ event: e, payload: p }) => {
      const state = window.__beardgitMockIPC;
      if (!state) return;
      for (const cb of state.callbacks.values()) {
        cb({ event: e, id: 0, payload: p });
      }
    },
    { event, payload },
  );
}

/** Read captured IPC calls — optionally filtered by command name. */
export async function getMockCalls(
  page: Page,
  cmd?: string,
): Promise<MockCall[]> {
  return page.evaluate((filter) => {
    const state = window.__beardgitMockIPC;
    if (!state) return [];
    return filter
      ? state.calls.filter((c) => c.cmd === filter)
      : [...state.calls];
  }, cmd);
}

/** Clear the captured-call buffer (responses untouched). */
export async function clearMockCalls(page: Page): Promise<void> {
  await page.evaluate(() => {
    if (window.__beardgitMockIPC) window.__beardgitMockIPC.calls = [];
  });
}
