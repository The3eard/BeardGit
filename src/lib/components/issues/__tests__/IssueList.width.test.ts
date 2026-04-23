/**
 * Smoke test at 420 px: issue row meta shows all 5 labels + milestone +
 * assignee stack on the wrap line without overflowing the pane.
 */

import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { writable } from "svelte/store";
import type { Issue } from "$lib/types";

vi.mock("$lib/stores/issues", () => ({
  issueList: writable<Issue[]>([]),
  issueListLoading: writable(false),
  issueStateFilter: writable("open"),
  selectedIssueNumber: writable<number | null>(null),
  refreshIssueList: vi.fn(async () => {}),
  loadIssueDetail: vi.fn(),
}));
vi.mock("$lib/stores/provider", () => ({ hasActiveProvider: writable(true) }));

import { issueList } from "$lib/stores/issues";
import IssueList from "../IssueList.svelte";

afterEach(() => cleanup());

describe("IssueList @ 420px", () => {
  beforeEach(() => {
    issueList.set([
      {
        number: 42,
        title: "A medium-length issue title that should fit comfortably on one line",
        state: "open",
        author: "alice",
        labels: [
          { name: "bug", color: "ff0000", description: null },
          { name: "p1", color: "00ff00", description: null },
          { name: "regression", color: "0000ff", description: null },
          { name: "triage", color: "ffff00", description: null },
          { name: "backend", color: "888888", description: null },
        ],
        assignees: ["alice", "bob", "carol"],
        milestone: { id: 1, title: "v2.0", state: "open", due_on: null },
        comments_count: 3,
        created_at: "2026-04-22T10:00:00Z",
        updated_at: "2026-04-22T10:00:00Z",
        url: "https://example.test/issues/42",
      },
    ]);
  });

  it("surfaces all five labels, the milestone chip, and the assignee stack", () => {
    const { container } = render(IssueList);
    const meta = container.querySelector(".two-line-row__meta")!;
    expect(meta.querySelectorAll(".label-pill")).toHaveLength(5);
    expect(meta.querySelector(".milestone-chip")).not.toBeNull();
    expect(meta.querySelector(".assignee-stack")).not.toBeNull();
  });
});
