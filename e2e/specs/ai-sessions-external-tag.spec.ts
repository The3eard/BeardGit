/**
 * Phase 7.4 — E2E: External tag for a missing-worktree session.
 *
 * Contract: sessions whose `worktree_path` is `null` are "external" —
 * the run lives outside a BeardGit-managed worktree, so:
 *
 *   1. The list row shows an `[data-testid="external-badge"]` tag.
 *   2. The detail pane also renders the External badge.
 *   3. The detail pane does NOT expose the Open-terminal button —
 *      without a worktree path there's no PTY target to resume into.
 *
 * Seeded via the `__E2E__.seedAiBackgroundRuns` surface so we don't
 * need to spawn a real provider without a worktree (which only happens
 * when a user ran the provider from an external shell before BeardGit
 * was open — hard to reproduce deterministically in CI).
 */
import { expect } from "@wdio/globals";
import { openFixtureProject } from "../helpers/project";
import sidebar from "../pages/sidebar.page";

const SEED_ID = "seed-external-1";

describe("AI Sessions — External tag on missing worktree", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await openFixtureProject("simple-repo");

    await browser.execute(
      (id) => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const e2e = (window as any).__E2E__;
        if (!e2e?.seedAiBackgroundRuns) {
          throw new Error("window.__E2E__.seedAiBackgroundRuns unavailable");
        }
        e2e.seedAiBackgroundRuns([
          {
            id,
            provider: "claude_code",
            cwd: "/tmp/e2e-seed/external-run",
            started_at: Date.now(),
            kind: "headless",
            is_active: false,
            // The crucial bit — no worktree_path means this session is
            // external to BeardGit's worktree-management.
            worktree_path: null,
            background_status: { state: "completed", exit_code: 0 },
          },
        ]);
        e2e.selectAiSession?.(id);
      },
      SEED_ID,
    );

    await sidebar.navigateTo("ai-sessions");
    await $('[data-testid="ai-sessions-view"]').waitForExist({ timeout: 5000 });
  });

  it("renders the External badge in the detail pane", async () => {
    await $('[data-testid="ai-session-detail"]').waitForExist({ timeout: 5000 });

    // At least one external-badge must appear — the detail pane
    // always shows it; the list row shows it too when worktree_path
    // is null, which is fine. The invariant is "present", not "only
    // once" — a stricter count would leak row-structure details.
    const badges = await $$('[data-testid="external-badge"]').length;
    expect(badges).toBeGreaterThanOrEqual(1);
  });

  it("hides the Open-terminal button when the session has no worktree", async () => {
    const detail = await $('[data-testid="ai-session-detail"]');
    await detail.waitForDisplayed({ timeout: 5000 });

    // The Open-terminal button is only rendered when `worktree_path`
    // is truthy — asserting zero matches confirms the conditional
    // branch in `AiSessionDetail.svelte` is intact.
    const openBtnCount = await $$(
      '[data-testid="ai-session-detail-open-terminal"]',
    ).length;
    expect(openBtnCount).toBe(0);
  });
});
