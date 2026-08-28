/**
 * Unit tests for the `fileEditor` store.
 *
 * Mocks the Tauri IPC layer and `runMutation` so the test cases can
 * assert on the call shape (path arg + invoke ordering for save+stage)
 * without needing a backend. Mocks the toast store so `runMutation`'s
 * own dependencies don't blow up under jsdom.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

const mocks = vi.hoisted(() => ({
  readWorkdirFile: vi.fn(),
  writeWorkdirFile: vi.fn(),
  stageFiles: vi.fn(),
  listWorkdirTree: vi.fn(),
  searchWorkdirFiles: vi.fn(),
  createWorkdirPath: vi.fn(),
  renameWorkdirPath: vi.fn(),
  deleteWorkdirPath: vi.fn(),
  addToast: vi.fn(),
  taskBegin: vi.fn(() => "task-1"),
  taskComplete: vi.fn(),
  taskFail: vi.fn(),
}));

vi.mock("$lib/api/tauri", () => ({
  readWorkdirFile: mocks.readWorkdirFile,
  writeWorkdirFile: mocks.writeWorkdirFile,
  stageFiles: mocks.stageFiles,
  listWorkdirTree: mocks.listWorkdirTree,
  searchWorkdirFiles: mocks.searchWorkdirFiles,
  createWorkdirPath: mocks.createWorkdirPath,
  renameWorkdirPath: mocks.renameWorkdirPath,
  deleteWorkdirPath: mocks.deleteWorkdirPath,
}));

vi.mock("$lib/stores/toast", () => ({
  addToast: mocks.addToast,
}));

vi.mock("$lib/stores/taskRunner", () => ({
  taskRunner: {
    begin: mocks.taskBegin,
    complete: mocks.taskComplete,
    fail: mocks.taskFail,
  },
}));

vi.mock("$lib/stores/tasksPopover", () => ({
  openTasksPopover: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import {
  __resetForTests,
  activeTabPath,
  closeTab,
  closeTabsUnder,
  openTab,
  persistTabsForProject,
  refreshTree,
  refreshTreePaths,
  parentDir,
  createPath,
  deletePath,
  renamePath,
  reloadActive,
  renameOpenTab,
  restoreTabsForProject,
  saveActive,
  setActiveTab,
  tabs,
  treeChildren,
  expandedDirs,
  loadingDirs,
  searchResults,
  searchTruncated,
  toggleDirectory,
  loadDirectory,
  resetTree,
  failedDirs,
  searchTree,
  SEARCH_RESULT_CAP,
  updateBuffer,
} from "../fileEditor";

/**
 * Stub `localStorage` for tests — jsdom's built-in implementation is
 * incomplete in this environment (no `.clear` / `.removeItem`). We
 * provide a tiny in-memory shim with the methods the store touches.
 */
const lsStore = new Map<string, string>();
beforeEach(() => {
  vi.clearAllMocks();
  __resetForTests();
  lsStore.clear();
});

vi.stubGlobal("localStorage", {
  getItem: (key: string) => (lsStore.has(key) ? lsStore.get(key)! : null),
  setItem: (key: string, value: string) => {
    lsStore.set(key, value);
  },
  removeItem: (key: string) => {
    lsStore.delete(key);
  },
  clear: () => lsStore.clear(),
  key: (i: number) => Array.from(lsStore.keys())[i] ?? null,
  get length() {
    return lsStore.size;
  },
});

afterEach(() => {
  __resetForTests();
});

