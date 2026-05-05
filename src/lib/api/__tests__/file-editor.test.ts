/**
 * File-editor wrappers — verify each TS function maps to the expected
 * Tauri command name and payload shape.
 *
 * The wrappers themselves are thin (`invoke(name, args)`), so the only
 * things worth asserting are:
 *  - the snake_case command name on the wire,
 *  - the camelCase argument keys (Tauri auto-converts to snake_case),
 *  - that the wrapper preserves the invoke return value untouched.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import {
  readWorkdirFile,
  writeWorkdirFile,
  listWorkdirTree,
  createWorkdirPath,
  renameWorkdirPath,
  deleteWorkdirPath,
} from "../tauri";

beforeEach(() => {
  mocks.invoke.mockReset();
});

describe("file-editor wrappers", () => {
  it("readWorkdirFile invokes 'read_workdir_file' with { path }", async () => {
    mocks.invoke.mockResolvedValue({ kind: "text", data: "hi", size: 2 });
    const out = await readWorkdirFile("notes.md");
    expect(mocks.invoke).toHaveBeenCalledWith("read_workdir_file", {
      path: "notes.md",
    });
    expect(out).toEqual({ kind: "text", data: "hi", size: 2 });
  });

  it("writeWorkdirFile invokes 'write_workdir_file' with { path, content }", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    await writeWorkdirFile("a/b.txt", "body");
    expect(mocks.invoke).toHaveBeenCalledWith("write_workdir_file", {
      path: "a/b.txt",
      content: "body",
    });
  });

  it("listWorkdirTree invokes with camelCase keys", async () => {
    mocks.invoke.mockResolvedValue([]);
    await listWorkdirTree("src", 100, true);
    expect(mocks.invoke).toHaveBeenCalledWith("list_workdir_tree", {
      prefix: "src",
      maxEntries: 100,
      respectGitignore: true,
    });
  });

  it("listWorkdirTree forwards null prefix for full walk", async () => {
    mocks.invoke.mockResolvedValue([]);
    await listWorkdirTree(null, 50, false);
    expect(mocks.invoke).toHaveBeenCalledWith("list_workdir_tree", {
      prefix: null,
      maxEntries: 50,
      respectGitignore: false,
    });
  });

  it("createWorkdirPath invokes with isDirectory flag", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    await createWorkdirPath("new-dir", true);
    expect(mocks.invoke).toHaveBeenCalledWith("create_workdir_path", {
      path: "new-dir",
      isDirectory: true,
    });
  });

  it("renameWorkdirPath invokes with fromPath/toPath", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    await renameWorkdirPath("a.txt", "b.txt");
    expect(mocks.invoke).toHaveBeenCalledWith("rename_workdir_path", {
      fromPath: "a.txt",
      toPath: "b.txt",
    });
  });

  it("deleteWorkdirPath invokes 'delete_workdir_path' with { path }", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    await deleteWorkdirPath("doomed.txt");
    expect(mocks.invoke).toHaveBeenCalledWith("delete_workdir_path", {
      path: "doomed.txt",
    });
  });
});
