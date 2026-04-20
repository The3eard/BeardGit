/**
 * Unit tests for `TasksDrawer.svelte`.
 *
 * Renders the drawer against a mocked aggregator store state and verifies
 * the shell behaves as specced:
 *
 * - Active + finished sections each render their respective entries.
 * - "Clear finished" button fires the store's `clearFinished` action.
 * - Esc key triggers the `onClose` callback.
 * - Mounting with `open=true` clears any `hasUnseenError` via `markSeen`.
 * - Clicking "View log" invokes the `open_log_directory` Tauri command.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import type { TaskEntry } from "../../../types/tasks";
import { invokeMock, mockInvokeResponse } from "../../../../test/setup";

// ---------------------------------------------------------------------------
// Mock the aggregator store so we can drive drawer state deterministically.
//
// `vi.hoisted` runs before the `vi.mock` factory so top-level stores aren't
// dereferenced before initialization.
// ---------------------------------------------------------------------------

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    tasksStore: writable<TaskEntry[]>([]),
    recentlyFinishedTasks: writable<TaskEntry[]>([]),
    activeTaskCount: writable<number>(0),
    hasUnseenError: writable<boolean>(false),
    markSeenMock: vi.fn(),
    clearFinishedMock: vi.fn(),
    cancelTaskByIdMock: vi.fn(async (_id: string) => {
      void _id;
    }),
  };
});

vi.mock("$lib/stores/tasks", () => ({
  tasksStore: mocks.tasksStore,
  activeTaskCount: mocks.activeTaskCount,
  recentlyFinishedTasks: mocks.recentlyFinishedTasks,
  hasUnseenError: mocks.hasUnseenError,
  markSeen: () => mocks.markSeenMock(),
  clearFinished: () => mocks.clearFinishedMock(),
  cancelTaskById: (id: string) => mocks.cancelTaskByIdMock(id),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeEntry(over: Partial<TaskEntry> = {}): TaskEntry {
  return {
    id: "t-1",
    kind: "git_fetch",
    title: "Fetch origin",
    startedAt: Date.now(),
    status: "running",
    actions: [{ id: "cancel", label: "Cancel", variant: "danger" }],
    ...over,
  };
}

import TasksDrawer from "../TasksDrawer.svelte";

beforeEach(() => {
  mocks.tasksStore.set([]);
  mocks.recentlyFinishedTasks.set([]);
  mocks.activeTaskCount.set(0);
  mocks.hasUnseenError.set(false);
  mocks.markSeenMock.mockClear();
  mocks.clearFinishedMock.mockClear();
  mocks.cancelTaskByIdMock.mockClear();
});

afterEach(() => {
  cleanup();
});

describe("TasksDrawer", () => {
  it("renders nothing when open=false", () => {
    const { queryByTestId } = render(TasksDrawer, {
      props: { open: false, onClose: vi.fn() },
    });
    expect(queryByTestId("tasks-drawer")).toBeNull();
  });

  it("renders active + finished sections from store state", () => {
    const running = makeEntry({
      id: "a",
      status: "running",
      title: "Fetching",
    });
    const done = makeEntry({
      id: "b",
      status: "success",
      title: "Pushed",
      finishedAt: Date.now() - 1000,
      actions: [{ id: "dismiss", label: "Dismiss", variant: "secondary" }],
    });
    mocks.tasksStore.set([running, done]);
    mocks.recentlyFinishedTasks.set([done]);

    const { getByTestId, getAllByTestId } = render(TasksDrawer, {
      props: { open: true, onClose: vi.fn() },
    });

    expect(getByTestId("tasks-drawer-active-section")).toBeTruthy();
    expect(getByTestId("tasks-drawer-finished-section")).toBeTruthy();
    const rows = getAllByTestId("task-row");
    expect(rows.length).toBeGreaterThanOrEqual(2);
  });

  it("renders an empty state when no tasks are present", () => {
    const { getByTestId } = render(TasksDrawer, {
      props: { open: true, onClose: vi.fn() },
    });
    expect(getByTestId("tasks-drawer-empty")).toBeTruthy();
  });

  it("Clear finished button delegates to the store", async () => {
    const done = makeEntry({
      id: "b",
      status: "success",
      finishedAt: Date.now(),
      actions: [{ id: "dismiss", label: "Dismiss", variant: "secondary" }],
    });
    mocks.tasksStore.set([done]);
    mocks.recentlyFinishedTasks.set([done]);

    const { getByTestId } = render(TasksDrawer, {
      props: { open: true, onClose: vi.fn() },
    });
    await fireEvent.click(getByTestId("tasks-drawer-clear"));

    expect(mocks.clearFinishedMock).toHaveBeenCalledTimes(1);
  });

  it("Esc triggers onClose", async () => {
    const onClose = vi.fn();
    render(TasksDrawer, {
      props: { open: true, onClose },
    });
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("X close button fires onClose", async () => {
    const onClose = vi.fn();
    const { getByTestId } = render(TasksDrawer, {
      props: { open: true, onClose },
    });
    await fireEvent.click(getByTestId("tasks-drawer-close"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("backdrop click fires onClose", async () => {
    const onClose = vi.fn();
    const { getByTestId } = render(TasksDrawer, {
      props: { open: true, onClose },
    });
    await fireEvent.click(getByTestId("tasks-drawer-backdrop"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("mounting with open=true calls markSeen", () => {
    mocks.hasUnseenError.set(true);
    render(TasksDrawer, {
      props: { open: true, onClose: vi.fn() },
    });
    expect(mocks.markSeenMock).toHaveBeenCalled();
  });

  it("View log button invokes open_log_directory", async () => {
    mockInvokeResponse("open_log_directory", undefined);
    const { getByTestId } = render(TasksDrawer, {
      props: { open: true, onClose: vi.fn() },
    });

    await fireEvent.click(getByTestId("tasks-drawer-view-log"));

    expect(invokeMock).toHaveBeenCalledWith("open_log_directory");
  });

  it("↓ moves focus forward across active + finished rows", async () => {
    const running = makeEntry({ id: "a", status: "running" });
    const done = makeEntry({
      id: "b",
      status: "success",
      finishedAt: Date.now(),
      actions: [{ id: "dismiss", label: "Dismiss", variant: "secondary" }],
    });
    mocks.tasksStore.set([running, done]);
    mocks.recentlyFinishedTasks.set([done]);

    const { getAllByTestId } = render(TasksDrawer, {
      props: { open: true, onClose: vi.fn() },
    });

    await fireEvent.keyDown(window, { key: "ArrowDown" });
    let wrappers = getAllByTestId("tasks-drawer-row-wrapper");
    expect(wrappers[0].getAttribute("data-focused")).toBe("true");
    expect(wrappers[1].getAttribute("data-focused")).toBe("false");

    await fireEvent.keyDown(window, { key: "ArrowDown" });
    wrappers = getAllByTestId("tasks-drawer-row-wrapper");
    expect(wrappers[0].getAttribute("data-focused")).toBe("false");
    expect(wrappers[1].getAttribute("data-focused")).toBe("true");

    // Wrap-around: third ArrowDown returns focus to index 0.
    await fireEvent.keyDown(window, { key: "ArrowDown" });
    wrappers = getAllByTestId("tasks-drawer-row-wrapper");
    expect(wrappers[0].getAttribute("data-focused")).toBe("true");
  });

  it("↑ moves focus backward and wraps to the last row", async () => {
    const running = makeEntry({ id: "a", status: "running" });
    const done = makeEntry({
      id: "b",
      status: "success",
      finishedAt: Date.now(),
      actions: [{ id: "dismiss", label: "Dismiss", variant: "secondary" }],
    });
    mocks.tasksStore.set([running, done]);
    mocks.recentlyFinishedTasks.set([done]);

    const { getAllByTestId } = render(TasksDrawer, {
      props: { open: true, onClose: vi.fn() },
    });

    // First ArrowUp with no focus yet → last row.
    await fireEvent.keyDown(window, { key: "ArrowUp" });
    const wrappers = getAllByTestId("tasks-drawer-row-wrapper");
    expect(wrappers[1].getAttribute("data-focused")).toBe("true");
  });

  it("Enter on the focused row fires the primary (first) action", async () => {
    const running = makeEntry({
      id: "a",
      status: "running",
      actions: [{ id: "cancel", label: "Cancel", variant: "danger" }],
    });
    mocks.tasksStore.set([running]);

    render(TasksDrawer, {
      props: { open: true, onClose: vi.fn() },
    });

    await fireEvent.keyDown(window, { key: "ArrowDown" });
    await fireEvent.keyDown(window, { key: "Enter" });

    expect(mocks.cancelTaskByIdMock).toHaveBeenCalledWith("a");
  });
});
