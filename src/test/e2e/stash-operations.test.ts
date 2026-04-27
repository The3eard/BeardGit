/**
 * E2E: Stash management workflow
 *
 * Tests loadStashes, selectStash, doStashPush, doStashPop, doStashApply,
 * doStashDrop, and clearStashState via the stashes store with mocked IPC.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { invokeMock, mockInvokeResponse } from "../setup";
import type { StashEntry, FileDiff } from "$lib/types";

import {
  stashes,
  selectedStashIndex,
  selectedStashDiff,
  loadStashes,
  selectStash,
  doStashPush,
  doStashApply,
  doStashPop,
  doStashDrop,
  clearStashState,
} from "$lib/stores/stashes";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MOCK_STASHES: StashEntry[] = [
  {
    index: 0,
    message: "WIP: auth feature",
    branch: "feature/auth",
    timestamp: 1700001000,
    oid: "stash0oid",
  },
  {
    index: 1,
    message: "WIP: bug fix",
    branch: "main",
    timestamp: 1700000000,
    oid: "stash1oid",
  },
];

const MOCK_DIFF: FileDiff[] = [
  {
    path: "src/auth.ts",
    old_path: null,
    status: "modified",
    hunks: [],
    additions: 8,
    deletions: 3,
  },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("stash operations workflow", () => {
  beforeEach(() => {
    clearStashState();
  });

  // ── loadStashes ──────────────────────────────────────────────────────

  it("loadStashes populates stashes store", async () => {
    mockInvokeResponse("stash_entries", MOCK_STASHES);

    await loadStashes();

    const list = get(stashes);
    expect(list).toHaveLength(2);
    expect(list[0].message).toBe("WIP: auth feature");
    expect(list[0].branch).toBe("feature/auth");
  });

  it("loadStashes sets empty list when no stashes exist", async () => {
    mockInvokeResponse("stash_entries", []);

    await loadStashes();

    expect(get(stashes)).toHaveLength(0);
  });

  it("loadStashes clears selection when selected stash no longer exists", async () => {
    stashes.set(MOCK_STASHES);
    selectedStashIndex.set(1);

    // Return only stash 0 after a pop/drop
    mockInvokeResponse("stash_entries", [MOCK_STASHES[0]]);

    await loadStashes();

    expect(get(selectedStashIndex)).toBeNull();
    expect(get(selectedStashDiff)).toBeNull();
  });

  it("loadStashes keeps selection when selected stash still exists", async () => {
    stashes.set(MOCK_STASHES);
    selectedStashIndex.set(0);

    mockInvokeResponse("stash_entries", MOCK_STASHES);

    await loadStashes();

    expect(get(selectedStashIndex)).toBe(0);
  });

  // ── selectStash ──────────────────────────────────────────────────────

  it("selectStash sets selectedStashIndex and loads diff", async () => {
    mockInvokeResponse("stash_show_parsed", MOCK_DIFF);

    await selectStash(0);

    expect(get(selectedStashIndex)).toBe(0);
    const diff = get(selectedStashDiff);
    expect(diff).not.toBeNull();
    expect(diff!).toHaveLength(1);
    expect(diff![0].path).toBe("src/auth.ts");
  });

  it("selectStash clears diff before loading new one", async () => {
    selectedStashDiff.set(MOCK_DIFF);

    const pendingMock = new Promise<FileDiff[]>((resolve) =>
      setTimeout(() => resolve([]), 50),
    );
    mockInvokeResponse("stash_show_parsed", () => pendingMock);

    selectStash(1);

    // Diff should be cleared synchronously
    expect(get(selectedStashDiff)).toBeNull();

    // Wait for the async to settle
    await pendingMock;
  });

  it("selectStash last-wins: does not apply diff for stale selection", async () => {
    // Stash 0 diff takes a long time
    let resolveStash0!: (v: FileDiff[]) => void;
    const stash0Promise = new Promise<FileDiff[]>((r) => (resolveStash0 = r));

    // First call stash 0 (slow)
    mockInvokeResponse("stash_show_parsed", (args: Record<string, unknown> | undefined) => {
      if (args?.index === 0) return stash0Promise;
      return Promise.resolve(MOCK_DIFF);
    });

    const p0 = selectStash(0);
    // Immediately select stash 1 (fast path returns MOCK_DIFF)
    const p1 = selectStash(1);

    await p1; // stash 1 resolves immediately

    // Stash 1 should be selected now with its diff
    expect(get(selectedStashIndex)).toBe(1);

    // Now resolve stash 0 late — it should be discarded
    resolveStash0([{ path: "stale.ts", old_path: null, status: "modified", hunks: [], additions: 0, deletions: 0 }]);
    await p0;

    // The diff must still be stash 1's (not stash 0's stale result)
    const diff = get(selectedStashDiff);
    expect(diff).not.toBeNull();
    expect(diff![0].path).toBe("src/auth.ts");
  });

  // ── doStashPush ──────────────────────────────────────────────────────
  //
  // Stash-list refresh is now driven by the `project-mutated` event
  // (see mutations.ts) rather than an inline `loadStashes()` call.
  // These tests therefore assert the IPC invocation only; the refresh
  // path is covered by mutations.test.ts.

  it("doStashPush invokes stash_push IPC with the message", async () => {
    mockInvokeResponse("stash_push", "stash{0}");

    await doStashPush("WIP: new work");

    const call = invokeMock.mock.calls.find((c) => c[0] === "stash_push");
    expect(call).toBeDefined();
    expect(call?.[1]).toEqual({ message: "WIP: new work" });
  });

  it("doStashPush accepts null message", async () => {
    mockInvokeResponse("stash_push", "stash{0}");

    await doStashPush(null);

    const call = invokeMock.mock.calls.find((c) => c[0] === "stash_push");
    expect(call?.[1]).toEqual({ message: null });
  });

  // ── doStashPop ───────────────────────────────────────────────────────

  it("doStashPop invokes stash_pop IPC with the index", async () => {
    stashes.set(MOCK_STASHES);
    mockInvokeResponse("stash_pop", "Applied stash{0}");

    await doStashPop(0);

    const call = invokeMock.mock.calls.find((c) => c[0] === "stash_pop");
    expect(call?.[1]).toEqual({ index: 0 });
  });

  // ── doStashApply ──────────────────────────────────────────────────────

  it("doStashApply invokes stash_apply IPC with the index", async () => {
    stashes.set(MOCK_STASHES);
    mockInvokeResponse("stash_apply", "Applied stash{0}");

    await doStashApply(0);

    const call = invokeMock.mock.calls.find((c) => c[0] === "stash_apply");
    expect(call?.[1]).toEqual({ index: 0 });
  });

  // ── doStashDrop ───────────────────────────────────────────────────────

  it("doStashDrop invokes stash_drop IPC with the index", async () => {
    stashes.set(MOCK_STASHES);
    mockInvokeResponse("stash_drop", "Dropped stash{1}");

    await doStashDrop(1);

    const call = invokeMock.mock.calls.find((c) => c[0] === "stash_drop");
    expect(call?.[1]).toEqual({ index: 1 });
  });

  // ── clearStashState ───────────────────────────────────────────────────

  it("clearStashState resets all stash state", () => {
    stashes.set(MOCK_STASHES);
    selectedStashIndex.set(0);
    selectedStashDiff.set(MOCK_DIFF);

    clearStashState();

    expect(get(stashes)).toHaveLength(0);
    expect(get(selectedStashIndex)).toBeNull();
    expect(get(selectedStashDiff)).toBeNull();
  });
});
