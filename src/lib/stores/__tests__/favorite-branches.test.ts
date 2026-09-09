/**
 * Favorite-branch store: load, toggle, persistence payload, rollback.
 *
 * The star is written back as the whole list, and the store is updated before
 * the IPC round-trip so the row reorders on the click. Both halves matter: a
 * wrong payload silently drops other stars, and a failed write that isn't
 * rolled back leaves the panel showing a star that isn't on disk.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import { get } from "svelte/store";
import {
  favoriteBranches,
  loadFavoriteBranches,
  toggleFavoriteBranch,
} from "../branches";
import { __resetRepoStateForTests } from "../repo-state";

beforeEach(() => {
  mocks.invoke.mockReset();
  __resetRepoStateForTests();
});

/** Seed the store the way a panel mount does. */
async function seed(stars: string[]) {
  mocks.invoke.mockResolvedValueOnce(stars);
  await loadFavoriteBranches();
  mocks.invoke.mockReset();
}

describe("loadFavoriteBranches", () => {
  it("reads the active repo's stars", async () => {
    mocks.invoke.mockResolvedValueOnce(["beta", "origin/main"]);

    await loadFavoriteBranches();

    expect(mocks.invoke).toHaveBeenCalledWith("get_favorite_branches");
    expect([...get(favoriteBranches)]).toEqual(["beta", "origin/main"]);
  });

  it("leaves the current set alone when the file cannot be read", async () => {
    await seed(["beta"]);
    mocks.invoke.mockRejectedValueOnce({ code: "io_error", message: "unparseable" });

    await loadFavoriteBranches();

    expect([...get(favoriteBranches)]).toEqual(["beta"]);
  });
});

describe("toggleFavoriteBranch", () => {
  it("stars a branch and persists the full list", async () => {
    await seed(["beta"]);
    mocks.invoke.mockResolvedValueOnce(undefined);

    await toggleFavoriteBranch("origin/main");

    expect(get(favoriteBranches).has("origin/main")).toBe(true);
    expect(mocks.invoke).toHaveBeenCalledWith("set_favorite_branches", {
      branches: ["beta", "origin/main"],
    });
  });

  it("unstars a branch that was starred", async () => {
    await seed(["beta", "main"]);
    mocks.invoke.mockResolvedValueOnce(undefined);

    await toggleFavoriteBranch("beta");

    expect(get(favoriteBranches).has("beta")).toBe(false);
    expect(mocks.invoke).toHaveBeenCalledWith("set_favorite_branches", {
      branches: ["main"],
    });
  });

  it("rolls the star back when the write fails", async () => {
    await seed(["beta"]);
    mocks.invoke.mockRejectedValueOnce({ code: "io_error", message: "read-only fs" });

    await toggleFavoriteBranch("feat/thing");

    expect([...get(favoriteBranches)]).toEqual(["beta"]);
  });

  it("restores a removed star when the write fails", async () => {
    await seed(["beta"]);
    mocks.invoke.mockRejectedValueOnce({ code: "io_error", message: "read-only fs" });

    await toggleFavoriteBranch("beta");

    expect([...get(favoriteBranches)]).toEqual(["beta"]);
  });
});
