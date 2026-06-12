/**
 * Unit tests for the PR per-file diff store and handler.
 *
 * Covers the paths in loadPrFileDiff:
 *   1. Success — both base + head resolve to text, store populates.
 *   2. Binary — getFileAtCommit returns `{ kind: "binary" }`, store gets
 *      a placeholder RawDiffContent with binary sentinels.
 *   3. Error — ensureCommitLocal rejects, error store is set.
 *   4. Preflight — BOTH base and head commits are ensured local (a
 *      missing base used to be silently swallowed and rendered the
 *      whole file as added).
 *   5. Dedupe — repeated file clicks never re-spawn the ensure (and
 *      thus the `git fetch` task) for an already-ensured sha.
 *   6. Fork fallback — an unfetchable head_repo_url falls back to
 *      origin before surfacing an error.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  prFileDiff,
  loadingPrFileDiff,
  prFileDiffError,
  loadPrFileDiff,
  clearMrPrState,
} from "$lib/stores/mr-pr";
import { invokeMock, mockInvokeResponse } from "../../../test/setup";
import type { MrPrDetail } from "$lib/types";

function makeDetail(overrides: Partial<MrPrDetail["summary"]> = {}): MrPrDetail {
  return {
    summary: {
      number: 1, title: "x", state: "open", author: "a",
      source_branch: "s", target_branch: "t", url: "u", draft: false,
      labels: [], reviewers: [], created_at: "", updated_at: "",
      additions: null, deletions: null, changed_files: null,
      base_sha: "bbbb", head_sha: "aaaa", head_repo_url: null,
      ...overrides,
    },
    body: "", comments: [], review_status: "pending", mergeable: null,
  };
}

const SAME_REPO_DETAIL = makeDetail();

/** Calls to `ensure_commit_local`, as `[sha, remoteUrl]` tuples. */
function ensureCalls(): Array<[unknown, unknown]> {
  return invokeMock.mock.calls
    .filter(([cmd]) => cmd === "ensure_commit_local")
    .map(([, args]) => {
      const a = args as { sha: string; remoteUrl: string | null };
      return [a.sha, a.remoteUrl];
    });
}

describe("loadPrFileDiff", () => {
  beforeEach(() => {
    prFileDiff.set(null);
    prFileDiffError.set(null);
    loadingPrFileDiff.set(false);
    // Also clears the module-level ensured-sha cache between cases.
    clearMrPrState();
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

  it("ensures BOTH head and base commits are local before reading blobs", async () => {
    mockInvokeResponse("ensure_commit_local", null);
    mockInvokeResponse("get_file_at_commit", () => ({ kind: "text", data: "" }));
    await loadPrFileDiff(SAME_REPO_DETAIL, "src/foo.ts");
    expect(ensureCalls()).toEqual([
      ["aaaa", null],
      ["bbbb", null],
    ]);
  });

  it("does not re-run the ensure preflight for already-ensured shas", async () => {
    mockInvokeResponse("ensure_commit_local", null);
    mockInvokeResponse("get_file_at_commit", () => ({ kind: "text", data: "" }));
    await loadPrFileDiff(SAME_REPO_DETAIL, "src/foo.ts");
    await loadPrFileDiff(SAME_REPO_DETAIL, "src/bar.ts");
    await loadPrFileDiff(SAME_REPO_DETAIL, "src/baz.ts");
    // One ensure per sha across all three file clicks — repeated clicks
    // must never re-spawn `git fetch` tasks for the same commit.
    expect(ensureCalls()).toHaveLength(2);
  });

  it("falls back to origin when the fork head_repo_url is unfetchable", async () => {
    const detail = makeDetail({ head_repo_url: "https://github.com/alice/fork.git" });
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "ensure_commit_local" && args?.remoteUrl) {
        throw new Error("could not read from remote");
      }
      if (cmd === "get_file_at_commit") return { kind: "text", data: "x" };
      return null;
    });
    await loadPrFileDiff(detail, "src/foo.ts");
    expect(get(prFileDiffError)).toBeNull();
    expect(ensureCalls()).toEqual([
      ["aaaa", "https://github.com/alice/fork.git"],
      ["aaaa", null],
      ["bbbb", null],
    ]);
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

  it("a failed ensure is retried on the next explicit attempt", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "ensure_commit_local") return Promise.reject(new Error("offline"));
      return Promise.resolve(null);
    });
    await loadPrFileDiff(SAME_REPO_DETAIL, "src/foo.ts");
    expect(get(prFileDiffError)).toContain("offline");

    // Network is back: the failure must not be cached forever.
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_file_at_commit") return { kind: "text", data: "x" };
      return null;
    });
    await loadPrFileDiff(SAME_REPO_DETAIL, "src/foo.ts");
    expect(get(prFileDiffError)).toBeNull();
    expect(get(prFileDiff)?.filename).toBe("src/foo.ts");
  });

  it("errors clearly when the PR summary is missing its shas", async () => {
    const detail = makeDetail({ base_sha: "", head_sha: "" });
    await loadPrFileDiff(detail, "src/foo.ts");
    expect(get(prFileDiff)).toBeNull();
    expect(get(prFileDiffError)).not.toBeNull();
    expect(ensureCalls()).toHaveLength(0);
  });
});
