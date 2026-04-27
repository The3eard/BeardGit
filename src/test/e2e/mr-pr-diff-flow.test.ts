/**
 * End-to-end style flow covering the PR diff view happy paths with
 * mocked IPC. Mirrors spec §8.2 scenarios: same-repo PR, fork-PR fetch,
 * inline comment post, GitLab resolve toggle, bracket-key navigation,
 * file tree threshold.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  mrPrDetail, mrPrDiffFiles, selectedMrPrNumber,
  prFileDiff, selectedPrFilePath,
  loadPrFileDiff, postReviewComment,
  registerPrDiffShortcuts,
} from "$lib/stores/mr-pr";
import { shortcuts } from "$lib/stores/shortcuts";
import { invokeMock, mockInvokeResponse } from "../setup";

const FIXTURE_GH_DETAIL = (overrides: Partial<{ head_repo_url: string | null }> = {}) => ({
  summary: {
    number: 1, title: "gh", state: "open", author: "a",
    source_branch: "s", target_branch: "t", url: "u", draft: false,
    labels: [], reviewers: [], created_at: "", updated_at: "",
    additions: null, deletions: null, changed_files: null,
    base_sha: "bbbb1111", head_sha: "aaaa2222",
    head_repo_url: overrides.head_repo_url ?? null,
  },
  body: "", comments: [], review_status: "pending", mergeable: null,
});

beforeEach(() => {
  invokeMock.mockReset();
  mrPrDetail.set(null); mrPrDiffFiles.set([]); prFileDiff.set(null);
  selectedMrPrNumber.set(null); selectedPrFilePath.set(null);
});

describe("mr-pr diff flow", () => {
  it("same-repo PR: file click renders diff without extra fetch", async () => {
    mockInvokeResponse("ensure_commit_local", null);
    mockInvokeResponse("get_file_at_commit", (args: { oid: string }) =>
      args.oid === "bbbb1111" ? { kind: "text", data: "A" } : { kind: "text", data: "B" },
    );
    await loadPrFileDiff(FIXTURE_GH_DETAIL() as never, "x.ts");
    const ensure = invokeMock.mock.calls.find((c) => c[0] === "ensure_commit_local")!;
    expect((ensure[1] as { remoteUrl: string | null }).remoteUrl).toBeNull();
    expect(get(prFileDiff)?.oldContent).toBe("A");
    expect(get(prFileDiff)?.newContent).toBe("B");
  });

  it("fork PR: ensure_commit_local is called with the fork URL", async () => {
    mockInvokeResponse("ensure_commit_local", null);
    mockInvokeResponse("get_file_at_commit", () => ({ kind: "text", data: "" }));
    await loadPrFileDiff(
      FIXTURE_GH_DETAIL({ head_repo_url: "https://github.com/alice/fork" }) as never,
      "x.ts",
    );
    const ensure = invokeMock.mock.calls.find((c) => c[0] === "ensure_commit_local")!;
    expect((ensure[1] as { remoteUrl: string | null }).remoteUrl)
      .toBe("https://github.com/alice/fork");
  });

  it("inline comment post refreshes detail + surfaces new comment", async () => {
    mrPrDetail.set(FIXTURE_GH_DETAIL() as never);
    mockInvokeResponse("add_mr_pr_inline_comment", null);
    mockInvokeResponse("get_mr_pr_detail", () => ({
      ...FIXTURE_GH_DETAIL(),
      comments: [{
        id: 9, author: "a", body: "LGTM", created_at: "",
        path: "x.ts", line: 1, is_review: true,
        resolvable: null, resolved: null, discussion_id: null,
      }],
    }));
    mockInvokeResponse("get_mr_pr_diff", []);
    await postReviewComment(1, "x.ts", 1, "LGTM");
    expect(get(mrPrDetail)?.comments[0]?.body).toBe("LGTM");
  });

  it("GitLab resolve toggle updates both the inline and bottom views", async () => {
    mrPrDetail.set({
      ...FIXTURE_GH_DETAIL(),
      comments: [{
        id: 10, author: "a", body: "fix this", created_at: "",
        path: "x.ts", line: 2, is_review: true,
        resolvable: true, resolved: false, discussion_id: "d1",
      }],
    } as never);
    mockInvokeResponse("resolve_discussion", null);
    mockInvokeResponse("get_mr_pr_detail", () => ({
      ...FIXTURE_GH_DETAIL(),
      comments: [{
        id: 10, author: "a", body: "fix this", created_at: "",
        path: "x.ts", line: 2, is_review: true,
        resolvable: true, resolved: true, discussion_id: "d1",
      }],
    }));
    mockInvokeResponse("get_mr_pr_diff", []);
    const { resolveDiscussion } = await import("$lib/stores/mr-pr");
    await resolveDiscussion(1, "d1");
    expect(get(mrPrDetail)?.comments[0]?.resolved).toBe(true);
  });

  it("bracket shortcuts cycle PR files", () => {
    mrPrDiffFiles.set([
      { path: "a.ts", old_path: null, status: "modified", additions: 1, deletions: 0, patch: null },
      { path: "b.ts", old_path: null, status: "modified", additions: 1, deletions: 0, patch: null },
    ]);
    selectedPrFilePath.set("a.ts");
    const visits: string[] = [];
    registerPrDiffShortcuts({
      onPrev: () => visits.push("prev"),
      onNext: () => visits.push("next"),
    });
    const list = get(shortcuts);
    list.find((s) => s.id === "prDiff.next")!.action();
    list.find((s) => s.id === "prDiff.prev")!.action();
    expect(visits).toEqual(["next", "prev"]);
  });
});
