/**
 * Unit tests for `TaskEntryRow.svelte`.
 *
 * Asserts the row renders the right icon / actions / progress for every
 * meaningful slice of `(kind, status, progress)`:
 *
 * - Running AI row shows a cancel button; click fires `onAction("cancel")`.
 * - Failed git row shows Retry + Dismiss actions (retry is exposed for AI
 *   and update only, so the dismiss-only branch is covered too).
 * - Completed row hides the cancel button.
 * - Determinate progress renders a bar with the correct percent; the
 *   indeterminate branch renders the animated stripe element.
 * - AI-kind entries with a provider hint in title/subtitle render the
 *   `ProviderIcon` instead of the generic glyph.
 */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import type { TaskEntry } from "../../../types/tasks";
import TaskEntryRow from "../TaskEntryRow.svelte";

afterEach(() => cleanup());

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

describe("TaskEntryRow", () => {
  it("running AI task renders cancel button; click fires onAction", async () => {
    const onAction = vi.fn();
    const entry = makeEntry({
      kind: "ai_background",
      title: "Claude Code run",
      status: "running",
      actions: [{ id: "cancel", label: "Cancel", variant: "danger" }],
    });

    const { getByTestId } = render(TaskEntryRow, { props: { entry, onAction } });
    const btn = getByTestId("task-row-action");
    expect(btn.getAttribute("data-action-id")).toBe("cancel");

    await fireEvent.click(btn);
    expect(onAction).toHaveBeenCalledWith("cancel");
  });

  it("failed git task with Retry + Dismiss renders two buttons", () => {
    const entry = makeEntry({
      status: "error",
      errorMessage: "remote rejected",
      actions: [
        { id: "retry", label: "Retry", variant: "primary" },
        { id: "dismiss", label: "Dismiss", variant: "secondary" },
      ],
    });

    const { getAllByTestId, getByTestId } = render(TaskEntryRow, {
      props: { entry, onAction: vi.fn() },
    });

    const actions = getAllByTestId("task-row-action");
    expect(actions).toHaveLength(2);
    expect(actions.map((a) => a.getAttribute("data-action-id"))).toEqual([
      "retry",
      "dismiss",
    ]);

    expect(getByTestId("task-row-error").textContent).toBe("remote rejected");
  });

  it("completed task hides the cancel button", () => {
    const entry = makeEntry({
      status: "success",
      finishedAt: Date.now(),
      actions: [{ id: "dismiss", label: "Dismiss", variant: "secondary" }],
    });

    const { getAllByTestId } = render(TaskEntryRow, {
      props: { entry, onAction: vi.fn() },
    });

    const actions = getAllByTestId("task-row-action");
    expect(actions).toHaveLength(1);
    expect(actions[0].getAttribute("data-action-id")).toBe("dismiss");
  });

  it("determinate progress renders a bar with the correct percent", () => {
    const entry = makeEntry({
      progress: {
        determinate: true,
        current: 25,
        total: 100,
        percent: 25,
      },
    });

    const { getByTestId } = render(TaskEntryRow, {
      props: { entry, onAction: vi.fn() },
    });

    const bar = getByTestId("task-row-progress-bar");
    expect(bar.getAttribute("style")).toContain("--progress: 25%");
    const wrapper = getByTestId("task-row-progress");
    expect(wrapper.getAttribute("data-determinate")).toBe("true");
  });

  it("indeterminate progress renders the animated stripe element", () => {
    const entry = makeEntry({
      progress: { determinate: false },
    });

    const { getByTestId } = render(TaskEntryRow, {
      props: { entry, onAction: vi.fn() },
    });

    const indet = getByTestId("task-row-progress-indet");
    expect(indet).toBeTruthy();
  });

  it("AI-kind with provider hint renders the ProviderIcon", () => {
    const entry = makeEntry({
      kind: "ai_background",
      title: "Claude Code run: fix/refactor-api",
    });

    const { getByTestId } = render(TaskEntryRow, {
      props: { entry, onAction: vi.fn() },
    });

    const iconWrap = getByTestId("task-row-icon");
    // ProviderIcon renders an <img>; the fallback glyph renders a span.
    expect(iconWrap.querySelector("img")).not.toBeNull();
    expect(
      iconWrap.querySelector(".task-row__icon-glyph"),
    ).toBeNull();
  });

  it("git kind renders the nerd-font glyph fallback", () => {
    const entry = makeEntry({ kind: "git_fetch", title: "Fetch origin" });

    const { getByTestId } = render(TaskEntryRow, {
      props: { entry, onAction: vi.fn() },
    });

    const iconWrap = getByTestId("task-row-icon");
    expect(iconWrap.querySelector("img")).toBeNull();
    expect(iconWrap.querySelector(".task-row__icon-glyph")).not.toBeNull();
  });
});
