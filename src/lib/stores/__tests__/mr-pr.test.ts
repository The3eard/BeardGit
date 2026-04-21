/**
 * MR/PR store tests — `loadMrPrDetail` error and timeout paths.
 *
 * The Tauri IPC layer is mocked via `vi.mock` so the detail/diff fetches
 * can be stubbed with never-resolving promises to exercise the 15 s
 * timeout wrapper without waiting real time.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

vi.mock("../../api/tauri", () => ({
  // Detail + diff are what the timeout test exercises.
  getMrPrDetail: vi.fn(),
  getMrPrDiff: vi.fn(),
  // All the other mutation / list APIs the store imports are stubbed
  // so `mr-pr.ts` can be evaluated; the tests below never call them.
  listMrPrs: vi.fn(),
  createMrPr: vi.fn(),
  editMrPr: vi.fn(),
  mergeMrPr: vi.fn(),
  closeMrPr: vi.fn(),
  approveMrPr: vi.fn(),
  requestChangesMrPr: vi.fn(),
  addMrPrComment: vi.fn(),
  addMrPrLabels: vi.fn(),
  removeMrPrLabels: vi.fn(),
  addMrPrReviewers: vi.fn(),
  removeMrPrReviewers: vi.fn(),
  markMrPrReady: vi.fn(),
  markMrPrDraft: vi.fn(),
  reopenMrPr: vi.fn(),
  resolveDiscussion: vi.fn(),
  unresolveDiscussion: vi.fn(),
  listLabels: vi.fn(),
  checkoutMrPrLocally: vi.fn(),
}));

import * as api from "../../api/tauri";
import {
  loadMrPrDetail,
  mrPrDetailLoading,
  mrPrDetailError,
  mrPrDetail,
  mrPrDiffFiles,
} from "../mr-pr";

describe("loadMrPrDetail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mrPrDetail.set(null);
    mrPrDiffFiles.set([]);
    mrPrDetailLoading.set(false);
    mrPrDetailError.set(null);
  });

  it("sets mrPrDetailError when the fetch times out", async () => {
    vi.useFakeTimers();
    (api.getMrPrDetail as unknown as ReturnType<typeof vi.fn>).mockReturnValue(
      new Promise(() => {}),
    );
    (api.getMrPrDiff as unknown as ReturnType<typeof vi.fn>).mockReturnValue(
      new Promise(() => {}),
    );

    const load = loadMrPrDetail(18);
    await vi.advanceTimersByTimeAsync(15_000);
    await load;

    expect(get(mrPrDetailLoading)).toBe(false);
    expect(get(mrPrDetailError)).toMatch(/timed out/i);

    vi.useRealTimers();
  });
});
