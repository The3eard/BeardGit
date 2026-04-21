/**
 * `runMutation` tests — verify the façade's toast + task policy.
 *
 * The five cases below cover the contract the call-site migrations in
 * Tasks 7.2–7.4 rely on:
 *
 *  1. Success path fires a 5 s auto-dismiss toast and resolves with the
 *     invoke's return value untouched (generic preservation).
 *  2. Failure path fires a sticky (`duration: null`) error toast whose
 *     body carries only the first line of a multi-line error, then
 *     rethrows the original error.
 *  3. Omitting `successToast` suppresses the success toast entirely —
 *     the silent-set ops (stage / unstage / discard) rely on this.
 *  4. `trackAsTask: true` funnels through `taskRunner.begin` and
 *     `taskRunner.complete` so long-ops show up in the Tasks popover.
 *  5. When the mutation is *not* tracked as a task, a failure still
 *     produces an ad-hoc task record so the "See details" toast action
 *     has an entry to open in the drawer.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

// `vi.mock` is hoisted to the top of the file, so the mock factories
// cannot close over module-level `const`s — we use `vi.hoisted` to
// create the spies *before* the mock factories reference them.
const mocks = vi.hoisted(() => ({
  addToast: vi.fn(),
  begin: vi.fn<(kind: string) => string>().mockReturnValue("task-1"),
  complete: vi.fn(),
  createAdhoc: vi.fn<(kind: string, err: unknown) => string>().mockReturnValue(
    "adhoc-1",
  ),
  openTasksPopover: vi.fn<(id?: string) => void>(),
}));

vi.mock("$lib/stores/toast", () => ({ addToast: mocks.addToast }));
vi.mock("$lib/stores/taskRunner", () => ({
  taskRunner: {
    begin: mocks.begin,
    complete: mocks.complete,
    createAdhoc: mocks.createAdhoc,
  },
}));
vi.mock("$lib/stores/tasksPopover", () => ({
  openTasksPopover: mocks.openTasksPopover,
}));

import { runMutation } from "../runMutation";
const { addToast, begin, complete, createAdhoc, openTasksPopover } = mocks;

beforeEach(() => {
  addToast.mockReset();
  begin.mockReset().mockReturnValue("task-1");
  complete.mockReset();
  createAdhoc.mockReset().mockReturnValue("adhoc-1");
  openTasksPopover.mockReset();
});

describe("runMutation", () => {
  it("fires success toast + resolves", async () => {
    const result = await runMutation({
      kind: "commit",
      invoke: async () => "oid",
      successToast: (r) => `Committed — ${r}`,
      failureToastPrefix: "Commit failed",
    });
    expect(result).toBe("oid");
    expect(addToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: "success", message: "Committed — oid" }),
    );
  });

  it("fires sticky failure toast + rethrows", async () => {
    const err = new Error("boom\nsecond line");
    await expect(
      runMutation({
        kind: "push",
        invoke: async () => {
          throw err;
        },
        failureToastPrefix: "Push failed",
      }),
    ).rejects.toBe(err);
    expect(addToast).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "error",
        message: "Push failed — boom",
        duration: null,
      }),
    );
    // Sticky error toasts carry the "See details" escalation action.
    const toastCall = addToast.mock.calls[0]?.[0] as
      | { actions?: Array<{ label: string; onclick: () => void }> }
      | undefined;
    expect(toastCall?.actions).toHaveLength(1);
    expect(toastCall?.actions?.[0]?.label).toBe("See details");
    // Invoking the action opens the popover pre-selected on the ad-hoc
    // error entry the failure path just created.
    toastCall?.actions?.[0]?.onclick();
    expect(openTasksPopover).toHaveBeenCalledWith("adhoc-1");
  });

  it("skips success toast when omitted (silent-set)", async () => {
    await runMutation({
      kind: "stage",
      invoke: async () => undefined,
      failureToastPrefix: "Stage failed",
    });
    expect(addToast).not.toHaveBeenCalled();
  });

  it("tracks as a task when trackAsTask=true", async () => {
    await runMutation({
      kind: "fetch",
      invoke: async () => "ok",
      successToast: () => "Fetched",
      failureToastPrefix: "Fetch failed",
      trackAsTask: true,
    });
    expect(begin).toHaveBeenCalledWith("fetch");
    expect(complete).toHaveBeenCalledWith("task-1", { ok: true });
  });

  it("creates an ad-hoc task on failure when not tracked", async () => {
    const err = new Error("bad");
    await expect(
      runMutation({
        kind: "commit",
        invoke: async () => {
          throw err;
        },
        failureToastPrefix: "Commit failed",
      }),
    ).rejects.toBe(err);
    expect(createAdhoc).toHaveBeenCalledWith("commit", err);
  });

  it("routes the See details action to the tracked task id", async () => {
    const err = new Error("fetch blew up");
    await expect(
      runMutation({
        kind: "fetch",
        invoke: async () => {
          throw err;
        },
        failureToastPrefix: "Fetch failed",
        trackAsTask: true,
      }),
    ).rejects.toBe(err);
    const toastCall = addToast.mock.calls[0]?.[0] as
      | { actions?: Array<{ label: string; onclick: () => void }> }
      | undefined;
    toastCall?.actions?.[0]?.onclick();
    // Tracked failures route to the task id returned by `begin`, not a
    // fresh ad-hoc record — the Tasks popover has one canonical row.
    expect(openTasksPopover).toHaveBeenCalledWith("task-1");
    expect(createAdhoc).not.toHaveBeenCalled();
  });
});
