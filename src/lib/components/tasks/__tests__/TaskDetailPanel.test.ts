/**
 * Unit tests for `TaskDetailPanel.svelte`.
 *
 * Verifies the drill-down view wires every task kind into the correct
 * output source:
 *
 * - Git ops / AI interactive → legacy `taskPanel.taskOutput` by numeric
 *   `TaskId`. A missing buffer triggers a `selectTask(id)` back-fill.
 * - AI background → `aiBackgroundTranscripts` by session id, with an
 *   optional `task_id` fallback to `taskOutput` when both stores hold
 *   lines.
 * - App update → no console output available, the localized "No
 *   output" placeholder renders.
 *
 * Metadata (kind, status, started, error) renders from the `TaskEntry`
 * prop for every kind.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import type { TaskEntry } from "$lib/types/tasks";
import type { AiSession, TaskOutputLine } from "$lib/types";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    taskOutput: writable<Map<number, TaskOutputLine[]>>(new Map()),
    aiBackgroundRuns: writable<Map<string, AiSession>>(new Map()),
    aiBackgroundTranscripts: writable<Map<string, string[]>>(new Map()),
    selectTaskMock: vi.fn(async (_id: number) => {
      void _id;
    }),
  };
});

vi.mock("$lib/stores/taskPanel", () => ({
  taskOutput: mocks.taskOutput,
  selectTask: (id: number) => mocks.selectTaskMock(id),
}));

vi.mock("$lib/stores/aiBackground", () => ({
  aiBackgroundRuns: mocks.aiBackgroundRuns,
  aiBackgroundTranscripts: mocks.aiBackgroundTranscripts,
}));

import TaskDetailPanel from "../TaskDetailPanel.svelte";

function makeEntry(over: Partial<TaskEntry> = {}): TaskEntry {
  return {
    id: "42",
    kind: "git_fetch",
    title: "Fetch origin",
    startedAt: Date.now() - 1000,
    status: "running",
    actions: [],
    ...over,
  };
}

function line(stream: "stdout" | "stderr", text: string): TaskOutputLine {
  return { stream, text };
}

beforeEach(() => {
  mocks.taskOutput.set(new Map());
  mocks.aiBackgroundRuns.set(new Map());
  mocks.aiBackgroundTranscripts.set(new Map());
  mocks.selectTaskMock.mockClear();
});

afterEach(() => cleanup());

describe("TaskDetailPanel — git ops", () => {
  it("renders metadata + the legacy task-output buffer for a numeric id", async () => {
    mocks.taskOutput.set(
      new Map([
        [
          42,
          [line("stdout", "remote: Counting"), line("stderr", "warning: x")],
        ],
      ]),
    );

    const entry = makeEntry({
      id: "42",
      kind: "git_fetch",
      title: "Fetch origin",
    });

    const { getByTestId } = render(TaskDetailPanel, { props: { entry } });
    await tick();

    const output = getByTestId("task-detail-output");
    expect(output.textContent).toContain("remote: Counting");
    expect(output.textContent).toContain("warning: x");
    const stderrLine = output.querySelector(
      '[data-stream="stderr"]',
    ) as HTMLElement | null;
    expect(stderrLine?.textContent).toContain("warning: x");
  });

  it("triggers selectTask back-fill when the local buffer is empty", async () => {
    const entry = makeEntry({ id: "99", kind: "git_push" });
    render(TaskDetailPanel, { props: { entry } });
    await tick();
    expect(mocks.selectTaskMock).toHaveBeenCalledWith(99);
  });

  it("renders no output block when nothing is available — meta header stands alone", async () => {
    // Empty buffer used to render a placeholder zone with an italic
    // "No output" line. That read as a broken state on
    // failure/cancellation rows, so we now omit the block entirely
    // and let the meta header speak for itself.
    const entry = makeEntry({ id: "7", kind: "git_pull" });
    const { queryByTestId, getByTestId } = render(TaskDetailPanel, {
      props: { entry },
    });
    await tick();
    expect(queryByTestId("task-detail-output")).toBeNull();
    // Meta still renders — the panel hasn't gone blank.
    expect(getByTestId("task-detail-kind")).toBeTruthy();
  });

  it("exposes the task kind + status + started labels", () => {
    const entry = makeEntry({
      id: "1",
      kind: "git_fetch",
      status: "running",
      startedAt: Date.now() - 60_000,
    });
    const { getByTestId } = render(TaskDetailPanel, { props: { entry } });

    expect(getByTestId("task-detail-kind").textContent).toMatch(/fetch/i);
    expect(getByTestId("task-detail-status").dataset.status).toBe("running");
    expect(getByTestId("task-detail-started").textContent).toBeTruthy();
  });

  it("surfaces the error message when the task failed", () => {
    const entry = makeEntry({
      id: "1",
      status: "error",
      errorMessage: "remote rejected",
    });
    const { getByTestId } = render(TaskDetailPanel, { props: { entry } });
    expect(getByTestId("task-detail-error").textContent).toContain(
      "remote rejected",
    );
  });
});

describe("TaskDetailPanel — AI background", () => {
  it("reads lines from aiBackgroundTranscripts when no task_id is known", async () => {
    mocks.aiBackgroundTranscripts.set(
      new Map([["sess-1", ["hello", "world"]]]),
    );
    mocks.aiBackgroundRuns.set(
      new Map([
        [
          "sess-1",
          {
            id: "sess-1",
            provider: "claude_code",
            cwd: "/tmp",
            started_at: Date.now(),
            kind: "headless",
            is_active: true,
            worktree_path: "/tmp",
            background_status: { state: "running" },
            task_id: null,
          },
        ],
      ]),
    );

    const entry = makeEntry({
      id: "ai-background:sess-1",
      kind: "ai_background",
      title: "Claude run",
    });
    const { getByTestId } = render(TaskDetailPanel, { props: { entry } });
    await tick();
    expect(getByTestId("task-detail-output").textContent).toContain("hello");
    expect(getByTestId("task-detail-output").textContent).toContain("world");
    // No numeric id to back-fill.
    expect(mocks.selectTaskMock).not.toHaveBeenCalled();
  });

  it("falls back to taskOutput via session.task_id when the coordinator forwarded lines", async () => {
    mocks.aiBackgroundRuns.set(
      new Map([
        [
          "sess-2",
          {
            id: "sess-2",
            provider: "claude_code",
            cwd: "/tmp",
            started_at: Date.now(),
            kind: "headless",
            is_active: true,
            worktree_path: "/tmp",
            background_status: { state: "running" },
            task_id: 7,
          },
        ],
      ]),
    );
    mocks.taskOutput.set(new Map([[7, [line("stdout", "from task id 7")]]]));

    const entry = makeEntry({
      id: "ai-background:sess-2",
      kind: "ai_background",
      title: "Codex run",
    });
    const { getByTestId } = render(TaskDetailPanel, { props: { entry } });
    await tick();
    expect(getByTestId("task-detail-output").textContent).toContain(
      "from task id 7",
    );
    // Back-fill uses the resolved numeric id.
    expect(mocks.selectTaskMock).toHaveBeenCalledWith(7);
  });
});

describe("TaskDetailPanel — app update", () => {
  it("renders meta only without an output block (app_update has no stream)", async () => {
    const entry = makeEntry({
      id: "auto-update",
      kind: "app_update",
      title: "BeardGit 0.2.0",
    });
    const { queryByTestId, getByTestId } = render(TaskDetailPanel, {
      props: { entry },
    });
    await tick();
    expect(queryByTestId("task-detail-output")).toBeNull();
    expect(getByTestId("task-detail-kind").textContent).toMatch(/update/i);
    // No back-fill for app_update.
    expect(mocks.selectTaskMock).not.toHaveBeenCalled();
  });
});

describe("TaskDetailPanel — store isolation", () => {
  it("does not mutate the global taskOutput store", async () => {
    mocks.taskOutput.set(new Map([[1, [line("stdout", "x")]]]));
    const entry = makeEntry({ id: "1" });
    render(TaskDetailPanel, { props: { entry } });
    await tick();
    // Snapshot assertion — the effect only calls selectTask when the
    // buffer is empty, and the buffer is populated here.
    expect(get(mocks.taskOutput).get(1)).toHaveLength(1);
  });
});
