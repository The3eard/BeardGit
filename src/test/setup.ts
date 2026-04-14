/**
 * Global Vitest setup file — mocks all Tauri IPC and browser APIs so that
 * store tests can run in a Node.js environment without a real Tauri window.
 *
 * Usage in tests:
 *   import { mockInvokeResponse, clearInvokeMocks, invokeMock } from "../setup";
 *
 * The `invoke` mock is fully configurable per test via `mockInvokeResponse()`.
 * Responses may be plain values or factory functions that receive the call args.
 */

import { vi, beforeEach } from "vitest";

// ---------------------------------------------------------------------------
// Configurable invoke response map
// ---------------------------------------------------------------------------

/** Map of IPC command name → response value or factory. */
const invokeResponses = new Map<string, unknown>();

/**
 * Register a mock response for a Tauri IPC command.
 * The response may be a plain value (returned as-is) or a function
 * `(args) => value` that is called with the invoke args each time.
 */
export function mockInvokeResponse(
  command: string,
  response: unknown,
): void {
  invokeResponses.set(command, response);
}

/** Clear all registered mock responses. Call in beforeEach. */
export function clearInvokeMocks(): void {
  invokeResponses.clear();
  invokeMock.mockClear();
}

// ---------------------------------------------------------------------------
// Core invoke mock
// ---------------------------------------------------------------------------

export const invokeMock = vi.fn(
  async (cmd: string, args?: Record<string, unknown>) => {
    if (invokeResponses.has(cmd)) {
      const response = invokeResponses.get(cmd);
      return typeof response === "function" ? response(args) : response;
    }
    // Default: return undefined rather than throwing, so tests that
    // only care about a subset of commands don't break on side-effect calls.
    return undefined;
  },
);

// ---------------------------------------------------------------------------
// Mock @tauri-apps/api/core
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

// ---------------------------------------------------------------------------
// Mock @tauri-apps/api/event
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(() => Promise.resolve()),
  once: vi.fn(() => Promise.resolve(() => {})),
}));

// ---------------------------------------------------------------------------
// Mock @tauri-apps/api/window
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    setTitle: vi.fn(() => Promise.resolve()),
    onCloseRequested: vi.fn(() => Promise.resolve(() => {})),
    show: vi.fn(() => Promise.resolve()),
    hide: vi.fn(() => Promise.resolve()),
  })),
  Window: vi.fn(),
}));

// ---------------------------------------------------------------------------
// Mock @tauri-apps/plugin-dialog
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(() => Promise.resolve(null)),
  save: vi.fn(() => Promise.resolve(null)),
  message: vi.fn(() => Promise.resolve()),
  ask: vi.fn(() => Promise.resolve(false)),
  confirm: vi.fn(() => Promise.resolve(false)),
}));

// ---------------------------------------------------------------------------
// Auto-clear invoke mock between tests so responses don't bleed across
// ---------------------------------------------------------------------------

beforeEach(() => {
  clearInvokeMocks();
});
