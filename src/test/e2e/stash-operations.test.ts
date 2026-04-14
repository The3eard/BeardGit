/**
 * E2E: Stash management workflow
 *
 * Tests loadStashes, selectStash, doStashPush, doStashPop, doStashApply,
 * doStashDrop, and clearStashState via the stashes store with mocked IPC.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { mockInvokeResponse } from "../setup";
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

  it("doStashPush calls stash_push and refreshes list", async () => {
    mockInvokeResponse("stash_push", "stash{0}");
    const updated: StashEntry[] = [
      { index: 0, message: "WIP: new work", branch: "develop", timestamp: 1700002000, oid: "newstash" },
      ...MOCK_STASHES.map((s) => ({ ...s, index: s.index + 1 })),
    ];
    mockInvokeResponse("stash_entries", updated);

    await doStashPush("WIP: new work");

    expect(get(stashes)).toHaveLength(3);
    expect(get(stashes)[0].message).toBe("WIP: new work");
  });

  it("doStashPush accepts null message", async () => {
    mockInvokeResponse("stash_push", "stash{0}");
    mockInvokeResponse("stash_entries", MOCK_STASHES);

    await doStashPush(null);

    expect(get(stashes)).toHaveLength(2);
  });

  // ── doStashPop ───────────────────────────────────────────────────────

  it("doStashPop calls stash_pop and refreshes list", async () => {
    stashes.set(MOCK_STASHES);
    mockInvokeResponse("stash_pop", "Applied stash{0}");
    mockInvokeResponse("stash_entries", [MOCK_STASHES[1]]);

    await doStashPop(0);

    expect(get(stashes)).toHaveLength(1);
    expect(get(stashes)[0].index).toBe(1);
  });

  // ── doStashApply ──────────────────────────────────────────────────────

  it("doStashApply calls stash_apply and refreshes list", async () => {
    stashes.set(MOCK_STASHES);
    mockInvokeResponse("stash_apply", "Applied stash{0}");
    mockInvokeResponse("stash_entries", MOCK_STASHES); // apply keeps the stash

    await doStashApply(0);

    // stash list unchanged (apply doesn't remove)
    expect(get(stashes)).toHaveLength(2);
  });

  // ── doStashDrop ───────────────────────────────────────────────────────

  it("doStashDrop calls stash_drop and refreshes list", async () => {
    stashes.set(MOCK_STASHES);
    mockInvokeResponse("stash_drop", "Dropped stash{1}");
    mockInvokeResponse("stash_entries", [MOCK_STASHES[0]]);

    await doStashDrop(1);

    expect(get(stashes)).toHaveLength(1);
    expect(get(stashes)[0].index).toBe(0);
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
