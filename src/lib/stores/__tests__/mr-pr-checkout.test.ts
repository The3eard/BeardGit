/**
 * Tests for the Phase 8.2 MR/PR enhancement store actions.
 *
 * Each test mocks the matching Tauri command via `mockInvokeResponse` and
 * asserts that the store action forwards the right arguments and returns
 * the right value.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";

import {
  checkoutMrPrLocally,
  loadRepoLabels,
  repoLabels,
  repoLabelsLoading,
} from "../mr-pr";
import {
  mockInvokeResponse,
  clearInvokeMocks,
  invokeMock,
} from "../../../test/setup";

beforeEach(() => {
  clearInvokeMocks();
  repoLabels.set([]);
  repoLabelsLoading.set(false);
});

describe("checkoutMrPrLocally", () => {
  it("invokes the checkout_mr_pr_locally command and returns the task id", async () => {
    mockInvokeResponse("checkout_mr_pr_locally", 7);

    const taskId = await checkoutMrPrLocally(42);

    expect(taskId).toBe(7);
    expect(invokeMock).toHaveBeenCalledWith("checkout_mr_pr_locally", {
      number: 42,
    });
  });
});

describe("loadRepoLabels", () => {
  it("populates repoLabels on success", async () => {
    const payload = [
      { name: "bug", color: "d73a4a", description: "Something broken" },
      { name: "docs", color: null, description: null },
    ];
    mockInvokeResponse("list_labels", payload);

    await loadRepoLabels();

    expect(get(repoLabels)).toEqual(payload);
    expect(get(repoLabelsLoading)).toBe(false);
  });

  it("clears repoLabels and stops loading on failure", async () => {
    mockInvokeResponse("list_labels", () => {
      throw new Error("boom");
    });

    await loadRepoLabels();

    expect(get(repoLabels)).toEqual([]);
    expect(get(repoLabelsLoading)).toBe(false);
  });
});
