/**
 * Contract test: `AiSessionDetail` populates its header from the
 * `selectedBackgroundSession` derived store. Seeds `aiBackgroundRuns` with one
 * fixture, writes its id into `selectedBackgroundSessionId`, and asserts the
 * detail pane renders the worktree path. Also covers the empty-state branch
 * when no session is selected.
 */
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";
import {
  aiBackgroundRuns,
  selectedBackgroundSessionId,
} from "$lib/stores/aiBackground";
import type { AiSession } from "$lib/types";
import AiSessionDetail from "../AiSessionDetail.svelte";

const FIXTURE: AiSession = {
  id: "s-abc",
  provider: "claude_code",
  cwd: "/repos/demo",
  started_at: 1_700_000_000,
  kind: "headless",
  is_active: true,
  worktree_path: "/repos/demo/.wt/ai-s-abc",
  background_status: { state: "running" },
};

afterEach(() => {
  cleanup();
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
});

describe("AiSessionDetail on selection", () => {
  it("populates header with worktree path when a session is selected", async () => {
    aiBackgroundRuns.set(new Map([[FIXTURE.id, FIXTURE]]));
    selectedBackgroundSessionId.set(FIXTURE.id);
    const { container } = render(AiSessionDetail);
    await tick();
    expect(container.textContent).toContain("/repos/demo/.wt/ai-s-abc");
  });

  it("renders an empty state when no session is selected", async () => {
    selectedBackgroundSessionId.set(null);
    const { container } = render(AiSessionDetail);
    await tick();
    expect(container.querySelector(".empty")).toBeTruthy();
  });
});
