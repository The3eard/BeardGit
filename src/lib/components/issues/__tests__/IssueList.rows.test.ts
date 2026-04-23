/**
 * Regression tests for IssueList row rendering after the TwoLineRow migration.
 *
 * Seeds a mocked `issueList` store with issues that exercise:
 * - the "all labels rendered" guarantee (label cap removed).
 * - milestone pill visible when `issue.milestone` is non-null.
 * - AssigneeStack rendered when `issue.assignees.length > 0`, with +N overflow.
 */

import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { writable } from "svelte/store";
import type { Issue } from "$lib/types";

// Mock the stores the component imports so we can drive row content.
vi.mock("$lib/stores/issues", () => {
  const issueList = writable<Issue[]>([]);
  const issueListLoading = writable(false);
  const issueStateFilter = writable("open");
  const selectedIssueNumber = writable<number | null>(null);
  return {
    issueList,
    issueListLoading,
    issueStateFilter,
    selectedIssueNumber,
    refreshIssueList: vi.fn(async () => {}),
    loadIssueDetail: vi.fn(),
  };
});
vi.mock("$lib/stores/provider", () => ({
  hasActiveProvider: writable(true),
}));

import { issueList } from "$lib/stores/issues";
import IssueList from "../IssueList.svelte";

afterEach(() => cleanup());

const seed: Issue = {
  number: 1,
  title: "Fix the thing",
  state: "open",
  author: "alice",
  labels: [
    { name: "bug", color: "ff0000", description: null },
    { name: "p1", color: "00ff00", description: null },
    { name: "blocked", color: "0000ff", description: null },
    { name: "regression", color: "888888", description: null },
    { name: "triage", color: "ffff00", description: null },
  ],
  assignees: ["alice", "bob", "carol", "dave"],
  milestone: { id: 1, title: "v1.0", state: "open", due_on: null },
  comments_count: 7,
  created_at: "2026-04-22T10:00:00Z",
  updated_at: "2026-04-22T10:00:00Z",
  url: "https://example.test/issues/1",
};

describe("IssueList rows", () => {
  beforeEach(() => {
    issueList.set([seed]);
  });

  it("renders all 5 labels (no cap)", () => {
    const { container } = render(IssueList);
    expect(container.querySelectorAll(".label-pill")).toHaveLength(5);
    // Legacy `+N` label-overflow element must be gone
    expect(container.querySelector(".label-overflow")).toBeNull();
  });

  it("renders the milestone chip when milestone is non-null", () => {
    const { container } = render(IssueList);
    expect(container.querySelector(".milestone-chip")?.textContent).toContain("v1.0");
  });

  it("renders the AssigneeStack capped at 3 with +1 overflow", () => {
    const { container } = render(IssueList);
    expect(container.querySelectorAll(".assignee-avatar")).toHaveLength(3);
    expect(container.querySelector(".assignee-overflow")?.textContent).toBe("+1");
  });
});
