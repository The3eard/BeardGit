import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { postReviewComment, mrPrDetail } from "$lib/stores/mr-pr";
import { invokeMock, mockInvokeResponse } from "../../../test/setup";

describe("postReviewComment", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    mrPrDetail.set({
      summary: {
        number: 1, title: "x", state: "open", author: "a",
        source_branch: "s", target_branch: "t", url: "u", draft: false,
        labels: [], reviewers: [], created_at: "", updated_at: "",
        additions: null, deletions: null, changed_files: null,
        base_sha: "bbbb", head_sha: "aaaa", head_repo_url: null,
      },
      body: "", comments: [], review_status: "pending", mergeable: null,
    });
  });

  it("forwards PR sha + refreshes detail on success", async () => {
    mockInvokeResponse("add_mr_pr_inline_comment", null);
    mockInvokeResponse("get_mr_pr_detail", (_args: unknown) => ({
      summary: {
        number: 1, title: "x", state: "open", author: "a",
        source_branch: "s", target_branch: "t", url: "u", draft: false,
        labels: [], reviewers: [], created_at: "", updated_at: "",
        additions: null, deletions: null, changed_files: null,
        base_sha: "bbbb", head_sha: "aaaa", head_repo_url: null,
      },
      body: "", comments: [{
        id: 7, author: "a", body: "LGTM", created_at: "", path: "a.ts",
        line: 3, is_review: true, resolvable: null, resolved: null, discussion_id: null,
      }],
      review_status: "pending", mergeable: null,
    }));
    mockInvokeResponse("get_mr_pr_diff", []);
    await postReviewComment(1, "a.ts", 3, "LGTM");
    const call = invokeMock.mock.calls.find((c) => c[0] === "add_mr_pr_inline_comment")!;
    expect((call[1] as { baseSha: string }).baseSha).toBe("bbbb");
    expect((call[1] as { headSha: string }).headSha).toBe("aaaa");
    expect(get(mrPrDetail)?.comments[0]?.body).toBe("LGTM");
  });
});
