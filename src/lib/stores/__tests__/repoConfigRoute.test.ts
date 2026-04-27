import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  repoConfigRoute,
  setSection,
  pendingRepoConfigSection,
  seedFromLocation,
  DEFAULT_SECTION,
} from "../repoConfigRoute";

function resetHash() {
  window.location.hash = "";
}

beforeEach(() => {
  resetHash();
  repoConfigRoute.set({ section: DEFAULT_SECTION });
  pendingRepoConfigSection.set(null);
});

describe("repoConfigRoute", () => {
  it("defaults to the general section", () => {
    expect(get(repoConfigRoute).section).toBe("general");
  });

  it("setSection updates the store and the hash", () => {
    setSection("protection");
    expect(get(repoConfigRoute).section).toBe("protection");
    expect(window.location.hash).toBe("#repo-config/protection");
  });

  it("unknown section ids fall back to the default", () => {
    setSection("does-not-exist");
    expect(get(repoConfigRoute).section).toBe("general");
  });

  it("seedFromLocation parses #repo-config/<id>", () => {
    window.location.hash = "#repo-config/features";
    seedFromLocation();
    expect(get(repoConfigRoute).section).toBe("features");
  });

  it("seedFromLocation ignores unrelated hashes", () => {
    window.location.hash = "#ai";
    seedFromLocation();
    // unchanged from before seeding
    expect(get(repoConfigRoute).section).toBe("general");
  });

  it("pendingRepoConfigSection is just a writable bridge", () => {
    pendingRepoConfigSection.set("labels");
    expect(get(pendingRepoConfigSection)).toBe("labels");
    pendingRepoConfigSection.set(null);
    expect(get(pendingRepoConfigSection)).toBeNull();
  });
});
