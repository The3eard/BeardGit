/**
 * Phase 7.1 — E2E: AI Sessions list paints fast.
 *
 * Contract: clicking the AI Sessions sidebar entry must mount the view
 * shell (and therefore the list panel) on the first frame, even when
 * the backend fetch that populates the list is slow. The Phase-2 fix
 * moved the initial `refreshAiBackgroundRuns()` off the critical path
 * (fire-and-forget inside `onMount`) — this spec is the end-to-end
 * belt-and-braces assertion that the shell is visible before the IPC
 * round-trip can possibly have resolved.
 *
 * Strategy:
 *  1. Seed two synthetic sessions via the `__E2E__` surface so the
 *     view has content and we can distinguish "shell without data"
 *     from "fully rendered list".
 *  2. Navigate to AI Sessions via the sidebar.
 *  3. Assert the `[data-testid="ai-sessions-view"]` shell exists
 *     within a tight window — it must beat any multi-hundred-ms
 *     backend latency.
 *
 * Covered unit-side by `AiSessionsView.async-first.test.ts` (vitest);
 * this spec guards the real Tauri webview where rAF scheduling and
 * CSS layout could still regress the first-paint contract.
 */
import { expect } from "@wdio/globals";
import { openFixtureProject } from "../helpers/project";
import sidebar from "../pages/sidebar.page";

describe("AI Sessions list paints fast", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await openFixtureProject("simple-repo");

    // Seed two synthetic sessions before the user navigates. The
    // `__E2E__.seedAiBackgroundRuns` surface writes directly into the
    // Svelte store so the view has content even though no real AI
    // provider ran in this test image.
    await browser.execute(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const e2e = (window as any).__E2E__;
      if (!e2e?.seedAiBackgroundRuns) {
        throw new Error("window.__E2E__.seedAiBackgroundRuns unavailable");
      }
      e2e.seedAiBackgroundRuns([
        {
          id: "seed-fast-1",
          provider: "claude_code",
          cwd: "/tmp/e2e-seed/first",
          started_at: Date.now(),
          kind: "headless",
          is_active: true,
          worktree_path: "/tmp/e2e-seed/first/.wt/ai-1",
          background_status: { state: "running" },
        },
        {
          id: "seed-fast-2",
          provider: "codex",
          cwd: "/tmp/e2e-seed/second",
          started_at: Date.now(),
          kind: "headless",
          is_active: false,
          worktree_path: "/tmp/e2e-seed/second/.wt/ai-2",
          background_status: { state: "completed", exit_code: 0 },
        },
      ]);
    });
  });

  it("mounts the sessions shell on tab click", async () => {
    // `.navigateTo` clicks the sidebar entry and waits a short grace
    // period; the shell must appear inside that window.
    await sidebar.navigateTo("ai-sessions");

    const shell = await $('[data-testid="ai-sessions-view"]');
    // Tight timeout — the shell mount is synchronous inside the route
    // component, so 2 s is generous even on slow CI runners.
    await shell.waitForExist({ timeout: 2000 });
    await expect(shell).toBeDisplayed();

    // The list should also be populated from the seed — proves the
    // writable store drove the render path without waiting on IPC.
    const rows = await $$('[data-testid="ai-session-row"]').length;
    expect(rows).toBeGreaterThanOrEqual(2);
  });
});
