/**
 * Unit tests for the compare store (spec 10): fetch, swap, and mode toggle.
 *
 * Drives the store through the mocked Tauri IPC (`mockInvokeResponse`) and
 * asserts the CompareSlice-backed facades end up in the right shape.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { mockInvokeResponse, clearInvokeMocks, invokeMock } from "../../test/setup";
import { __resetRepoStateForTests, createRepoState, setActiveRepoPath } from "./repo-state";
import {
  compareRefA,
  compareRefB,
  compareMode,
  compareMergeBase,
  compareCommits,
  compareBehindCount,
  compareCommitsCapped,
  compareFiles,
  compareLoading,
  compareError,
  compareOpenDiff,
  setCompareRefA,
  setCompareRefB,
  swapCompareRefs,
  setCompareMode,
  runCompare,
  loadMoreCompareCommits,
  openCompareFileDiff,
  openCompare,
  COMPARE_PAGE_LIMIT,
} from "./compare";

/** A CommitInfo-shaped stub with the given oid. */
function commit(oid: string) {
  return {
    oid,
    summary: `summary ${oid}`,
    body: "",
    author: "T",
    email: "t@e",
    timestamp: 0,
    parents: [],
    refs: [],
  };
}

/** Wire up a happy-path backend: merge-base, files, ahead (a..b), behind (b..a). */
function mockCompareBackend(opts?: {
  ahead?: unknown[];
  behind?: unknown[];
  files?: unknown[];
  mergeBase?: string | null;
}) {
  const ahead = opts?.ahead ?? [commit("b2"), commit("b1")];
  const behind = opts?.behind ?? [commit("a1")];
  const files = opts?.files ?? [{ path: "feat.txt", status: "added" }];
  mockInvokeResponse("get_merge_base", opts?.mergeBase ?? "base0");
  mockInvokeResponse("get_diff_between_commits", files);
  mockInvokeResponse("get_commits_between", (args: Record<string, unknown>) => {
    // Direction-aware by ref name: main→feature is the ahead set, the reverse
    // is the behind set (so a swap correctly exchanges the two).
    if (args.from === "main" && args.to === "feature") return ahead;
    if (args.from === "feature" && args.to === "main") return behind;
    return [];
  });
}

