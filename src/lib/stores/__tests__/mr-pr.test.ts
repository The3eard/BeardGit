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
  mrPrDiffLoading,
  mrPrDiffError,
} from "../mr-pr";

describe("loadMrPrDetail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mrPrDetail.set(null);
    mrPrDiffFiles.set([]);
    mrPrDetailLoading.set(false);
    mrPrDetailError.set(null);
    mrPrDiffLoading.set(false);
    mrPrDiffError.set(null);
  });

  it("sets mrPrDetailError and mrPrDiffError when both fetches time out", async () => {
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
    expect(get(mrPrDiffLoading)).toBe(false);
    expect(get(mrPrDetailError)).toMatch(/timed out/i);
    expect(get(mrPrDiffError)).toMatch(/timed out/i);

    vi.useRealTimers();
  });

  it("renders meta as soon as it lands even if the diff fetch is still hanging", async () => {
    vi.useFakeTimers();
    const fastDetail = {
      summary: {
        number: 18,
        title: "fast meta",
        state: "open" as const,
        author: "me",
        source_branch: "feat/x",
        target_branch: "main",
        url: "https://example.com/pr/18",
        draft: false,
        labels: [],
        reviewers: [],
        created_at: "",
        updated_at: "",
        additions: 0,
        deletions: 0,
        changed_files: 5,
      },
      body: "",
      comments: [],
      review_status: "pending" as const,
      mergeable: true,
    };
    (api.getMrPrDetail as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(
      fastDetail,
    );
    (api.getMrPrDiff as unknown as ReturnType<typeof vi.fn>).mockReturnValue(
      new Promise(() => {}),
    );

    const load = loadMrPrDetail(18);
    // Let the meta promise resolve (microtask queue) without advancing the
    // timeout timer — we want to assert the meta lands first.
    await vi.advanceTimersByTimeAsync(0);
    await Promise.resolve();
    await Promise.resolve();

    expect(get(mrPrDetail)).not.toBeNull();
    expect(get(mrPrDetailLoading)).toBe(false);
    expect(get(mrPrDiffLoading)).toBe(true);

    // Now let the diff timeout fire so the store resolves cleanly.
    await vi.advanceTimersByTimeAsync(15_000);
    await load;

    expect(get(mrPrDiffLoading)).toBe(false);
    expect(get(mrPrDiffError)).toMatch(/timed out/i);
    // Meta state untouched by the diff failure.
    expect(get(mrPrDetail)).not.toBeNull();
    expect(get(mrPrDetailError)).toBeNull();

    vi.useRealTimers();
  });
});
