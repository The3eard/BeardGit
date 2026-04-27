/**
 * Releases store tests — `selectRelease` timeout path.
 *
 * Stubs `getReleaseDetail` with a never-resolving promise so the
 * 15 s `withTimeout` wrapper can be exercised under fake timers
 * without waiting real wall-clock time. Asserts that the error
 * store is set and loading is cleared when the timer wins the race.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

vi.mock("../../api/tauri", () => ({
  // Only `getReleaseDetail` is exercised in these tests; the rest are
  // stubbed so `releases.ts` can evaluate without throwing on import.
  listReleases: vi.fn(),
  getReleaseDetail: vi.fn(),
  createRelease: vi.fn(),
  editRelease: vi.fn(),
  deleteRelease: vi.fn(),
  publishRelease: vi.fn(),
  uploadReleaseAsset: vi.fn(),
  deleteReleaseAsset: vi.fn(),
  createTagAndRelease: vi.fn(),
}));

import * as api from "../../api/tauri";
import {
  selectRelease,
  releaseDetailLoading,
  releaseDetailError,
  releaseDetail,
  selectedReleaseTag,
} from "../releases";

describe("selectRelease", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    selectedReleaseTag.set(null);
    releaseDetail.set(null);
    releaseDetailLoading.set(false);
    releaseDetailError.set(null);
  });

  it("sets releaseDetailError when the fetch times out", async () => {
    vi.useFakeTimers();
    (api.getReleaseDetail as unknown as ReturnType<typeof vi.fn>).mockReturnValue(
      new Promise(() => {}),
    );

    selectRelease("v1.0.0");
    await vi.advanceTimersByTimeAsync(15_000);
    // Flush any queued microtasks in the .catch/.finally chain.
    await Promise.resolve();
    await Promise.resolve();

    expect(get(releaseDetailLoading)).toBe(false);
    expect(get(releaseDetailError)).toMatch(/timed out/i);

    vi.useRealTimers();
  });
});
