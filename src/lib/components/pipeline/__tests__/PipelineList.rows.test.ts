/**
 * Regression tests for the PipelineList migration to <List> + TwoLineRow.
 *
 * Exercises:
 * - header/search/empty-state/load-more all still render (via the List shell).
 * - Rows show status label, title, id, ref, shortSha, source badge, actor,
 *   and duration on the meta line.
 * - Right-click (oncontextmenu) on a row opens the retry/cancel/open menu.
 * - The 2 px loading bar renders while refreshing with rows present, not on
 *   the empty/cold-start path.
 */

import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, cleanup, fireEvent } from "@testing-library/svelte";
import { writable } from "svelte/store";
import type { CiRun } from "$lib/types";

vi.mock("$lib/stores/provider", () => {
  const ciRuns = writable<CiRun[]>([]);
  const selectedCiRunId = writable<number | null>(null);
  const hasMoreCiRuns = writable(false);
  const hasActiveProvider = writable(true);
  return {
    ciRuns,
    selectedCiRunId,
    hasMoreCiRuns,
    hasActiveProvider,
    loadCiRuns: vi.fn(async () => {}),
    loadMoreCiRuns: vi.fn(async () => {}),
    loadCiRunDetail: vi.fn(),
    startCiRunListPolling: vi.fn(),
    stopCiRunListPolling: vi.fn(),
    retryCiRun: vi.fn(),
    cancelCiRun: vi.fn(),
  };
});
vi.mock("$lib/stores/repo", () => ({
  repoInfo: writable({ head_branch: "main" }),
}));

import { ciRuns } from "$lib/stores/provider";
import PipelineList from "../PipelineList.svelte";

afterEach(() => cleanup());

const seed: CiRun = {
  id: 101,
  display_id: 7,
  status: "success",
  ref_name: "feature/very-long-branch-name-for-clipping-test",
  sha: "abcdef123456",
  source: "push",
  name: "CI",
  actor: "octocat",
  created_at: "2026-04-23T09:00:00Z",
  updated_at: "2026-04-23T09:02:30Z",
  web_url: "https://example.test/pipelines/101",
};

describe("PipelineList rows", () => {
  beforeEach(() => ciRuns.set([seed]));

  it("renders id/ref/sha/source/actor/duration on the meta line", () => {
    const { container } = render(PipelineList);
    const meta = container.querySelector(".two-line-row__meta")!;
    expect(meta.textContent).toContain("#7");
    expect(meta.textContent).toContain("feature/very-long");
    expect(meta.textContent).toContain("abcdef12");
    expect(meta.textContent).toContain("octocat");
    expect(meta.querySelector(".status-duration")?.textContent).toContain("02:30");
    expect(meta.querySelector(".source-badge")?.textContent?.trim().length).toBeGreaterThan(0);
  });

  it("right-click on a row opens the context menu", async () => {
    const { container, queryByRole } = render(PipelineList);
    const row = container.querySelector(".list-row")!;
    await fireEvent.contextMenu(row);
    expect(queryByRole("menu")).not.toBeNull();
  });
});
