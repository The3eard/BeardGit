import { describe, it, expect } from "vitest";
import { filterIssuesLocal } from "./issue-provider";
import type { Issue } from "../types";

function mkIssue(overrides: Partial<Issue>): Issue {
  return {
    number: 1,
    title: "t",
    state: "open",
    author: "a",
    labels: [],
    assignees: [],
    milestone: null,
    comments_count: 0,
    created_at: "",
    updated_at: "",
    url: "",
    ...overrides,
  };
}

describe("filterIssuesLocal", () => {
  it("empty tags returns all", () => {
    const items = [mkIssue({ number: 1 }), mkIssue({ number: 2 })];
    expect(filterIssuesLocal(items, [])).toHaveLength(2);
  });

  it("filters by author substring (case-insensitive)", () => {
    const items = [
      mkIssue({ author: "alice" }),
      mkIssue({ author: "bob" }),
    ];
    const r = filterIssuesLocal(items, [
      { id: "t", type: "author", value: "ALI", display: "" },
    ]);
    expect(r).toHaveLength(1);
    expect(r[0].author).toBe("alice");
  });

  it("filters by label name", () => {
    const items = [
      mkIssue({
        labels: [{ name: "bug", color: "d73a4a", description: null }],
      }),
      mkIssue({
        labels: [{ name: "docs", color: "0075ca", description: null }],
      }),
    ];
    expect(
      filterIssuesLocal(items, [
        { id: "t", type: "label", value: "bug", display: "" },
      ]),
    ).toHaveLength(1);
  });

  it("filters by state exactly", () => {
    const items = [mkIssue({ state: "open" }), mkIssue({ state: "closed" })];
    expect(
      filterIssuesLocal(items, [
        { id: "t", type: "state", value: "closed", display: "" },
      ]),
    ).toHaveLength(1);
  });

  it("filters by assignee", () => {
    const items = [
      mkIssue({ assignees: ["alice", "bob"] }),
      mkIssue({ assignees: ["carol"] }),
    ];
    expect(
      filterIssuesLocal(items, [
        { id: "t", type: "assignee", value: "bob", display: "" },
      ]),
    ).toHaveLength(1);
  });

  it("filters by milestone title", () => {
    const items = [
      mkIssue({
        milestone: { id: 1, title: "v1.0", state: "open", due_on: null },
      }),
      mkIssue({ milestone: null }),
    ];
    expect(
      filterIssuesLocal(items, [
        { id: "t", type: "milestone", value: "v1", display: "" },
      ]),
    ).toHaveLength(1);
  });

  it("text tag matches number or title", () => {
    const items = [mkIssue({ number: 42, title: "Crash" })];
    expect(
      filterIssuesLocal(items, [
        { id: "t", type: "text", value: "42", display: "" },
      ]),
    ).toHaveLength(1);
    expect(
      filterIssuesLocal(items, [
        { id: "t", type: "text", value: "crash", display: "" },
      ]),
    ).toHaveLength(1);
  });

  it("AND-composes multiple tags", () => {
    const items = [
      mkIssue({ author: "alice", state: "open" }),
      mkIssue({ author: "alice", state: "closed" }),
    ];
    expect(
      filterIssuesLocal(items, [
        { id: "1", type: "author", value: "alice", display: "" },
        { id: "2", type: "state", value: "closed", display: "" },
      ]),
    ).toHaveLength(1);
  });
});