describe("compare store", () => {
  beforeEach(() => {
    clearInvokeMocks();
    __resetRepoStateForTests();
  });

  it("runCompare loads merge-base, files, ahead list, and behind count", async () => {
    mockCompareBackend();
    compareRefA.set("main");
    compareRefB.set("feature");
    await runCompare();

    expect(get(compareMergeBase)).toBe("base0");
    expect(get(compareFiles)).toEqual([{ path: "feat.txt", status: "added" }]);
    expect(get(compareCommits).map((c) => c.oid)).toEqual(["b2", "b1"]);
    expect(get(compareBehindCount)).toBe(1);
    expect(get(compareLoading)).toBe(false);
    expect(get(compareError)).toBeNull();
  });

  it("is a no-op when a ref is missing", async () => {
    mockCompareBackend();
    compareRefA.set("main");
    compareRefB.set(null);
    await runCompare();
    expect(invokeMock).not.toHaveBeenCalledWith("get_merge_base", expect.anything());
  });

  it("three-dot diffs from the merge-base; two-dot diffs from A", async () => {
    mockCompareBackend();
    // Capture the "from" endpoint passed to get_diff_between_commits.
    let lastFrom: unknown;
    mockInvokeResponse("get_diff_between_commits", (args: Record<string, unknown>) => {
      lastFrom = args.fromOid;
      return [];
    });

    compareRefA.set("main");
    compareRefB.set("feature");
    await runCompare(); // default three-dot
    expect(get(compareMode)).toBe("three-dot");
    expect(lastFrom).toBe("base0"); // merge-base

    await setCompareMode("two-dot");
    expect(get(compareMode)).toBe("two-dot");
    expect(lastFrom).toBe("main"); // direct A..B
  });

  it("swap flips the refs and re-runs (ahead/behind swap)", async () => {
    mockCompareBackend();
    compareRefA.set("main");
    compareRefB.set("feature");
    await runCompare();
    expect(get(compareCommits).map((c) => c.oid)).toEqual(["b2", "b1"]);

    await swapCompareRefs();
    expect(get(compareRefA)).toBe("feature");
    expect(get(compareRefB)).toBe("main");
    // Now "ahead" is main..feature reversed → the former "behind" set.
    expect(get(compareCommits).map((c) => c.oid)).toEqual(["a1"]);
    expect(get(compareBehindCount)).toBe(2);
  });

  it("flags the ahead list as capped at the page limit", async () => {
    const full = Array.from({ length: COMPARE_PAGE_LIMIT }, (_, i) => commit(`c${i}`));
    mockCompareBackend({ ahead: full });
    compareRefA.set("main");
    compareRefB.set("feature");
    await runCompare();
    expect(get(compareCommits)).toHaveLength(COMPARE_PAGE_LIMIT);
    expect(get(compareCommitsCapped)).toBe(true);
  });

  it("loadMore appends the next page anchored on the last OID", async () => {
    const page1 = Array.from({ length: COMPARE_PAGE_LIMIT }, (_, i) => commit(`p1-${i}`));
    mockCompareBackend({ ahead: page1 });
    compareRefA.set("main");
    compareRefB.set("feature");
    await runCompare();

    const page2 = [commit("p2-0")];
    let sawAnchor: unknown;
    mockInvokeResponse("get_commits_between", (args: Record<string, unknown>) => {
      sawAnchor = args.anchor;
      return page2;
    });
    await loadMoreCompareCommits();
    expect(sawAnchor).toBe(`p1-${COMPARE_PAGE_LIMIT - 1}`);
    expect(get(compareCommits)).toHaveLength(COMPARE_PAGE_LIMIT + 1);
    expect(get(compareCommitsCapped)).toBe(false);
  });

  it("openCompareFileDiff loads both sides into the panel", async () => {
    mockCompareBackend();
    mockInvokeResponse("get_file_at_commit", (args: Record<string, unknown>) => ({
      kind: "text",
      data: `content-of-${args.oid}`,
    }));
    compareRefA.set("main");
    compareRefB.set("feature");
    await runCompare();

    await openCompareFileDiff("feat.txt");
    const diff = get(compareOpenDiff);
    expect(diff?.filename).toBe("feat.txt");
    // new side = ref B (feature); old side = three-dot "from" = merge-base.
    expect(diff?.newContent).toBe("content-of-feature");
    expect(diff?.oldContent).toBe("content-of-base0");
  });

  it("setCompareRefB triggers a compare when A is already set", async () => {
    mockCompareBackend();
    setCompareRefA("main");
    expect(get(compareCommits)).toEqual([]); // B still missing → no fetch
    await setCompareRefB("feature");
    expect(get(compareCommits).map((c) => c.oid)).toEqual(["b2", "b1"]);
  });

  it("openCompare pre-fills refs and switches state", () => {
    openCompare("main", null);
    expect(get(compareRefA)).toBe("main");
    expect(get(compareRefB)).toBeNull();
  });

  it("a compare started in repo A lands in A even if the user switches to B mid-flight", async () => {
    const A = createRepoState("/repo/a");
    const B = createRepoState("/repo/b");

    // Deferred merge-base so we can switch tabs while the compare is in flight.
    let resolveMergeBase!: (v: string) => void;
    mockInvokeResponse(
      "get_merge_base",
      () =>
        new Promise<string>((resolve) => {
          resolveMergeBase = resolve;
        }),
    );
    mockInvokeResponse("get_diff_between_commits", [{ path: "feat.txt", status: "added" }]);
    mockInvokeResponse("get_commits_between", (args: Record<string, unknown>) => {
      if (args.from === "main" && args.to === "feature") return [commit("b2"), commit("b1")];
      if (args.from === "feature" && args.to === "main") return [commit("a1")];
      return [];
    });

    // Start the compare in repo A.
    setActiveRepoPath("/repo/a");
    A.compare.refA.set("main");
    A.compare.refB.set("feature");
    const pending = runCompare();

    // Switch to repo B before A's compare resolves.
    setActiveRepoPath("/repo/b");

    // Resolve the deferred merge-base → A's compare runs to completion.
    resolveMergeBase("base0");
    await pending;

    // A's slice got the data it asked for…
    expect(get(A.compare.mergeBase)).toBe("base0");
    expect(get(A.compare.commits).map((c) => c.oid)).toEqual(["b2", "b1"]);
    expect(get(A.compare.behindCount)).toBe(1);
    expect(get(A.compare.files)).toEqual([{ path: "feat.txt", status: "added" }]);
    expect(get(A.compare.loading)).toBe(false);

    // …and B's slice is completely untouched (no cross-repo bleed).
    expect(get(B.compare.mergeBase)).toBeNull();
    expect(get(B.compare.commits)).toEqual([]);
    expect(get(B.compare.behindCount)).toBe(0);
    expect(get(B.compare.files)).toEqual([]);
    expect(get(B.compare.loading)).toBe(false);
  });

  it("loadMore started in repo A lands in A even if the user switches to B mid-flight", async () => {
    const A = createRepoState("/repo/a");
    const B = createRepoState("/repo/b");

    // A already has a page of ahead commits loaded.
    setActiveRepoPath("/repo/a");
    A.compare.refA.set("main");
    A.compare.refB.set("feature");
    A.compare.commits.set([commit("p1-0"), commit("p1-1")]);

    // B shares the same refs, so the old refs-changed guard would NOT catch the
    // cross-repo write — only capturing the slice does.
    B.compare.refA.set("main");
    B.compare.refB.set("feature");

    // Deferred next page so we can switch tabs while loadMore is in flight.
    let resolvePage!: (v: unknown[]) => void;
    mockInvokeResponse(
      "get_commits_between",
      () =>
        new Promise<unknown[]>((resolve) => {
          resolvePage = resolve;
        }),
    );

    // Start loadMore in repo A.
    const pending = loadMoreCompareCommits();

    // Switch to repo B before the page resolves.
    setActiveRepoPath("/repo/b");

    // Resolve the deferred page → A's loadMore runs to completion.
    resolvePage([commit("p2-0")]);
    await pending;

    // A's slice got the appended page…
    expect(get(A.compare.commits).map((c) => c.oid)).toEqual(["p1-0", "p1-1", "p2-0"]);
    expect(get(A.compare.loadingMore)).toBe(false);

    // …and B's slice is untouched (no cross-repo bleed).
    expect(get(B.compare.commits)).toEqual([]);
    expect(get(B.compare.loadingMore)).toBe(false);
  });
});
