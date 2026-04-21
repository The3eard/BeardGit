/**
 * Contract test: the "Open terminal here" button in `AiSessionDetail`
 * routes through `runMutation` so every terminal-open attempt produces
 * a success toast on resolve and a sticky failure toast on reject.
 * Also asserts the button is hidden when the session's worktree is not
 * reachable (External sessions — Phase 3 row-layout rule).
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { AiSession } from "$lib/types";

const { runMutation, openTerminalForAiBackgroundSession } = vi.hoisted(() => ({
  runMutation: vi.fn(async (opts: { invoke: () => Promise<unknown> }) =>
    opts.invoke(),
  ),
  openTerminalForAiBackgroundSession: vi.fn(),
}));

vi.mock("$lib/api/runMutation", () => ({
  runMutation,
}));

vi.mock("$lib/stores/aiBackground", async () => {
  const actual = await vi.importActual<object>("$lib/stores/aiBackground");
  return { ...actual, openTerminalForAiBackgroundSession };
});

import {
  aiBackgroundRuns,
  selectedBackgroundSessionId,
} from "$lib/stores/aiBackground";
import AiSessionDetail from "../AiSessionDetail.svelte";

const FIXTURE: AiSession = {
  id: "s-term",
  provider: "codex",
  cwd: "/repos/demo",
  started_at: 0,
  kind: "headless",
  is_active: false,
  worktree_path: "/repos/demo/.wt/ai-term",
  background_status: { state: "completed", exit_code: 0 },
};

afterEach(() => {
  cleanup();
  runMutation.mockClear();
  openTerminalForAiBackgroundSession.mockReset();
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
});

describe("Open terminal wiring", () => {
  it("clicking Open terminal invokes runMutation with openTerminalForAiBackgroundSession", async () => {
    aiBackgroundRuns.set(new Map([[FIXTURE.id, FIXTURE]]));
    selectedBackgroundSessionId.set(FIXTURE.id);
    openTerminalForAiBackgroundSession.mockResolvedValue(42);

    const { getByText } = render(AiSessionDetail);
    await tick();
    await fireEvent.click(getByText(/open terminal/i));

    expect(runMutation).toHaveBeenCalledTimes(1);
    expect(openTerminalForAiBackgroundSession).toHaveBeenCalledWith("s-term");
  });

  it("surfaces errors through runMutation without throwing when the PTY call rejects", async () => {
    aiBackgroundRuns.set(new Map([[FIXTURE.id, FIXTURE]]));
    selectedBackgroundSessionId.set(FIXTURE.id);
    openTerminalForAiBackgroundSession.mockRejectedValue(new Error("pty boom"));

    const { getByText } = render(AiSessionDetail);
    await tick();
    // Swallow the rejection inside the runMutation mock so the test assertions
    // stay readable while still observing that the handler delegated.
    runMutation.mockImplementationOnce(async (opts: { invoke: () => Promise<unknown> }) => {
      try {
        return await opts.invoke();
      } catch {
        return undefined;
      }
    });
    await fireEvent.click(getByText(/open terminal/i));

    expect(runMutation).toHaveBeenCalledTimes(1);
    expect(openTerminalForAiBackgroundSession).toHaveBeenCalledWith("s-term");
  });

  it("hides the Open terminal button when worktree_path is missing", async () => {
    aiBackgroundRuns.set(
      new Map([[FIXTURE.id, { ...FIXTURE, worktree_path: null }]]),
    );
    selectedBackgroundSessionId.set(FIXTURE.id);
    const { queryByText } = render(AiSessionDetail);
    await tick();
    expect(queryByText(/open terminal/i)).toBeNull();
  });
});
