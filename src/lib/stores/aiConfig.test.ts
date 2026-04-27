import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import {
  configFiles,
  activeFilePath,
  activeFileDirty,
  configFileChangedOnDisk,
  loadConfigFiles,
  openFile,
  saveFile,
  clearConfigState,
  dismissDiskChange,
} from "./aiConfig";

import type { AiConfigFile } from "$lib/types";

describe("aiConfig store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearConfigState();
  });

  it("loadConfigFiles populates the store", async () => {
    const mockFiles: AiConfigFile[] = [
      { path: "/repo/CLAUDE.md", kind: "instructions", scope: "project" },
      { path: "/repo/.claude/settings.json", kind: "settings", scope: "project" },
    ];
    mockInvoke.mockResolvedValueOnce(mockFiles);
    await loadConfigFiles();
    expect(get(configFiles)).toEqual(mockFiles);
  });

  it("openFile loads content and sets active path", async () => {
    mockInvoke.mockResolvedValueOnce("# Hello");
    await openFile("/repo/CLAUDE.md");
    expect(get(activeFilePath)).toBe("/repo/CLAUDE.md");
    expect(get(activeFileDirty)).toBe(false);
  });

  it("saveFile writes content and clears dirty flag", async () => {
    activeFilePath.set("/repo/CLAUDE.md");
    activeFileDirty.set(true);
    mockInvoke.mockResolvedValueOnce(undefined);
    await saveFile("# Updated");
    expect(get(activeFileDirty)).toBe(false);
    expect(mockInvoke).toHaveBeenCalledWith("ai_write_config_file", {
      path: "/repo/CLAUDE.md",
      content: "# Updated",
    });
  });

  it("configFileChangedOnDisk starts false", () => {
    expect(get(configFileChangedOnDisk)).toBe(false);
  });

  it("dismissDiskChange clears the flag", () => {
    configFileChangedOnDisk.set(true);
    dismissDiskChange();
    expect(get(configFileChangedOnDisk)).toBe(false);
  });

  it("clearConfigState resets configFileChangedOnDisk", () => {
    configFileChangedOnDisk.set(true);
    clearConfigState();
    expect(get(configFileChangedOnDisk)).toBe(false);
  });
});
