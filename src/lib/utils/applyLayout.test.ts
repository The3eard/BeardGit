import { describe, it, expect } from "vitest";
import { applyLayout, DEFAULT_ORDER } from "./applyLayout";
import type { SidebarNavItem } from "./applyLayout";

const ITEMS: SidebarNavItem[] = [
  { id: "graph", label: "Graph", icon: "" },
  { id: "changes", label: "Changes", icon: "" },
  { id: "branches", label: "Branches", icon: "" },
  { id: "tags", label: "Tags", icon: "" },
  { id: "stashes", label: "Stashes", icon: "" },
  { id: "worktrees", label: "Worktrees", icon: "" },
  { id: "reflog", label: "Reflog", icon: "" },
  { id: "bisect", label: "Bisect", icon: "" },
  { id: "submodules", label: "Submodules", icon: "" },
  { id: "ai-config", label: "AI Config", icon: "" },
  { id: "ai-sessions", label: "AI Sessions", icon: "" },
  { id: "requests", label: "Requests", icon: "" },
];

describe("applyLayout", () => {
  it("DEFAULT_ORDER contains the canonical ids in the documented order", () => {
    expect(DEFAULT_ORDER).toEqual([
      "graph",
      "changes",
      "branches",
      "tags",
      "stashes",
      "worktrees",
      "reflog",
      "bisect",
      "submodules",
      "ai-config",
      "ai-sessions",
      "requests",
    ]);
  });

  it("returns items in DEFAULT_ORDER when saved order is empty", () => {
    const out = applyLayout(ITEMS, [], []);
    expect(out.map((i) => i.id)).toEqual(DEFAULT_ORDER);
  });

  it("applies the saved order when provided", () => {
    const saved = ["changes", "graph", "branches"];
    const out = applyLayout(ITEMS, saved, []);
    // Saved order first, then any default ids not in the saved list.
    expect(out.slice(0, 3).map((i) => i.id)).toEqual([
      "changes",
      "graph",
      "branches",
    ]);
    expect(out.length).toBe(ITEMS.length);
  });

  it("filters hidden ids out of the result", () => {
    const out = applyLayout(ITEMS, [], ["bisect", "reflog"]);
    const ids = out.map((i) => i.id);
    expect(ids).not.toContain("bisect");
    expect(ids).not.toContain("reflog");
    expect(ids.length).toBe(ITEMS.length - 2);
  });

  it("ignores unknown ids in the saved order (resilient to renames)", () => {
    const out = applyLayout(ITEMS, ["ghost", "graph", "also-ghost"], []);
    // "graph" stays first, and the remaining default ids are appended.
    expect(out[0]?.id).toBe("graph");
    expect(out.length).toBe(ITEMS.length);
    expect(out.every((i) => ITEMS.some((r) => r.id === i.id))).toBe(true);
  });

  it("appends new ids not present in the saved order at the end", () => {
    // Simulate an old saved order from a release before "requests" existed.
    const savedLegacy = DEFAULT_ORDER.filter((id) => id !== "requests");
    const out = applyLayout(ITEMS, savedLegacy, []);
    expect(out.map((i) => i.id)).toEqual(DEFAULT_ORDER);
    expect(out[out.length - 1]?.id).toBe("requests");
  });

  it("combines order + hidden correctly", () => {
    const saved = ["changes", "graph", "ai-sessions"];
    const hidden = ["graph", "submodules"];
    const out = applyLayout(ITEMS, saved, hidden);
    const ids = out.map((i) => i.id);
    expect(ids[0]).toBe("changes");
    expect(ids).not.toContain("graph");
    expect(ids).not.toContain("submodules");
    // All remaining non-hidden default ids are still present.
    expect(ids.length).toBe(ITEMS.length - 2);
  });
});
