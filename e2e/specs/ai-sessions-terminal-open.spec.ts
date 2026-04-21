/**
 * Phase 7.3 — E2E: Open terminal mounts a PTY tab.
 *
 * Contract: clicking "Open terminal" in the AI Session detail pane
 * must invoke the `ai_open_background_terminal` Tauri command with
 * `{ sessionId: <id> }` — the same command the background-runs
 * resume path uses. A missing or misspelled command name here would
 * regress the entire resume-from-worktree flow.
 *
 * Why we intercept instead of exercising the real backend: the seeded
 * session only lives in the frontend store, so the Rust coordinator
 * returns "session not found" when called for real. We install an
 * invoke-spy on `window.__TAURI_INTERNALS__.invoke` that records the
 * call, short-circuits the real round-trip for this one command, and
 * otherwise delegates back to the native invoker. That way we verify
 * the wiring without spawning a real AI provider (which would need a
 * mock binary in the E2E image — see `regression/ai-background.spec.ts`
 * for the equivalent dialog-side caveat).
 */
import { expect } from "@wdio/globals";
import { openFixtureProject } from "../helpers/project";
import sidebar from "../pages/sidebar.page";

const SEED_ID = "seed-term-1";
const SEED_WT = "/tmp/e2e-seed/terminal/.wt/ai-term";

describe("AI Sessions — Open terminal invokes PTY command", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await openFixtureProject("simple-repo");

    await browser.execute(
      (payload) => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const e2e = (window as any).__E2E__;
        if (!e2e?.seedAiBackgroundRuns) {
          throw new Error("window.__E2E__.seedAiBackgroundRuns unavailable");
        }
        e2e.seedAiBackgroundRuns([
          {
            id: payload.id,
            provider: "claude_code",
            cwd: "/tmp/e2e-seed/terminal",
            started_at: Date.now(),
            kind: "headless",
            is_active: false,
            worktree_path: payload.wt,
            background_status: { state: "completed", exit_code: 0 },
          },
        ]);
        e2e.selectAiSession?.(payload.id);

        // Install an invoke spy for `ai_open_background_terminal`.
        // Mutating window.__TAURI_INTERNALS__.invoke in place means the
        // wrapper survives code-splitting and lazy imports — every
        // caller in the frontend ends up here.
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const internals = (window as any).__TAURI_INTERNALS__;
        if (!internals || typeof internals.invoke !== "function") {
          throw new Error("window.__TAURI_INTERNALS__.invoke unavailable");
        }
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const w = window as any;
        w.__E2E_TERMINAL_INVOKES__ = [];
        const originalInvoke = internals.invoke.bind(internals);
        internals.invoke = (cmd: string, args?: Record<string, unknown>) => {
          if (cmd === "ai_open_background_terminal") {
            w.__E2E_TERMINAL_INVOKES__.push({ cmd, args });
            // Return a synthetic tab id so the frontend success path
            // (toast + mutation event) runs without hitting the Rust
            // coordinator for the seeded session.
            return Promise.resolve(42);
          }
          return originalInvoke(cmd, args);
        };
      },
      { id: SEED_ID, wt: SEED_WT },
    );

    await sidebar.navigateTo("ai-sessions");
    await $('[data-testid="ai-sessions-view"]').waitForExist({ timeout: 5000 });
  });

  it("records an invoke of ai_open_background_terminal with the session id", async () => {
    const btn = await $('[data-testid="ai-session-detail-open-terminal"]');
    await btn.waitForDisplayed({ timeout: 5000 });
    await btn.click();

    // `runMutation` is async — give the microtask queue a chance to
    // drain so the invoke lands in the spy array before we read it.
    await browser.waitUntil(
      async () => {
        const count = await browser.execute(
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          () => ((window as any).__E2E_TERMINAL_INVOKES__ ?? []).length,
        );
        return count >= 1;
      },
      { timeout: 3000, timeoutMsg: "no ai_open_background_terminal invoke recorded" },
    );

    const calls = (await browser.execute(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      () => (window as any).__E2E_TERMINAL_INVOKES__ ?? [],
    )) as Array<{ cmd: string; args: { sessionId?: string } }>;
    expect(calls).toHaveLength(1);
    expect(calls[0].cmd).toBe("ai_open_background_terminal");
    expect(calls[0].args?.sessionId).toBe(SEED_ID);

    // The success path must not have raised a sticky error toast —
    // same no-error contract as the commit-path spec.
    const errorToasts = await $$(
      '[data-testid="toast"][data-type="error"]',
    ).length;
    expect(errorToasts).toBe(0);
  });
});
