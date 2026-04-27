/**
 * Unit tests for `repoConfig.ts` store.
 *
 * Covers:
 *   - Diff-driven patch building (`buildPatch`) — every single-field
 *     change + no-op + tri-state homepage transitions.
 *   - `isPatchEmpty` predicate.
 *   - Store lifecycle: `setLoading` → `setLoadedConfig` → `updateCurrent`
 *     → `commitSavedConfig` → `resetRepoConfigStore`.
 *   - Derived `repoConfigDirty` flips with `current` mutations.
 */

import { describe, expect, it, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  buildPatch,
  isPatchEmpty,
  setLoadedConfig,
  setLoading,
  setLoadError,
  updateCurrent,
  commitSavedConfig,
  resetRepoConfigStore,
  repoConfigStore,
  repoConfigDirty,
  repoConfigPatch,
} from "../repoConfig";
import type { RemoteRepoConfig } from "../../types/repoConfig";

function baseConfig(): RemoteRepoConfig {
  return {
    description: "A repo",
    homepage: "https://example.com",
    topics: ["a", "b"],
    visibility: "public",
    default_branch: "main",
    issues_enabled: true,
    wiki_enabled: false,
    archived: false,
    branch_protection: null,
    labels: [],
  };
}

beforeEach(() => {
  resetRepoConfigStore();
});

describe("buildPatch", () => {
  it("returns an all-empty patch when snapshots match", () => {
    const c = baseConfig();
    const patch = buildPatch(c, c);
    expect(isPatchEmpty(patch)).toBe(true);
    expect(patch.homepage).toEqual({ kind: "unchanged" });
    expect(patch.topics_added).toEqual([]);
    expect(patch.topics_removed).toEqual([]);
  });

  it("captures a description change", () => {
    const before = baseConfig();
    const after = { ...before, description: "New description" };
    const patch = buildPatch(before, after);
    expect(patch.description).toBe("New description");
    expect(isPatchEmpty(patch)).toBe(false);
  });

  it("emits PatchValue.Clear when homepage goes from set to empty", () => {
    const before = baseConfig();
    const after = { ...before, homepage: null };
    const patch = buildPatch(before, after);
    expect(patch.homepage).toEqual({ kind: "clear" });
  });

  it("emits PatchValue.Set when homepage changes to a new URL", () => {
    const before = baseConfig();
    const after = { ...before, homepage: "https://new.example.com" };
    const patch = buildPatch(before, after);
    expect(patch.homepage).toEqual({
      kind: "set",
      value: "https://new.example.com",
    });
  });

  it("treats empty string as Clear (user clicked clear then saved blank)", () => {
    const before = baseConfig();
    const after = { ...before, homepage: "" };
    const patch = buildPatch(before, after);
    expect(patch.homepage).toEqual({ kind: "clear" });
  });

  it("computes sorted topic add/remove deltas", () => {
    const before = { ...baseConfig(), topics: ["a", "b", "c"] };
    const after = { ...baseConfig(), topics: ["z", "a", "d"] };
    const patch = buildPatch(before, after);
    expect(patch.topics_added).toEqual(["d", "z"]);
    expect(patch.topics_removed).toEqual(["b", "c"]);
  });

  it("captures visibility transitions", () => {
    const before = baseConfig();
    const after = { ...before, visibility: "private" as const };
    const patch = buildPatch(before, after);
    expect(patch.visibility).toBe("private");
  });

  it("captures default-branch rename", () => {
    const before = baseConfig();
    const after = { ...before, default_branch: "develop" };
    const patch = buildPatch(before, after);
    expect(patch.default_branch).toBe("develop");
  });

  it("captures issues and wiki toggles", () => {
    const before = baseConfig();
    const after = { ...before, issues_enabled: false, wiki_enabled: true };
    const patch = buildPatch(before, after);
    expect(patch.issues_enabled).toBe(false);
    expect(patch.wiki_enabled).toBe(true);
  });

  it("captures archive transitions as archive: true/false", () => {
    const before = baseConfig();
    const after = { ...before, archived: true };
    const patch = buildPatch(before, after);
    expect(patch.archive).toBe(true);

    const back = buildPatch(after, before);
    expect(back.archive).toBe(false);
  });

  it("handles hostile topic names as literal strings", () => {
    // Shell-injection regression: the topic arrives as a single
    // argv entry; the store never sees command syntax.
    const before = { ...baseConfig(), topics: [] };
    const after = {
      ...baseConfig(),
      topics: ["x; echo INJECTED", "clean"],
    };
    const patch = buildPatch(before, after);
    expect(patch.topics_added).toContain("x; echo INJECTED");
  });
});