describe("fileEditor store", () => {
  describe("openTab", () => {
    it("calls readWorkdirFile and adds the tab", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "hello",
        size: 5,
      });
      await openTab("src/lib/foo.ts");

      expect(mocks.readWorkdirFile).toHaveBeenCalledWith("src/lib/foo.ts");
      const list = get(tabs);
      expect(list).toHaveLength(1);
      expect(list[0].path).toBe("src/lib/foo.ts");
      expect(list[0].name).toBe("foo.ts");
      expect(list[0].diskContent).toBe("hello");
      expect(list[0].bufferContent).toBe("hello");
      expect(list[0].dirty).toBe(false);
      expect(list[0].status).toBe("ok");
      expect(get(activeTabPath)).toBe("src/lib/foo.ts");
    });

    it("focuses an existing tab without re-reading", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "x",
        size: 1,
      });
      await openTab("a.ts");
      mocks.readWorkdirFile.mockClear();

      await openTab("a.ts");
      expect(mocks.readWorkdirFile).not.toHaveBeenCalled();
      expect(get(tabs)).toHaveLength(1);
    });

    it("flags binary files with the binary status", async () => {
      mocks.readWorkdirFile.mockResolvedValue({ kind: "binary", size: 12 });
      await openTab("logo.png");
      const list = get(tabs);
      expect(list[0].status).toBe("binary");
      expect(list[0].size).toBe(12);
    });

    it("flags too-large files with the too_large status", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "too_large",
        size: 5_000_000,
      });
      await openTab("big.bin");
      const list = get(tabs);
      expect(list[0].status).toBe("too_large");
      expect(list[0].size).toBe(5_000_000);
    });
  });

  describe("closeTab", () => {
    it("removes the tab from the list", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "",
        size: 0,
      });
      await openTab("a.ts");
      await openTab("b.ts");
      await closeTab("a.ts");
      const list = get(tabs);
      expect(list.map((t) => t.path)).toEqual(["b.ts"]);
      expect(get(activeTabPath)).toBe("b.ts");
    });

    it("closes the last tab and clears activeTabPath", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "",
        size: 0,
      });
      await openTab("only.ts");
      await closeTab("only.ts");
      expect(get(tabs)).toHaveLength(0);
      expect(get(activeTabPath)).toBeNull();
    });
  });

  describe("saveActive", () => {
    it("calls writeWorkdirFile and updates diskContent", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "old",
        size: 3,
      });
      mocks.writeWorkdirFile.mockResolvedValue(undefined);
      await openTab("note.txt");

      updateBuffer("note.txt", "new content");
      expect(get(tabs)[0].dirty).toBe(true);

      await saveActive();
      expect(mocks.writeWorkdirFile).toHaveBeenCalledWith(
        "note.txt",
        "new content",
      );
      expect(mocks.stageFiles).not.toHaveBeenCalled();
      const list = get(tabs);
      expect(list[0].diskContent).toBe("new content");
      expect(list[0].dirty).toBe(false);
    });

    it("calls writeWorkdirFile AND stageFiles when stage:true", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "x",
        size: 1,
      });
      mocks.writeWorkdirFile.mockResolvedValue(undefined);
      mocks.stageFiles.mockResolvedValue(undefined);
      await openTab("staged.ts");
      updateBuffer("staged.ts", "y");

      await saveActive({ stage: true });
      expect(mocks.writeWorkdirFile).toHaveBeenCalledWith("staged.ts", "y");
      expect(mocks.stageFiles).toHaveBeenCalledWith(["staged.ts"]);
    });

    it("is a no-op when no tab is active", async () => {
      await saveActive();
      expect(mocks.writeWorkdirFile).not.toHaveBeenCalled();
    });
  });

  describe("reloadActive", () => {
    it("re-fetches and replaces the buffer with disk content", async () => {
      mocks.readWorkdirFile.mockResolvedValueOnce({
        kind: "text",
        data: "first",
        size: 5,
      });
      await openTab("a.txt");
      updateBuffer("a.txt", "edits");

      mocks.readWorkdirFile.mockResolvedValueOnce({
        kind: "text",
        data: "second",
        size: 6,
      });
      await reloadActive();
      const tab = get(tabs)[0];
      expect(tab.diskContent).toBe("second");
      expect(tab.bufferContent).toBe("second");
      expect(tab.dirty).toBe(false);
      expect(tab.externalChange).toBe(false);
    });
  });

  describe("setActiveTab", () => {
    it("ignores paths that aren't open", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "",
        size: 0,
      });
      await openTab("a.ts");
      setActiveTab("never-opened.ts");
      expect(get(activeTabPath)).toBe("a.ts");
    });
  });

  describe("renameOpenTab + closeTabsUnder", () => {
    it("renameOpenTab updates path and name", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "",
        size: 0,
      });
      await openTab("old/a.ts");
      renameOpenTab("old/a.ts", "old/b.ts");
      const tab = get(tabs)[0];
      expect(tab.path).toBe("old/b.ts");
      expect(tab.name).toBe("b.ts");
      expect(get(activeTabPath)).toBe("old/b.ts");
    });

    it("closeTabsUnder closes every tab beneath a directory", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "",
        size: 0,
      });
      await openTab("dir/a.ts");
      await openTab("dir/b.ts");
      await openTab("other/c.ts");

      closeTabsUnder("dir");
      const paths = get(tabs).map((t) => t.path);
      expect(paths).toEqual(["other/c.ts"]);
    });
  });

  const dir = (path: string) => ({
    path,
    name: path.split("/").pop() ?? path,
    is_directory: true,
    size: null,
  });
  const file = (path: string) => ({
    path,
    name: path.split("/").pop() ?? path,
    is_directory: false,
    size: 0,
  });

  describe("tree listing", () => {
    it("loads the repo root under the empty prefix", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([file("a.ts"), dir("src")]);

      await refreshTree(true);

      expect(mocks.listWorkdirTree).toHaveBeenCalledWith(null, expect.any(Number), true);
      expect(get(treeChildren).get("")).toHaveLength(2);
    });

    it("lists a directory only when it is first expanded", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([dir("src")]);
      await refreshTree(true);
      mocks.listWorkdirTree.mockClear();

      mocks.listWorkdirTree.mockResolvedValueOnce([file("src/a.ts")]);
      await toggleDirectory("src", true);

      expect(mocks.listWorkdirTree).toHaveBeenCalledWith("src", expect.any(Number), true);
      expect(get(expandedDirs).has("src")).toBe(true);
      expect(get(treeChildren).get("src")).toHaveLength(1);
    });

    it("does not re-list a directory the user collapses and re-opens", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([file("src/a.ts")]);
      await toggleDirectory("src", true);
      mocks.listWorkdirTree.mockClear();

      await toggleDirectory("src", true); // collapse
      await toggleDirectory("src", true); // re-open

      expect(get(expandedDirs).has("src")).toBe(true);
      expect(mocks.listWorkdirTree).not.toHaveBeenCalled();
    });

    it("refreshes the root and every open directory", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([file("src/a.ts")]);
      await toggleDirectory("src", true);
      mocks.listWorkdirTree.mockClear();
      mocks.listWorkdirTree.mockResolvedValue([]);

      await refreshTree(true);

      const prefixes = mocks.listWorkdirTree.mock.calls.map((c) => c[0]);
      expect(prefixes).toEqual([null, "src"]);
    });

    /**
     * The reachability bug the caching created: a refresh that skipped
     * collapsed folders, plus an expand that skipped cached ones, left a
     * file that exists on disk with no path to the screen.
     */
    it("forgets a collapsed directory on refresh, so re-opening re-lists it", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([file("docs/a.md")]);
      await toggleDirectory("docs", true);
      await toggleDirectory("docs", true); // collapse — still cached

      mocks.listWorkdirTree.mockResolvedValue([]); // the refresh itself
      await refreshTree(true);
      expect(get(treeChildren).has("docs")).toBe(false);

      // Re-opening now goes back to disk and sees the new file.
      mocks.listWorkdirTree.mockClear();
      mocks.listWorkdirTree.mockResolvedValueOnce([
        file("docs/a.md"),
        file("docs/new.md"),
      ]);
      await toggleDirectory("docs", true);

      expect(mocks.listWorkdirTree).toHaveBeenCalledWith("docs", expect.any(Number), true);
      expect(get(treeChildren).get("docs")).toHaveLength(2);
    });

    it("marks a directory whose listing failed, rather than showing it empty", async () => {
      mocks.listWorkdirTree.mockRejectedValueOnce(new Error("boom"));

      await loadDirectory("src", true);

      expect(get(loadingDirs).has("src")).toBe(false);
      expect(get(failedDirs).has("src")).toBe(true);
      expect(get(treeChildren).has("src")).toBe(false);
    });

    it("clears the failed mark once the directory lists again", async () => {
      mocks.listWorkdirTree.mockRejectedValueOnce(new Error("boom"));
      await loadDirectory("src", true);

      mocks.listWorkdirTree.mockResolvedValueOnce([file("src/a.ts")]);
      await loadDirectory("src", true);

      expect(get(failedDirs).has("src")).toBe(false);
    });

    it("clears the loading flag even for a listing that arrives too late", async () => {
      // Expand, collapse mid-flight, then refresh: the answer lands under a
      // stale sequence and is discarded, but the flag it set is its own to
      // clear. Left set, the row shows a spinner nothing will ever stop.
      let resolveSlow: (v: unknown) => void = () => {};
      mocks.listWorkdirTree.mockReturnValueOnce(
        new Promise((r) => {
          resolveSlow = r;
        }),
      );
      const inFlight = loadDirectory("docs", true);
      expect(get(loadingDirs).has("docs")).toBe(true);

      resetTree();
      resolveSlow([file("docs/a.md")]);
      await inFlight;

      expect(get(treeChildren).has("docs")).toBe(false);
      expect(get(loadingDirs).has("docs")).toBe(false);
    });

    it("abandons a refresh that was superseded before its children loaded", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([dir("src")]);
      await refreshTree(true);
      mocks.listWorkdirTree.mockResolvedValueOnce([file("src/a.ts")]);
      await toggleDirectory("src", true);

      // Refresh A's root resolves after refresh B has bumped the sequence.
      let resolveRootA: (v: unknown) => void = () => {};
      mocks.listWorkdirTree.mockReturnValueOnce(
        new Promise((r) => {
          resolveRootA = r;
        }),
      );
      const refreshA = refreshTree(false);

      mocks.listWorkdirTree.mockResolvedValue([]);
      await refreshTree(true);
      const callsAfterB = mocks.listWorkdirTree.mock.calls.length;

      resolveRootA([dir("src")]);
      await refreshA;

      // A must not go on to list `src` under B's sequence.
      expect(mocks.listWorkdirTree.mock.calls.length).toBe(callsAfterB);
    });

    it("drops a listing that lands after the tree was reset", async () => {
      // A project switch resets the tree; the listing still in flight from
      // the previous repo must not write its paths into the new one.
      let resolveSlow: (v: unknown) => void = () => {};
      mocks.listWorkdirTree.mockReturnValueOnce(
        new Promise((r) => {
          resolveSlow = r;
        }),
      );
      const inFlight = loadDirectory("old-project-dir", true);

      resetTree();
      resolveSlow([file("old-project-dir/stale.ts")]);
      await inFlight;

      expect(get(treeChildren).has("old-project-dir")).toBe(false);
    });
  });

  describe("partial tree refresh after a path mutation", () => {
    /**
     * These pin the difference between `refreshTree` and
     * `refreshTreePaths`, and they are written to fail if the CRUD wrappers
     * go back to the full refresh. That matters because the whole 1151-test
     * suite passed either way before they existed — the change is invisible
     * to every other assertion.
     *
     * What the full refresh did wrong was not the re-listing; it was
     * `treeChildren.set(new Map())` first. Emptying the map is a blank frame
     * the user sees for nothing, since `project-mutated` has usually already
     * refreshed the tree by the time the wrapper's own refresh lands.
     */
    it("re-lists only the parent, leaving sibling directories cached", async () => {
      // Two directories listed and cached.
      mocks.listWorkdirTree.mockResolvedValueOnce([dir("src"), dir("docs")]);
      await loadDirectory("", true);
      mocks.listWorkdirTree.mockResolvedValueOnce([file("src/a.ts")]);
      await loadDirectory("src", true);
      mocks.listWorkdirTree.mockResolvedValueOnce([file("docs/x.md")]);
      await loadDirectory("docs", true);
      mocks.listWorkdirTree.mockClear();

      mocks.listWorkdirTree.mockResolvedValueOnce([
        file("src/a.ts"),
        file("src/new.ts"),
      ]);
      await createPath("src/new.ts", false, true);

      // Exactly one listing, for the parent — not the root, not the sibling.
      const prefixes = mocks.listWorkdirTree.mock.calls.map((c) => c[0]);
      expect(prefixes).toEqual(["src"]);

      // And the sibling's cache is untouched, which is what a full refresh
      // would have dropped.
      expect(get(treeChildren).get("docs")).toEqual([file("docs/x.md")]);
      expect(get(treeChildren).get("src")).toEqual([
        file("src/a.ts"),
        file("src/new.ts"),
      ]);
    });

    it("treats a top-level path as living in the root", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([file("a.ts")]);
      await loadDirectory("", true);
      mocks.listWorkdirTree.mockClear();

      mocks.listWorkdirTree.mockResolvedValueOnce([file("a.ts"), file("b.ts")]);
      await createPath("b.ts", false, true);

      // `null` is how the root is addressed over IPC.
      expect(mocks.listWorkdirTree.mock.calls.map((c) => c[0])).toEqual([null]);
    });

    it("drops the cached subtree of a deleted directory", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([dir("src")]);
      await loadDirectory("", true);
      mocks.listWorkdirTree.mockResolvedValueOnce([dir("src/old")]);
      await loadDirectory("src", true);
      mocks.listWorkdirTree.mockResolvedValueOnce([file("src/old/stale.ts")]);
      await loadDirectory("src/old", true);
      await toggleDirectory("src/old", true);
      expect(get(treeChildren).has("src/old")).toBe(true);
      mocks.listWorkdirTree.mockClear();

      mocks.listWorkdirTree.mockResolvedValueOnce([]);
      await deletePath("src/old", true);

      // Without the purge, re-creating `src/old` later would show
      // `stale.ts` — the partial refresh no longer clears it wholesale.
      expect(get(treeChildren).has("src/old")).toBe(false);
      expect(get(expandedDirs).has("src/old")).toBe(false);
      // The parent survives, re-listed.
      expect(get(treeChildren).get("src")).toEqual([]);
    });

    it("re-lists both ends of a cross-directory rename", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([dir("a"), dir("b")]);
      await loadDirectory("", true);
      mocks.listWorkdirTree.mockClear();
      mocks.listWorkdirTree.mockResolvedValue([]);

      await renamePath("a/f.ts", "b/f.ts", true);

      const prefixes = mocks.listWorkdirTree.mock.calls.map((c) => c[0]).sort();
      expect(prefixes).toEqual(["a", "b"]);
    });

    it("lists a same-directory rename's parent once, not twice", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([dir("src")]);
      await loadDirectory("", true);
      mocks.listWorkdirTree.mockClear();
      mocks.listWorkdirTree.mockResolvedValue([]);

      await renamePath("src/old.ts", "src/new.ts", true);

      // Deduped: two listings for one key would race two writes for it.
      expect(mocks.listWorkdirTree.mock.calls.map((c) => c[0])).toEqual(["src"]);
    });

    it("refreshTreePaths keeps every other listing, unlike refreshTree", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([dir("keep"), dir("touch")]);
      await loadDirectory("", true);
      mocks.listWorkdirTree.mockResolvedValueOnce([file("keep/k.ts")]);
      await loadDirectory("keep", true);
      mocks.listWorkdirTree.mockClear();

      mocks.listWorkdirTree.mockResolvedValueOnce([file("touch/t.ts")]);
      await refreshTreePaths(["touch"], true);
      expect(get(treeChildren).get("keep")).toEqual([file("keep/k.ts")]);

      // For contrast: the full refresh empties the map first, which is the
      // blank frame this replaced.
      mocks.listWorkdirTree.mockResolvedValue([]);
      await refreshTree(true);
      expect(get(treeChildren).get("keep")).toBeUndefined();
    });

    it("parentDir handles nesting, top level, and trailing segments", () => {
      expect(parentDir("a/b/c.ts")).toBe("a/b");
      expect(parentDir("a.ts")).toBe("");
      expect(parentDir("a/b")).toBe("a");
    });
  });

  describe("searchTree", () => {
    it("asks the backend rather than filtering what is already loaded", async () => {
      mocks.searchWorkdirFiles.mockResolvedValueOnce([file("deep/down/needle.ts")]);

      await searchTree("needle", true);

      expect(mocks.searchWorkdirFiles).toHaveBeenCalledWith("needle", SEARCH_RESULT_CAP, true);
      expect(get(searchResults)).toHaveLength(1);
    });

    it("treats an empty query as 'not searching', without a round-trip", async () => {
      mocks.searchWorkdirFiles.mockResolvedValueOnce([file("a.ts")]);
      await searchTree("a", true);
      mocks.searchWorkdirFiles.mockClear();

      await searchTree("   ", true);

      expect(mocks.searchWorkdirFiles).not.toHaveBeenCalled();
      expect(get(searchResults)).toEqual([]);
    });

    it("flags a result set that came back at the cap", async () => {
      mocks.searchWorkdirFiles.mockResolvedValueOnce(
        Array.from({ length: SEARCH_RESULT_CAP }, (_, i) => file(`f${i}.ts`)),
      );

      await searchTree("f", true);

      expect(get(searchTruncated)).toBe(true);
    });

    it("ignores a slow answer to a query the user has moved past", async () => {
      // Typing produces overlapping searches that need not resolve in
      // order; the stale one must not win.
      let resolveSlow: (v: unknown) => void = () => {};
      mocks.searchWorkdirFiles.mockReturnValueOnce(
        new Promise((r) => {
          resolveSlow = r;
        }),
      );
      const slow = searchTree("ol", true);

      mocks.searchWorkdirFiles.mockResolvedValueOnce([file("new.ts")]);
      await searchTree("old", true);

      resolveSlow([file("stale.ts")]);
      await slow;

      expect(get(searchResults).map((e) => e.path)).toEqual(["new.ts"]);
    });
  });

  describe("localStorage round-trip", () => {
    it("persistTabsForProject + restoreTabsForProject re-creates the same paths", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "x",
        size: 1,
      });
      await openTab("p/a.ts");
      await openTab("p/b.ts");
      setActiveTab("p/a.ts");
      persistTabsForProject("/projects/foo");

      // Sanity: confirm localStorage actually carries the payload.
      const raw = localStorage.getItem("beardgit:editor-tabs:/projects/foo");
      expect(raw).not.toBeNull();

      __resetForTests();
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "x",
        size: 1,
      });
      await restoreTabsForProject("/projects/foo");
      const paths = get(tabs).map((t) => t.path);
      expect(paths).toEqual(["p/a.ts", "p/b.ts"]);
      expect(get(activeTabPath)).toBe("p/a.ts");
    });

    it("restoreTabsForProject for an unknown project clears the store", async () => {
      mocks.readWorkdirFile.mockResolvedValue({
        kind: "text",
        data: "x",
        size: 1,
      });
      await openTab("a.ts");
      await restoreTabsForProject("/projects/nope");
      expect(get(tabs)).toEqual([]);
      expect(get(activeTabPath)).toBeNull();
    });
  });
});
