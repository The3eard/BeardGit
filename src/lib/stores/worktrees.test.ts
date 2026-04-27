import { describe, it, expect } from "vitest";
import type { WorktreeInfo, AiWorktree } from "$lib/types";

// We'll test the join function directly
import { enrichWorktrees } from "./worktrees";

describe("enrichWorktrees", () => {
  const baseWorktrees: WorktreeInfo[] = [
    { path: "/repo", branch: "main", head_oid: "abc", is_main: true, is_locked: false },
    { path: "/repo-feat", branch: "feature/auth", head_oid: "def", is_main: false, is_locked: false },
    { path: "/repo-wt-api", branch: "worktree-refactor-api", head_oid: "ghi", is_main: false, is_locked: false },
  ];

  const aiWorktrees: AiWorktree[] = [
    { path: "/repo-wt-api", branch: "worktree-refactor-api", provider: "claude_code", session_id: "sess-1", status: "active" },
  ];

  it("enriches matching worktrees with AI data", () => {
    const result = enrichWorktrees(baseWorktrees, aiWorktrees);
    const aiWt = result.find((w) => w.path === "/repo-wt-api");
    expect(aiWt?.ai_provider).toBe("claude_code");
    expect(aiWt?.ai_status).toBe("active");
    expect(aiWt?.ai_session_id).toBe("sess-1");
  });

  it("leaves non-AI worktrees with null AI fields", () => {
    const result = enrichWorktrees(baseWorktrees, aiWorktrees);
    const regular = result.find((w) => w.path === "/repo-feat");
    expect(regular?.ai_provider).toBeNull();
    expect(regular?.ai_status).toBeNull();
    expect(regular?.ai_session_id).toBeNull();
  });

  it("sorts: current first, then AI active, then alphabetical", () => {
    const result = enrichWorktrees(baseWorktrees, aiWorktrees);
    expect(result[0].is_main).toBe(true);
    expect(result[1].ai_status).toBe("active");
    expect(result[2].branch).toBe("feature/auth");
  });

  it("handles empty AI worktrees", () => {
    const result = enrichWorktrees(baseWorktrees, []);
    expect(result).toHaveLength(3);
    result.forEach((w) => {
      expect(w.ai_provider).toBeNull();
    });
  });
});