describe("isPatchEmpty", () => {
  it("returns true for the default/empty patch", () => {
    const before = baseConfig();
    expect(isPatchEmpty(buildPatch(before, before))).toBe(true);
  });

  it("returns false whenever homepage moves off 'unchanged'", () => {
    expect(
      isPatchEmpty(
        buildPatch(baseConfig(), { ...baseConfig(), homepage: null }),
      ),
    ).toBe(false);
  });
});

describe("store lifecycle", () => {
  it("setLoading flips loading=true and clears before/current", () => {
    setLoading("/tmp/repo");
    const s = get(repoConfigStore);
    expect(s.loading).toBe(true);
    expect(s.repoPath).toBe("/tmp/repo");
    expect(s.before).toBeNull();
    expect(s.current).toBeNull();
  });

  it("setLoadedConfig populates before + current with a deep copy", () => {
    const cfg = baseConfig();
    setLoadedConfig("/tmp/repo", cfg);
    const s = get(repoConfigStore);
    expect(s.before).toEqual(cfg);
    expect(s.current).toEqual(cfg);
    // Mutating the source must not leak into the store.
    cfg.topics.push("new");
    expect(get(repoConfigStore).current!.topics).toEqual(["a", "b"]);
  });

  it("updateCurrent mutates only `current`, not `before`", () => {
    setLoadedConfig("/tmp/repo", baseConfig());
    updateCurrent((draft) => {
      draft.description = "Edited";
    });
    const s = get(repoConfigStore);
    expect(s.current!.description).toBe("Edited");
    expect(s.before!.description).toBe("A repo");
  });

  it("repoConfigDirty reflects mutations to current", () => {
    setLoadedConfig("/tmp/repo", baseConfig());
    expect(get(repoConfigDirty)).toBe(false);
    updateCurrent((draft) => {
      draft.description = "Edited";
    });
    expect(get(repoConfigDirty)).toBe(true);
  });

  it("repoConfigPatch is null while no config is loaded", () => {
    expect(get(repoConfigPatch)).toBeNull();
  });

  it("commitSavedConfig resets before/current to the fresh snapshot", () => {
    setLoadedConfig("/tmp/repo", baseConfig());
    updateCurrent((draft) => {
      draft.description = "Edited";
    });
    const fresh: RemoteRepoConfig = { ...baseConfig(), description: "Edited" };
    commitSavedConfig(fresh);
    expect(get(repoConfigDirty)).toBe(false);
    expect(get(repoConfigStore).before).toEqual(fresh);
  });

  it("setLoadError clears loading and records the message", () => {
    setLoading("/tmp/repo");
    setLoadError("boom");
    const s = get(repoConfigStore);
    expect(s.loading).toBe(false);
    expect(s.error).toBe("boom");
  });

  it("resetRepoConfigStore restores the initial (empty) state", () => {
    setLoadedConfig("/tmp/repo", baseConfig());
    resetRepoConfigStore();
    const s = get(repoConfigStore);
    expect(s.repoPath).toBeNull();
    expect(s.before).toBeNull();
    expect(s.current).toBeNull();
    expect(s.loading).toBe(false);
    expect(s.error).toBeNull();
  });
});
