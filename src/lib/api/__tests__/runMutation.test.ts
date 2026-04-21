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
}));

vi.mock("$lib/stores/toast", () => ({ addToast: mocks.addToast }));
vi.mock("$lib/stores/taskRunner", () => ({
  taskRunner: {
    begin: mocks.begin,
    complete: mocks.complete,
    createAdhoc: mocks.createAdhoc,
  },
}));

import { runMutation } from "../runMutation";
const { addToast, begin, complete, createAdhoc } = mocks;

beforeEach(() => {
  addToast.mockReset();
  begin.mockReset().mockReturnValue("task-1");
  complete.mockReset();
  createAdhoc.mockReset().mockReturnValue("adhoc-1");
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
});
