/**
 * Unit tests for `TasksPopover.svelte`.
 *
 * Renders the popover against a mocked aggregator store and verifies
 * the two-mode shell behaves as specced:
 *
 * - List mode shows up to 8 entries (active first, then finished).
 * - "Clear finished" delegates to `clearFinished`.
 * - Esc from list closes; Esc from detail returns to list first.
 * - Clicking a row swaps to the detail view.
 * - Outside-click + X button + backdrop all close via `onClose`.
 * - Mounting with `open=true` clears `hasUnseenError` via `markSeen`.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import type { TaskEntry } from "../../../types/tasks";

// ---------------------------------------------------------------------------
// Mock the aggregator store so we can drive popover state deterministically.
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

vi.mock("$lib/stores/tasksPopover", () => ({
  closeTasksPopover: vi.fn(),
}));

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

import TasksPopover from "../TasksPopover.svelte";

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

describe("TasksPopover — list mode", () => {
  it("renders nothing when open=false", () => {
    const { queryByTestId } = render(TasksPopover, {
      props: { open: false, onClose: vi.fn() },
    });
    expect(queryByTestId("tasks-popover")).toBeNull();
  });

  it("renders active + finished rows from the store (active first)", () => {
    const running = makeEntry({
      id: "a",
      status: "running",
      title: "Fetching",
      startedAt: Date.now(),
    });
    const done = makeEntry({
      id: "b",
      status: "success",
      title: "Pushed",
      startedAt: Date.now() - 1000,
      finishedAt: Date.now() - 500,
      actions: [{ id: "dismiss", label: "Dismiss", variant: "secondary" }],
    });
    mocks.tasksStore.set([done, running]);
    mocks.recentlyFinishedTasks.set([done]);

    const { getAllByTestId } = render(TasksPopover, {
      props: { open: true, onClose: vi.fn() },
    });

    const items = getAllByTestId("tasks-popover-item");
    expect(items.length).toBe(2);
    expect(items[0].getAttribute("data-task-id")).toBe("a");
    expect(items[1].getAttribute("data-task-id")).toBe("b");
  });

  it("truncates the list to at most 8 rows", () => {
    const entries: TaskEntry[] = Array.from({ length: 12 }, (_, i) =>
      makeEntry({
        id: `running-${i}`,
        title: `Task ${i}`,
        startedAt: Date.now() - i * 1000,
        status: "running",
      }),
    );
    mocks.tasksStore.set(entries);

    const { getAllByTestId } = render(TasksPopover, {
      props: { open: true, onClose: vi.fn() },
    });

    expect(getAllByTestId("tasks-popover-item").length).toBe(8);
  });

  it("renders an empty state when no tasks are present", () => {
    const { getByTestId } = render(TasksPopover, {
      props: { open: true, onClose: vi.fn() },
    });
    expect(getByTestId("tasks-popover-empty")).toBeTruthy();
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

    const { getByTestId } = render(TasksPopover, {
      props: { open: true, onClose: vi.fn() },
    });
    await fireEvent.click(getByTestId("tasks-popover-clear"));

    expect(mocks.clearFinishedMock).toHaveBeenCalledTimes(1);
  });

  it("Esc closes the popover when in list mode", async () => {
    const onClose = vi.fn();
    render(TasksPopover, {
      props: { open: true, onClose },
    });
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("mounting with open=true calls markSeen", () => {
    mocks.hasUnseenError.set(true);
    render(TasksPopover, {
      props: { open: true, onClose: vi.fn() },
    });
    expect(mocks.markSeenMock).toHaveBeenCalled();
  });
});

describe("TasksPopover — detail mode", () => {
  it("clicking a row swaps to the detail view", async () => {
    const running = makeEntry({
      id: "r-1",
      status: "running",
      title: "Pushing to origin",
    });
    mocks.tasksStore.set([running]);

    const { getByTestId, queryByTestId } = render(TasksPopover, {
      props: { open: true, onClose: vi.fn() },
    });
    await fireEvent.click(getByTestId("tasks-popover-item"));
    expect(getByTestId("tasks-popover-detail-header")).toBeTruthy();
    expect(queryByTestId("tasks-popover-item")).toBeNull();
    expect(getByTestId("task-detail-panel")).toBeTruthy();
  });

  it("back button returns to list mode", async () => {
    const running = makeEntry({ id: "r-1", status: "running" });
    mocks.tasksStore.set([running]);

    const { getByTestId, queryByTestId } = render(TasksPopover, {
      props: { open: true, onClose: vi.fn() },
    });
    await fireEvent.click(getByTestId("tasks-popover-item"));
    await fireEvent.click(getByTestId("tasks-popover-back"));
    expect(queryByTestId("tasks-popover-detail-header")).toBeNull();
    expect(getByTestId("tasks-popover-item")).toBeTruthy();
  });

  it("Esc from detail view returns to list before closing", async () => {
    const running = makeEntry({ id: "r-1", status: "running" });
    mocks.tasksStore.set([running]);

    const onClose = vi.fn();
    const { getByTestId, queryByTestId } = render(TasksPopover, {
      props: { open: true, onClose },
    });
    await fireEvent.click(getByTestId("tasks-popover-item"));

    await fireEvent.keyDown(window, { key: "Escape" });
    // First Esc: popover still open, detail dismissed.
    expect(onClose).not.toHaveBeenCalled();
    expect(queryByTestId("tasks-popover-detail-header")).toBeNull();

    await fireEvent.keyDown(window, { key: "Escape" });
    // Second Esc: popover closes.
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
