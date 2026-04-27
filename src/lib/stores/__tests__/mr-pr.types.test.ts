/**
 * Type-only assertions that MrPr carries the PR-head SHA fields the diff
 * view relies on. No runtime behaviour.
 */
import { describe, it, expect } from "vitest";
import type { MrPr } from "$lib/types";

describe("MrPr type surface", () => {
  it("exposes base_sha, head_sha, head_repo_url", () => {
    const m: MrPr = {
      number: 1,
      title: "x",
      state: "open",
      author: "a",
      source_branch: "s",
      target_branch: "t",
      url: "u",
      draft: false,
      labels: [],
      reviewers: [],
      created_at: "",
      updated_at: "",
      additions: null,
      deletions: null,
      changed_files: null,
      base_sha: "bbbb",
      head_sha: "aaaa",
      head_repo_url: null,
    };
    expect(m.base_sha).toBe("bbbb");
    expect(m.head_sha).toBe("aaaa");
    expect(m.head_repo_url).toBeNull();
  });
});
