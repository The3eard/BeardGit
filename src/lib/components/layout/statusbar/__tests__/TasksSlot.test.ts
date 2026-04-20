/**
 * Unit tests for `TasksSlot.svelte`.
 *
 * The slot subscribes to four derived stores from `$lib/stores/tasks`:
 *
 *   - `activeTaskCount` — number badge
 *   - `anyRunning`      — spin animation trigger
 *   - `latestEntry`     — status-coloured glyph
 *   - `hasUnseenError`  — red-dot overlay
 *
 * All four are mocked with plain writables so each test can drive the
 * slot synchronously and assert the rendered output + click behaviour.
 *
 * The old cluster-0.3 pulse-on-count-increase behaviour was retired
 * alongside the drawer — the spin animation replaces it so users see
 * that background work is actively in flight, not just a count bump.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { TaskEntry } from "$lib/types/tasks";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    activeTaskCount: writable<number>(0),
    anyRunning: writable<boolean>(false),
    latestEntry: writable<TaskEntry | null>(null),
    hasUnseenError: writable<boolean>(false),
  };
});

vi.mock("$lib/stores/tasks", () => ({
  activeTaskCount: mocks.activeTaskCount,
  anyRunning: mocks.anyRunning,
  latestEntry: mocks.latestEntry,
  hasUnseenError: mocks.hasUnseenError,
}));

import TasksSlot from "../TasksSlot.svelte";

function entry(over: Partial<TaskEntry> = {}): TaskEntry {
  return {
    id: "t-1",
    kind: "git_fetch",
    title: "Fetch origin",
    startedAt: Date.now(),
    status: "running",
    actions: [],
    ...over,
  };
}

beforeEach(() => {
  mocks.activeTaskCount.set(0);
  mocks.anyRunning.set(false);
  mocks.latestEntry.set(null);
  mocks.hasUnseenError.set(false);
});

afterEach(() => cleanup());

describe("TasksSlot", () => {
  it("renders without a count when no tasks are active", async () => {
    const { queryByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    expect(queryByTestId("statusbar-tasks-count")).toBeNull();
  });

  it("renders the active count when > 0", async () => {
    mocks.activeTaskCount.set(3);
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    expect(getByTestId("statusbar-tasks-count").textContent).toBe("3");
  });

  it("invokes onOpen when clicked", async () => {
    const onOpen = vi.fn();
    const { getByTestId } = render(TasksSlot, { props: { onOpen } });
    await fireEvent.click(getByTestId("statusbar-tasks-slot"));
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("shows the red error dot when hasUnseenError is true", async () => {
    mocks.hasUnseenError.set(true);
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    expect(getByTestId("statusbar-tasks-error-dot")).toBeTruthy();
    expect(
      getByTestId("statusbar-tasks-slot").classList.contains("has-error"),
    ).toBe(true);
  });

  it("clears the red error dot when hasUnseenError flips false", async () => {
    mocks.hasUnseenError.set(true);
    const { queryByTestId } = render(TasksSlot, {
      props: { onOpen: vi.fn() },
    });
    await tick();
    expect(queryByTestId("statusbar-tasks-error-dot")).toBeTruthy();

    mocks.hasUnseenError.set(false);
    await tick();
    expect(queryByTestId("statusbar-tasks-error-dot")).toBeNull();
  });

  it("spins while any task is running", async () => {
    mocks.anyRunning.set(true);
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    const slot = getByTestId("statusbar-tasks-slot");
    expect(slot.classList.contains("spinning")).toBe(true);
    expect(slot.querySelector(".glyph.spin")).toBeTruthy();
  });

  it("stops spinning when anyRunning flips false", async () => {
    mocks.anyRunning.set(true);
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    expect(getByTestId("statusbar-tasks-slot").classList.contains("spinning"))
      .toBe(true);

    mocks.anyRunning.set(false);
    await tick();
    expect(getByTestId("statusbar-tasks-slot").classList.contains("spinning"))
      .toBe(false);
  });

  it("applies state-idle when there is no latest entry", async () => {
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    expect(getByTestId("statusbar-tasks-slot").dataset.state).toBe("idle");
  });

  it("applies state-running when the latest entry is running", async () => {
    mocks.latestEntry.set(entry({ status: "running" }));
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    const slot = getByTestId("statusbar-tasks-slot");
    expect(slot.dataset.state).toBe("running");
    expect(slot.classList.contains("state-running")).toBe(true);
  });

  it("applies state-error when the latest entry failed", async () => {
    mocks.latestEntry.set(
      entry({ status: "error", errorMessage: "boom" }),
    );
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    const slot = getByTestId("statusbar-tasks-slot");
    expect(slot.dataset.state).toBe("error");
    expect(slot.classList.contains("state-error")).toBe(true);
  });

  it("applies state-success when the latest entry succeeded", async () => {
    mocks.latestEntry.set(entry({ status: "success" }));
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    expect(getByTestId("statusbar-tasks-slot").dataset.state).toBe("success");
  });

  it("applies state-cancelled when the latest entry was cancelled", async () => {
    mocks.latestEntry.set(entry({ status: "cancelled" }));
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    expect(getByTestId("statusbar-tasks-slot").dataset.state).toBe("cancelled");
  });

  it("reactively transitions the state class when the latest entry changes", async () => {
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    expect(getByTestId("statusbar-tasks-slot").dataset.state).toBe("idle");

    mocks.latestEntry.set(entry({ status: "running" }));
    await tick();
    expect(getByTestId("statusbar-tasks-slot").dataset.state).toBe("running");

    mocks.latestEntry.set(entry({ status: "success" }));
    await tick();
    expect(getByTestId("statusbar-tasks-slot").dataset.state).toBe("success");
  });
});
