/**
 * Smoke test for the mock-IPC harness — validates that `installMockIPC`
 * actually intercepts `invoke()` calls from the SvelteKit bundle, not
 * just that the helper functions don't throw.
 *
 * The harness is detected via `window.__beardgitMockIPC` rather than
 * a `<html>` data attribute because SvelteKit hydration rewrites the
 * `<html>` element and any pre-bundle attributes would be lost.
 */

import { expect, test } from "@playwright/test";

import {
  applyTheme,
  getMockCalls,
  installMockIPC,
  patchMockResponses,
} from "./helpers";
import {
  makeRecentRepo,
  makeStatusSummary,
} from "../../src/test/fixtures";

test.describe("mock IPC smoke", () => {
  test("init script defines window.__TAURI_INTERNALS__ before bundle runs", async ({ page }) => {
    await installMockIPC(page, {
      get_recent_repos: [
        makeRecentRepo({ name: "alpha", path: "/repos/alpha" }),
        makeRecentRepo({ name: "beta", path: "/repos/beta" }),
      ],
      get_status_summary: makeStatusSummary({ staged: 2, unstaged: 1 }),
    });

    await page.goto("/");

    const harness = await page.evaluate(() => {
      const internals = window.__TAURI_INTERNALS__ as
        | {
            invoke?: unknown;
            transformCallback?: unknown;
            unregisterCallback?: unknown;
            convertFileSrc?: unknown;
          }
        | undefined;
      const state = window.__beardgitMockIPC;
      return {
        hasInvoke: typeof internals?.invoke === "function",
        hasTransformCallback: typeof internals?.transformCallback === "function",
        hasUnregisterCallback: typeof internals?.unregisterCallback === "function",
        hasConvertFileSrc: typeof internals?.convertFileSrc === "function",
        hasState: !!state,
        registeredCommands: state ? Object.keys(state.responses).sort() : [],
      };
    });

    expect(harness).toEqual({
      hasInvoke: true,
      hasTransformCallback: true,
      hasUnregisterCallback: true,
      hasConvertFileSrc: true,
      hasState: true,
      registeredCommands: ["get_recent_repos", "get_status_summary"],
    });
  });

  test("invoke() resolves against the response map", async ({ page }) => {
    await installMockIPC(page);
    await page.goto("/");

    await patchMockResponses(page, {
      get_status_summary: makeStatusSummary({ staged: 5 }),
    });

    const result = await page.evaluate(async () => {
      const internals = window.__TAURI_INTERNALS__ as {
        invoke: (cmd: string, args?: unknown) => Promise<unknown>;
      };
      return internals.invoke("get_status_summary");
    });
    expect(result).toEqual(
      expect.objectContaining({ staged: 5, unstaged: 0 }),
    );
  });

  test("captures call history with args", async ({ page }) => {
    await installMockIPC(page, {
      open_repo: { path: "/x", head_branch: "main", head_oid: null, branch_count: 1 },
    });
    await page.goto("/");

    await page.evaluate(async () => {
      const internals = window.__TAURI_INTERNALS__ as {
        invoke: (cmd: string, args?: unknown) => Promise<unknown>;
      };
      await internals.invoke("open_repo", { path: "/x" });
      await internals.invoke("open_repo", { path: "/y" });
    });

    const calls = await getMockCalls(page, "open_repo");
    expect(calls).toHaveLength(2);
    expect(calls[0]).toMatchObject({ cmd: "open_repo", args: { path: "/x" } });
    expect(calls[1]).toMatchObject({ cmd: "open_repo", args: { path: "/y" } });
    expect(calls[1].index).toBeGreaterThan(calls[0].index);
  });

  test("applyTheme triggers theme tokens to land on <html>", async ({ page }) => {
    await installMockIPC(page);
    await page.goto("/");
    await applyTheme(page, "dark");

    const accent = await page.evaluate(() =>
      getComputedStyle(document.documentElement)
        .getPropertyValue("--overlay-accent-blue")
        .trim(),
    );
    expect(accent.length).toBeGreaterThan(0);
  });

});
