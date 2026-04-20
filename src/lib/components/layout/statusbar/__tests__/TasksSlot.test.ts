/**
 * Unit tests for `TasksSlot.svelte`.
 *
 * The slot subscribes to two derived stores from `$lib/stores/tasks`:
 *
 *   - `activeTaskCount` — number badge
 *   - `hasUnseenError` — red-dot overlay
 *
 * Both are mocked with plain writables so each test can drive the slot
 * synchronously and assert the rendered output + click behaviour.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    activeTaskCount: writable<number>(0),
    hasUnseenError: writable<boolean>(false),
  };
});

vi.mock("$lib/stores/tasks", () => ({
  activeTaskCount: mocks.activeTaskCount,
  hasUnseenError: mocks.hasUnseenError,
}));

import TasksSlot from "../TasksSlot.svelte";

beforeEach(() => {
  mocks.activeTaskCount.set(0);
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

  it("pulses when the active count increments", async () => {
    mocks.activeTaskCount.set(1);
    const { getByTestId } = render(TasksSlot, { props: { onOpen: vi.fn() } });
    await tick();
    const el = getByTestId("statusbar-tasks-slot");
    expect(el.classList.contains("pulsing")).toBe(false);

    mocks.activeTaskCount.set(2);
    await tick();
    expect(el.classList.contains("pulsing")).toBe(true);
  });
});
