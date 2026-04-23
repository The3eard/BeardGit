/**
 * Unit tests for the PR per-file diff store and handler.
 *
 * Covers the three paths in loadPrFileDiff:
 *   1. Success — both base + head resolve to text, store populates.
 *   2. Binary — getFileAtCommit returns `{ kind: "binary" }`, store gets
 *      a placeholder RawDiffContent with binary sentinels.
 *   3. Error — ensureCommitLocal rejects, error store is set.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  prFileDiff,
  loadingPrFileDiff,
  prFileDiffError,
  loadPrFileDiff,
} from "$lib/stores/mr-pr";
import { invokeMock, mockInvokeResponse } from "../../../test/setup";
import type { MrPrDetail } from "$lib/types";

const SAME_REPO_DETAIL: MrPrDetail = {
  summary: {
    number: 1, title: "x", state: "open", author: "a",
    source_branch: "s", target_branch: "t", url: "u", draft: false,
    labels: [], reviewers: [], created_at: "", updated_at: "",
    additions: null, deletions: null, changed_files: null,
    base_sha: "bbbb", head_sha: "aaaa", head_repo_url: null,
  },
  body: "", comments: [], review_status: "pending", mergeable: null,
};

describe("loadPrFileDiff", () => {
  beforeEach(() => {
    prFileDiff.set(null);
    prFileDiffError.set(null);
    loadingPrFileDiff.set(false);
    invokeMock.mockReset();
  });

  it("populates prFileDiff on success", async () => {
    mockInvokeResponse("ensure_commit_local", null);
    mockInvokeResponse("get_file_at_commit", (args: unknown) => {
      if ((args as { oid: string }).oid === "bbbb") return { kind: "text", data: "old" };
      return { kind: "text", data: "new" };
    });
    await loadPrFileDiff(SAME_REPO_DETAIL, "src/foo.ts");
    expect(get(prFileDiff)).toEqual({ oldContent: "old", newContent: "new", filename: "src/foo.ts", binary: false });
    expect(get(prFileDiffError)).toBeNull();
  });

  it("emits a binary placeholder when either side is binary", async () => {
    mockInvokeResponse("ensure_commit_local", null);
    mockInvokeResponse("get_file_at_commit", () => ({ kind: "binary" }));
    await loadPrFileDiff(SAME_REPO_DETAIL, "img.png");
    const d = get(prFileDiff);
    expect(d?.filename).toBe("img.png");
    expect(d?.oldContent).toBe("");
    expect(d?.newContent).toBe("");
    // Sentinel flag consumed by DiffEditor placeholder branch.
    expect((d as { binary?: boolean } | null)?.binary).toBe(true);
  });

  it("sets prFileDiffError when ensureCommitLocal rejects", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "ensure_commit_local") return Promise.reject(new Error("fetch failed"));
      return Promise.resolve(null);
    });
    await loadPrFileDiff(SAME_REPO_DETAIL, "src/foo.ts");
    expect(get(prFileDiff)).toBeNull();
    expect(get(prFileDiffError)).toContain("fetch failed");
  });
});
