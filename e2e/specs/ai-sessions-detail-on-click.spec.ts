/**
 * Phase 7.2 — E2E: AI Sessions detail populates on row click.
 *
 * Contract: clicking a row in the session list must write its id into
 * `selectedBackgroundSessionId`, which drives the detail pane through
 * the `selectedBackgroundSession` derived store. The worktree path of
 * the clicked session must appear in the detail pane's header.
 *
 * Unit coverage: `AiSessionDetail.selection.test.ts` exercises the
 * derived-store wiring in isolation; this spec proves the real click
 * event in the webview flips the same switch end-to-end.
 *
 * Strategy: seed two sessions with distinct worktree paths, click the
 * first row, assert the detail pane's `[data-testid="ai-session-detail-wt-path"]`
 * shows the first session's path — not the second.
 */
import { expect } from "@wdio/globals";
import { openFixtureProject } from "../helpers/project";
import sidebar from "../pages/sidebar.page";

const FIRST_WT = "/tmp/e2e-seed/detail-first/.wt/ai-first";
const SECOND_WT = "/tmp/e2e-seed/detail-second/.wt/ai-second";

describe("AI Sessions detail populates on click", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await openFixtureProject("simple-repo");

    await browser.execute(
      (paths) => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const e2e = (window as any).__E2E__;
        if (!e2e?.seedAiBackgroundRuns) {
          throw new Error("window.__E2E__.seedAiBackgroundRuns unavailable");
        }
        e2e.seedAiBackgroundRuns([
          {
            id: "seed-detail-1",
            provider: "claude_code",
            cwd: "/tmp/e2e-seed/detail-first",
            started_at: Date.now(),
            kind: "headless",
            is_active: false,
            worktree_path: paths.first,
            background_status: { state: "completed", exit_code: 0 },
          },
          {
            id: "seed-detail-2",
            provider: "codex",
            cwd: "/tmp/e2e-seed/detail-second",
            started_at: Date.now(),
            kind: "headless",
            is_active: false,
            worktree_path: paths.second,
            background_status: { state: "completed", exit_code: 0 },
          },
        ]);
        // Clear any pre-existing selection so the detail pane starts
        // on the empty-state branch — gives the assertion signal a
        // clean edge to detect.
        e2e.selectAiSession?.(null);
      },
      { first: FIRST_WT, second: SECOND_WT },
    );

    await sidebar.navigateTo("ai-sessions");
    await $('[data-testid="ai-sessions-view"]').waitForExist({ timeout: 5000 });
  });

  it("clicking the first row shows its worktree path in the detail pane", async () => {
    // Wait for both rows to render. The seed runs before navigation so
    // the list should be populated on first paint, but allow a beat
    // for the store subscription to flush.
    await browser.waitUntil(
      async () => (await $$('[data-testid="ai-session-row"]').length) >= 2,
      { timeout: 5000, timeoutMsg: "expected at least two session rows" },
    );

    const firstRow = await $('[data-testid="ai-session-row"][data-session-id="seed-detail-1"]');
    await firstRow.waitForDisplayed({ timeout: 5000 });
    await firstRow.click();

    const wtPath = await $('[data-testid="ai-session-detail-wt-path"]');
    await wtPath.waitForDisplayed({ timeout: 5000 });
    const text = await wtPath.getText();
    expect(text).toContain(FIRST_WT);
    expect(text).not.toContain(SECOND_WT);
  });
});
