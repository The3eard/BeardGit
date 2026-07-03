/**
 * Unit tests for `CompareView.svelte` (spec 10 compare view).
 *
 * The compare store's fetch/swap/mode logic is covered by
 * `src/lib/stores/compare.test.ts`; here we mock the store facade so we can
 * drive the *view* wiring in isolation and assert:
 * - the empty state renders until both refs are set,
 * - picking a ref through the header RefPicker calls the store action,
 * - the ahead/behind summary (and the 3-dot merge-base chip) renders,
 * - the 3-dot / 2-dot toggle and the swap button call their actions,
 * - the windowed commit list shows "Load more" only when capped and wires it,
 * - the error state renders the store's error message.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { tick } from "svelte";

// Hoisted store doubles: writable facades + spy actions, mirroring the real
// `$lib/stores/compare` surface the component imports.
const s = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    compareRefA: writable<string | null>(null),
    compareRefB: writable<string | null>(null),
    compareMode: writable<"three-dot" | "two-dot">("three-dot"),
    compareMergeBase: writable<string | null>(null),
    compareCommits: writable<unknown[]>([]),
    compareBehindCount: writable(0),
    compareCommitsCapped: writable(false),
    compareLoadingMore: writable(false),
    compareFiles: writable<unknown[]>([]),
    compareLoading: writable(false),
    compareError: writable<string | null>(null),
    compareSelectedFilePath: writable<string | null>(null),
    compareOpenDiff: writable<unknown | null>(null),
    compareLoadingDiff: writable(false),
    compareDiffError: writable<string | null>(null),
    setCompareRefA: vi.fn(),
    setCompareRefB: vi.fn(),
    swapCompareRefs: vi.fn(),
    setCompareMode: vi.fn(),
    loadMoreCompareCommits: vi.fn(),
    openCompareFileDiff: vi.fn(),
    closeCompareFileDiff: vi.fn(),
    runCompare: vi.fn(),
  };
});

vi.mock("$lib/stores/compare", () => s);
vi.mock("$lib/api/tauri", () => ({
  getBranches: vi.fn().mockResolvedValue([{ name: "main" }]),
  listTags: vi.fn().mockResolvedValue([{ name: "v1.0.0" }]),
}));

import CompareView from "../CompareView.svelte";

/** A CommitInfo-shaped stub. */
function commit(oid: string) {
  return { oid, summary: `sum ${oid}`, body: "", author: "T", email: "t@e", timestamp: 0, parents: [], refs: [] };
}

/** Reset every store double to its default before each test. */
function resetStores() {
  s.compareRefA.set(null);
  s.compareRefB.set(null);
  s.compareMode.set("three-dot");
  s.compareMergeBase.set(null);
  s.compareCommits.set([]);
  s.compareBehindCount.set(0);
  s.compareCommitsCapped.set(false);
  s.compareLoadingMore.set(false);
  s.compareFiles.set([]);
  s.compareLoading.set(false);
  s.compareError.set(null);
  s.compareSelectedFilePath.set(null);
  s.compareOpenDiff.set(null);
  s.compareLoadingDiff.set(false);
  s.compareDiffError.set(null);
}

afterEach(() => cleanup());
beforeEach(() => {
  resetStores();
  Object.values(s).forEach((v) => {
    if (typeof v === "function" && "mockClear" in v) (v as ReturnType<typeof vi.fn>).mockClear();
  });
});

describe("CompareView", () => {
  it("shows the empty state until both refs are set", () => {
    const { getByText } = render(CompareView);
    expect(getByText("Compare two refs")).toBeTruthy();
  });

  it("picking a ref through the Base picker calls setCompareRefA", async () => {
    const { getByLabelText } = render(CompareView);
    const base = getByLabelText("Base") as HTMLInputElement;
    await fireEvent.input(base, { target: { value: "main" } });
    await fireEvent.keyDown(base, { key: "Enter" });
    expect(s.setCompareRefA).toHaveBeenCalledWith("main");
  });

  it("renders the ahead/behind summary once both refs are set", async () => {
    s.compareRefA.set("main");
    s.compareRefB.set("feature");
    s.compareCommits.set([commit("b2"), commit("b1")]);
    s.compareBehindCount.set(1);
    s.compareFiles.set([{ path: "f.txt", status: "added" }]);
    const { getByText } = render(CompareView);
    await tick();
    expect(getByText("2 ahead")).toBeTruthy();
    expect(getByText("1 behind")).toBeTruthy();
  });

  it("shows the merge-base chip only in 3-dot mode", async () => {
    s.compareRefA.set("main");
    s.compareRefB.set("feature");
    s.compareMergeBase.set("base0abc");
    const { getByText, queryByText, rerender } = render(CompareView);
    await tick();
    expect(getByText(/merge-base/)).toBeTruthy();
    // Switching to 2-dot hides it.
    s.compareMode.set("two-dot");
    await rerender({});
    await tick();
    expect(queryByText(/merge-base/)).toBeNull();
  });

  it("clicking the 2-dot toggle calls setCompareMode", async () => {
    const { getByRole } = render(CompareView);
    await fireEvent.click(getByRole("button", { name: "2-dot" }));
    expect(s.setCompareMode).toHaveBeenCalledWith("two-dot");
  });

  it("clicking swap calls swapCompareRefs", async () => {
    const { getByLabelText } = render(CompareView);
    await fireEvent.click(getByLabelText("Swap sides"));
    expect(s.swapCompareRefs).toHaveBeenCalled();
  });

  it("shows Load more only when the commit list is capped and wires it", async () => {
    s.compareRefA.set("main");
    s.compareRefB.set("feature");
    s.compareCommits.set([commit("b1")]);
    const { queryByRole, getByRole, rerender } = render(CompareView);
    await tick();
    // Not capped → no Load more.
    expect(queryByRole("button", { name: "Load more" })).toBeNull();
    s.compareCommitsCapped.set(true);
    await rerender({});
    await tick();
    await fireEvent.click(getByRole("button", { name: "Load more" }));
    expect(s.loadMoreCompareCommits).toHaveBeenCalled();
  });

  it("renders the error state with the store's message", async () => {
    s.compareRefA.set("main");
    s.compareRefB.set("feature");
    s.compareError.set("bad revspec");
    const { getByText } = render(CompareView);
    await tick();
    expect(getByText("Compare failed")).toBeTruthy();
    expect(getByText("bad revspec")).toBeTruthy();
  });
});
