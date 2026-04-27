import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("$lib/api/tauri", async () => {
  return {
    loadRemoteRepoConfig: vi.fn(),
  };
});

import * as tauri from "$lib/api/tauri";
import { loadConfig, invalidate, __resetForTests } from "../loader";
import type { RemoteRepoConfig } from "$lib/types/repoConfig";

const mockConfig = (desc = "x"): RemoteRepoConfig => ({
  description: desc,
  homepage: null,
  topics: [],
  visibility: "public",
  default_branch: "main",
  issues_enabled: true,
  wiki_enabled: false,
  archived: false,
  branch_protection: null,
  labels: [],
});

beforeEach(() => {
  __resetForTests();
  (tauri.loadRemoteRepoConfig as unknown as ReturnType<typeof vi.fn>).mockReset();
});

describe("loader", () => {
  it("dedupes concurrent in-flight calls for the same repo", async () => {
    const fn = tauri.loadRemoteRepoConfig as unknown as ReturnType<typeof vi.fn>;
    fn.mockResolvedValue(mockConfig("a"));

    const [r1, r2] = await Promise.all([loadConfig("/repo"), loadConfig("/repo")]);
    expect(r1).toEqual(r2);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("reuses a fresh cached value without hitting the api", async () => {
    const fn = tauri.loadRemoteRepoConfig as unknown as ReturnType<typeof vi.fn>;
    fn.mockResolvedValueOnce(mockConfig("first"));
    const first = await loadConfig("/repo");
    const second = await loadConfig("/repo");
    expect(first).toEqual(second);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("force: true skips the cache", async () => {
    const fn = tauri.loadRemoteRepoConfig as unknown as ReturnType<typeof vi.fn>;
    fn.mockResolvedValueOnce(mockConfig("a"));
    fn.mockResolvedValueOnce(mockConfig("b"));
    await loadConfig("/repo");
    const forced = await loadConfig("/repo", { force: true });
    expect(forced.description).toBe("b");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("invalidate() drops the cache", async () => {
    const fn = tauri.loadRemoteRepoConfig as unknown as ReturnType<typeof vi.fn>;
    fn.mockResolvedValueOnce(mockConfig("a"));
    fn.mockResolvedValueOnce(mockConfig("b"));
    await loadConfig("/repo");
    invalidate("/repo");
    const second = await loadConfig("/repo");
    expect(second.description).toBe("b");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("propagates errors and does not cache a rejection", async () => {
    const fn = tauri.loadRemoteRepoConfig as unknown as ReturnType<typeof vi.fn>;
    fn.mockRejectedValueOnce(new Error("nope"));
    await expect(loadConfig("/repo")).rejects.toThrow("nope");

    fn.mockResolvedValueOnce(mockConfig("ok"));
    const ok = await loadConfig("/repo");
    expect(ok.description).toBe("ok");
    expect(fn).toHaveBeenCalledTimes(2);
  });
});
