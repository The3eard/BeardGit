/**
 * Unit tests for `watchCloneTask` (`clone.ts`).
 *
 * The clone runs as a task, so the tab can only be opened once that task
 * reports success. Three things are worth pinning:
 *
 * - A running task opens nothing (the point of the whole change).
 * - Success opens the tab exactly once, at the path validation derived.
 * - Failure reports and opens nothing.
 *
 * Plus the synchronous-first-callback case: `subscribe` fires immediately
 * with the current value, so a task that is *already* terminal has to work
 * without touching an uninitialised `unsubscribe`.
 *
 * Each test uses a distinct task id. A watcher lives until its own task goes
 * terminal, so one left running by an earlier test is still subscribed to the
 * shared `tasksStore` — which mirrors production, where every clone gets its
 * own id, and keeps the cases from firing each other's assertions.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TaskEntry } from "../../types/tasks";

const openProjectTab = vi.fn();
const addToast = vi.fn();

vi.mock("../projects", () => ({
  openProjectTab: (path: string) => openProjectTab(path),
}));
vi.mock("../toast", () => ({
  addToast: (t: unknown) => addToast(t),
}));
vi.mock("$lib/paraglide/messages", () => ({
  clone_dialog_success_toast: ({ name, path }: { name: string; path: string }) =>
    `Cloned ${name} into ${path}`,
  clone_dialog_error_clone: ({ message }: { message: string }) =>
    `Clone failed: ${message}`,
}));

const { tasksStore } = await import("../tasks");
const { watchCloneTask } = await import("../clone");

function entry(id: string, status: TaskEntry["status"], errorMessage?: string): TaskEntry {
  return {
    id,
    kind: "git_clone",
    title: "Clone repo",
    startedAt: 1,
    status,
    errorMessage,
    actions: [],
  };
}

describe("watchCloneTask", () => {
  beforeEach(() => {
    tasksStore.set([]);
    openProjectTab.mockClear();
    addToast.mockClear();
  });

  it("opens nothing while the clone is still running", () => {
    watchCloneTask(1, "/tmp/repo", "repo");
    tasksStore.set([entry("1", "running")]);

    expect(openProjectTab).not.toHaveBeenCalled();
    expect(addToast).not.toHaveBeenCalled();
  });

  it("opens the tab once the task succeeds", () => {
    watchCloneTask(2, "/tmp/repo", "repo");
    tasksStore.set([entry("2", "running")]);
    tasksStore.set([entry("2", "success")]);

    expect(openProjectTab).toHaveBeenCalledExactlyOnceWith("/tmp/repo");
    expect(addToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: "success" }),
    );
  });

  it("stops watching after the first terminal state", () => {
    watchCloneTask(3, "/tmp/repo", "repo");
    tasksStore.set([entry("3", "success")]);
    // A later re-emission of the same id must not open a second tab.
    tasksStore.set([entry("3", "success")]);

    expect(openProjectTab).toHaveBeenCalledTimes(1);
  });

  it("reports a failed clone and opens nothing", () => {
    watchCloneTask(9, "/tmp/repo", "repo");
    tasksStore.set([entry("9", "error", "auth failed")]);

    expect(openProjectTab).not.toHaveBeenCalled();
    expect(addToast).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "error",
        message: "Clone failed: auth failed",
      }),
    );
  });

  it("says nothing when the user cancels", () => {
    watchCloneTask(9, "/tmp/repo", "repo");
    tasksStore.set([entry("9", "cancelled")]);

    expect(openProjectTab).not.toHaveBeenCalled();
    expect(addToast).not.toHaveBeenCalled();
  });

  it("handles a task that is already terminal when the watch starts", () => {
    // `subscribe` fires synchronously, so this exercises the path where the
    // first callback settles before `unsubscribe` has been assigned.
    tasksStore.set([entry("11", "success")]);
    watchCloneTask(11, "/tmp/repo", "repo");

    expect(openProjectTab).toHaveBeenCalledExactlyOnceWith("/tmp/repo");
  });

  it("ignores other tasks running at the same time", () => {
    watchCloneTask(13, "/tmp/repo", "repo");
    tasksStore.set([entry("12", "success"), entry("13", "running")]);

    expect(openProjectTab).not.toHaveBeenCalled();
  });
});
