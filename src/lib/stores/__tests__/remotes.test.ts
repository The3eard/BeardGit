/**
 * Unit tests for the `remotes` store — verifies refresh, error
 * tolerance, and the derived `remoteNames` slice.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

vi.mock("../../api/tauri", () => ({
  getRemotes: vi.fn(),
}));

import { getRemotes } from "../../api/tauri";
import { remotes, remoteNames, refreshRemotes, __resetRemotesForTests } from "../remotes";

const mockedGetRemotes = getRemotes as ReturnType<typeof vi.fn>;

describe("remotes store", () => {
  beforeEach(() => {
    __resetRemotesForTests();
    mockedGetRemotes.mockReset();
  });

  it("starts empty", () => {
    expect(get(remotes)).toEqual([]);
    expect(get(remoteNames)).toEqual([]);
  });

  it("populates after refreshRemotes()", async () => {
    mockedGetRemotes.mockResolvedValueOnce([
      { name: "origin", url: "git@github.com:x/y.git" },
      { name: "upstream", url: "git@github.com:a/b.git" },
    ]);
    await refreshRemotes();
    expect(get(remotes)).toHaveLength(2);
    expect(get(remoteNames)).toEqual(["origin", "upstream"]);
  });

  it("swallows errors and leaves the store empty", async () => {
    mockedGetRemotes.mockRejectedValueOnce(new Error("boom"));
    await refreshRemotes();
    expect(get(remotes)).toEqual([]);
  });
});
