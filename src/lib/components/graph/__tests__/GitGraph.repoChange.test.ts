/**
 * Regression: the graph must not jump back to the top when a background
 * mutation lands or when switching tabs. `graphRepoChangeAction` encodes
 * that decision — it keys off the repo path (not the fresh `repoInfo`
 * object identity every mutation/refresh produces).
 */

import { describe, it, expect } from "vitest";
import { graphRepoChangeAction } from "../GitGraph.helpers";

describe("graphRepoChangeAction", () => {
  it("does nothing for a mutation refresh (same path, fresh repoInfo object)", () => {
    // refreshRepoInfo() swaps repoInfo for a new object with the SAME path.
    expect(graphRepoChangeAction("/repo/a", "/repo/a", true)).toBe("none");
  });

  it("restores a revisited tab whose viewport is already hydrated", () => {
    expect(graphRepoChangeAction("/repo/a", "/repo/b", true)).toBe("restore");
  });

  it("resets to the top for a repo with no restored viewport", () => {
    expect(graphRepoChangeAction("/repo/a", "/repo/b", false)).toBe("reset");
  });

  it("loads from the top on the first-ever repo activation", () => {
    expect(graphRepoChangeAction(null, "/repo/a", false)).toBe("reset");
  });

  it("does nothing when detaching to no repo (terminal tab / all tabs closed)", () => {
    expect(graphRepoChangeAction("/repo/a", null, false)).toBe("none");
  });
});
