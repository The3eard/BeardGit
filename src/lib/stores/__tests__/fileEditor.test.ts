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
  reloadActive,
  renameOpenTab,
  restoreTabsForProject,
  saveActive,
  setActiveTab,
  tabs,
  treeEntries,
  treeTruncated,
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

  describe("refreshTree", () => {
    it("populates treeEntries and treeTruncated", async () => {
      mocks.listWorkdirTree.mockResolvedValueOnce([
        { path: "a.ts", name: "a.ts", is_directory: false, size: 0 },
      ]);
      await refreshTree(true);
      expect(get(treeEntries)).toHaveLength(1);
      expect(get(treeTruncated)).toBe(false);
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
