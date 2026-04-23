/**
 * Unit tests for the empty-state copy in `MrPrDetail.svelte`.
 *
 * Phase 7.2 of the Forge Data Fixes plan swaps the "Changed files"
 * section from a blank `<div>` to a neutral localized message when
 * the diff payload is empty (`$mrPrDiffFiles.length === 0`). These
 * assertions lock in:
 *   - With a non-empty PR the file list renders and the empty copy
 *     stays hidden.
 *   - With an empty PR the "No changes in this pull request." copy
 *     renders in place of the file list.
 *
 * The component imports a large store surface from `mr-pr.ts`, so the
 * mocks here stub every identifier the component references. We only
 * care about the reactive pieces the empty-state branch reads:
 *   - `mrPrDetail` — drives whether the content snippet renders at all.
 *   - `mrPrDiffFiles` — drives the empty-state branch.
 *   - `mrPrDetailLoading` / `mrPrDetailError` — must be falsy/null
 *     so `ForgeDetailShell` falls through to the content snippet.
 *   - `selectedMrPrNumber` — read by the retry closure; any value ok.
 * The rest are supplied as no-op writables / vi.fn() to satisfy the
 * destructuring import in the SFC.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type {
  MrPrDetail as MrPrDetailT,
  MrPrDiffFile,
  Label,
} from "$lib/types";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mrPrDetail: writable<MrPrDetailT | null>(null),
    mrPrDetailLoading: writable(false),
    mrPrDetailError: writable<string | null>(null),
    mrPrDiffFiles: writable<MrPrDiffFile[]>([]),
    mrPrDiffLoading: writable(false),
    mrPrDiffError: writable<string | null>(null),
    selectedMrPrNumber: writable<number | null>(null),
    selectedPrFilePath: writable<string | null>(null),
    repoLabels: writable<Label[]>([]),
    repoLabelsLoading: writable(false),
    activeProvider: writable<{ kind: "github" | "gitlab" } | null>({
      kind: "github",
    }),
  };
});

vi.mock("$lib/stores/mr-pr", () => ({
  mrPrDetail: mocks.mrPrDetail,
  mrPrDetailLoading: mocks.mrPrDetailLoading,
  mrPrDetailError: mocks.mrPrDetailError,
  mrPrDiffFiles: mocks.mrPrDiffFiles,
  mrPrDiffLoading: mocks.mrPrDiffLoading,
  mrPrDiffError: mocks.mrPrDiffError,
  selectedMrPrNumber: mocks.selectedMrPrNumber,
  selectedPrFilePath: mocks.selectedPrFilePath,
  repoLabels: mocks.repoLabels,
  repoLabelsLoading: mocks.repoLabelsLoading,
  loadMrPrDetail: vi.fn(),
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
  checkoutMrPrLocally: vi.fn(),
  loadRepoLabels: vi.fn(),
}));

vi.mock("$lib/stores/provider", () => ({
  activeProvider: mocks.activeProvider,
}));

// Tauri plugins used by the SFC — stub them out so the import doesn't
// explode in jsdom.
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

import MrPrDetail from "../MrPrDetail.svelte";

function makeDetail(over: Partial<MrPrDetailT["summary"]> = {}): MrPrDetailT {
  return {
    summary: {
      number: 1,
      title: "test PR",
      state: "open",
      author: "alice",
      source_branch: "feature",
      target_branch: "main",
      url: "https://example.com/pr/1",
      draft: false,
      labels: [],
      reviewers: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      additions: 0,
      deletions: 0,
      changed_files: 0,
      base_sha: "",
      head_sha: "",
      head_repo_url: null,
      ...over,
    },
    body: "",
    comments: [],
    review_status: "pending",
    mergeable: true,
  };
}

beforeEach(() => {
  mocks.mrPrDetail.set(null);
  mocks.mrPrDetailLoading.set(false);
  mocks.mrPrDetailError.set(null);
  mocks.mrPrDiffFiles.set([]);
  mocks.mrPrDiffLoading.set(false);
  mocks.mrPrDiffError.set(null);
  mocks.selectedMrPrNumber.set(1);
  mocks.repoLabels.set([]);
  mocks.repoLabelsLoading.set(false);
  mocks.activeProvider.set({ kind: "github" });
});

afterEach(() => cleanup());

describe("MrPrDetail — empty-state copy", () => {
  it("renders the 'no changes' message when the diff is empty", async () => {
    mocks.mrPrDetail.set(makeDetail());
    mocks.mrPrDiffFiles.set([]);
    const { getByText } = render(MrPrDetail);
    await tick();
    expect(getByText("No changes in this pull request.")).toBeTruthy();
  });

  it("renders the file rows when the diff has entries", async () => {
    mocks.mrPrDetail.set(makeDetail());
    mocks.mrPrDiffFiles.set([
      {
        path: "src/foo.ts",
        old_path: null,
        status: "modified",
        additions: 3,
        deletions: 1,
        patch: null,
      },
    ]);
    const { queryByText, getByText } = render(MrPrDetail);
    await tick();
    expect(queryByText("No changes in this pull request.")).toBeNull();
    expect(getByText("src/foo.ts")).toBeTruthy();
  });
});
