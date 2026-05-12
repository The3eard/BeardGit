/**
 * E2E: clicking a PR file row opens the bottom diff panel and renders
 * the DiffEditor with the loaded content.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { render, fireEvent, waitFor } from "@testing-library/svelte";
import { get } from "svelte/store";
import Page from "../../routes/+page.svelte";
import {
  mrPrDetail, mrPrDiffFiles, selectedMrPrNumber,
  prFileDiff, loadingPrFileDiff,
} from "$lib/stores/mr-pr";
import { providerStatus } from "$lib/stores/provider";
import { invokeMock, mockInvokeResponse } from "../setup";

beforeEach(() => {
  invokeMock.mockReset();
  mockInvokeResponse("ensure_commit_local", null);
  mockInvokeResponse("get_file_at_commit", (args: { oid: string }) =>
    args.oid === "bbbb" ? { kind: "text", data: "OLD" } : { kind: "text", data: "NEW" },
  );
  // Page.onMount also hydrates sidebar layout + collapsed state + project
  // state; silence those paths so unhandled rejections don't pollute the
  // test output.
  mockInvokeResponse("get_sidebar_nav_layout", { order: [], hidden: [] });
  mockInvokeResponse("get_sidebar_collapsed", false);
  mockInvokeResponse("restore_projects", []);
  mockInvokeResponse("get_active_project_index", null);
  // MrPrList mounts and immediately calls refreshMrPrList → list_mr_prs.
  // Without a mock it resolves to undefined and crashes the filter.
  mockInvokeResponse("list_mr_prs", []);
  // Pretend a forge is connected so the provider-disconnect reroute in
  // navigation.ts doesn't bounce "merge-requests" back to "graph" while
  // onMount is finishing.
  providerStatus.set({
    providers: [{
      kind: "github",
      instance_url: "https://github.com",
      user: { id: 1, username: "u", display_name: "U", email: null, avatar_url: null, profile_url: "" },
      project_name: null,
    }],
    active_index: 0,
  });
});

describe("PR diff panel wiring", () => {
  it("opens the diff panel and renders DiffEditor on file click", async () => {
    // Set stores as if we'd opened PR #1 with one changed file.
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
    mrPrDiffFiles.set([
      { path: "a.ts", old_path: null, status: "modified", additions: 1, deletions: 0, patch: null },
    ]);
    selectedMrPrNumber.set(1);

    const { getByRole, container } = render(Page);
    // navigate — fake activeView change via store if needed
    const { activeViewStore } = await import("$lib/stores/navigation");
    activeViewStore.set("merge-requests");
    await waitFor(() => expect(container.querySelector(".mr-pr-layout")).toBeTruthy());

    const row = getByRole("button", { name: /a\.ts/ });
    await fireEvent.click(row);

    await waitFor(() => {
      expect(get(prFileDiff)).not.toBeNull();
      expect(container.querySelector(".diff-panel")).toBeTruthy();
    });
  });
});
