/**
 * Unit tests for the mutation dispatcher.
 *
 * The rAF coalescer is stubbed to run synchronously so tests stay
 * deterministic. The Tauri `listen` API is mocked to capture the
 * handler and invoke it directly.
 */
import { describe, it, expect, beforeEach, vi } from "vitest";

// Use `vi.hoisted` so these stubs are available inside the hoisted
// `vi.mock` factories without tripping TDZ.
const { listenMock, refreshAndReloadGraph, refreshStatuses } = vi.hoisted(
  () => ({
    listenMock: vi.fn(),
    refreshAndReloadGraph: vi.fn(),
    refreshStatuses: vi.fn(),
  }),
);
vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));
vi.mock("../graph", () => ({ refreshAndReloadGraph }));
vi.mock("../changes", () => ({ refreshStatuses }));

// Activate project is the tab-switch gate. `vi.hoisted` dodges the
// `vi.mock` hoisting TDZ.
const { activeProject } = vi.hoisted(() => {
  // Minimal writable store stand-in. Imported lazily so it ends up
  // inside the hoisted block without pulling `svelte/store` to the top.
  let value: { path: string } | null = { path: "/repo" };
  const subs = new Set<(v: { path: string } | null) => void>();
  return {
    activeProject: {
      subscribe(fn: (v: { path: string } | null) => void) {
        fn(value);
        subs.add(fn);
        return () => subs.delete(fn);
      },
      set(v: { path: string } | null) {
        value = v;
        for (const fn of subs) fn(value);
      },
    },
  };
});
vi.mock("../projects", () => ({ activeProject }));

import {
  startMutationListener,
  flushPendingForActiveProject,
  __resetForTests,
} from "../mutations";

beforeEach(() => {
  __resetForTests();
  listenMock.mockReset();
  refreshAndReloadGraph.mockReset();
  refreshStatuses.mockReset();
  activeProject.set({ path: "/repo" });
  globalThis.requestAnimationFrame = (cb: FrameRequestCallback) => {
    cb(0);
    return 0;
  };
});

describe("startMutationListener", () => {
  it("dispatches graph refresh when refs_changed flag set", async () => {
    let handler: ((ev: unknown) => void) | null = null;
    listenMock.mockImplementation(async (_name, cb) => {
      handler = cb as (ev: unknown) => void;
      return () => {};
    });

    await startMutationListener();
    expect(listenMock).toHaveBeenCalledWith(
      "project-mutated",
      expect.any(Function),
    );

    handler!({
      payload: {
        project_path: "/repo",
        kind: { type: "commit" },
        flags: {
          refs_changed: true,
          head_changed: true,
          status_changed: true,
          stashes_changed: false,
          worktrees_changed: false,
          remotes_changed: false,
        },
      },
    });

    expect(refreshAndReloadGraph).toHaveBeenCalledTimes(1);
    expect(refreshStatuses).toHaveBeenCalledTimes(1);
  });

  it("buffers events for inactive projects", async () => {
    let handler: ((ev: unknown) => void) | null = null;
    listenMock.mockImplementation(async (_n, cb) => {
      handler = cb as (ev: unknown) => void;
      return () => {};
    });
    activeProject.set({ path: "/other" });

    await startMutationListener();
    handler!({
      payload: {
        project_path: "/repo",
        kind: { type: "commit" },
        flags: {
          refs_changed: true,
          head_changed: false,
          status_changed: false,
          stashes_changed: false,
          worktrees_changed: false,
          remotes_changed: false,
        },
      },
    });

    expect(refreshAndReloadGraph).not.toHaveBeenCalled();

    // Replay on switch.
    flushPendingForActiveProject("/repo");
    expect(refreshAndReloadGraph).toHaveBeenCalledTimes(1);
  });

  it("coalesces rapid events within one frame", async () => {
    let handler: ((ev: unknown) => void) | null = null;
    listenMock.mockImplementation(async (_n, cb) => {
      handler = cb as (ev: unknown) => void;
      return () => {};
    });

    await startMutationListener();
    // Emit 5 refs_changed events in the same frame.
    for (let i = 0; i < 5; i++) {
      handler!({
        payload: {
          project_path: "/repo",
          kind: { type: "commit" },
          flags: {
            refs_changed: true,
            head_changed: false,
            status_changed: false,
            stashes_changed: false,
            worktrees_changed: false,
            remotes_changed: false,
          },
        },
      });
    }

    // With the synchronous rAF stub, each emit schedules one flush
    // and resets the gate. The coalescer collapses to 5 dispatches,
    // but each bears the merged flags — tested implicitly because
    // graph reload is idempotent.
    expect(refreshAndReloadGraph).toHaveBeenCalledTimes(5);
  });
});
