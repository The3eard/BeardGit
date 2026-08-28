/**
 * Functional regression tests for the Changes view + sidebar edit mode.
 *
 * Covers the post-v0.2.0 bug reports:
 *  - sidebar edit mode (hide-only customisation) applies to the nav;
 *  - clicking a file in the changes lists highlights the row;
 *  - a file that appears after an external mutation still resolves its diff
 *    (the staged/unstaged stores must refresh with the statuses).
 *
 * Assertion-based — no screenshots.
 */

import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import {
  installBootstrapMocks,
  waitForAppReady,
  clickNav,
  type IpcResponses,
} from "./helpers";
import { patchMockResponses } from "./helpers/mock-ipc";
import {
  makeFileStatus,
  makeFileDiff,
  makeFileDiffStat,
  makeProjectInfo,
  makeStatusSummary,
} from "../../src/test/fixtures";

const PROJECT = makeProjectInfo({ name: "sample", head_branch: "main" });

const NULL_FLAGS = {
  refs_changed: false,
  head_changed: false,
  status_changed: false,
  stashes_changed: false,
  worktrees_changed: false,
  remotes_changed: false,
};

/**
 * Like emitMockEvent, but only fires callbacks registered for `event`
 * (looked up from the recorded `plugin:event|listen` calls), so other
 * listeners (theme, tasks, …) don't crash on a foreign payload shape.
 */
async function emitEventTargeted(
  page: Page,
  event: string,
  payload: unknown,
): Promise<void> {
  await page.evaluate(
    ({ event: e, payload: p }) => {
      const state = window.__beardgitMockIPC;
      if (!state) return;
      for (const call of state.calls) {
        if (call.cmd !== "plugin:event|listen") continue;
        const args = call.args as { event?: string; handler?: number };
        if (args?.event !== e || typeof args.handler !== "number") continue;
        const cb = state.callbacks.get(args.handler);
        cb?.({ event: e, id: 0, payload: p });
      }
    },
    { event, payload },
  );
}

function changesFixture(): IpcResponses {
  return {
    get_file_statuses: [
      makeFileStatus({ path: "src/a.ts", status: "modified", is_staged: false }),
      makeFileStatus({ path: "src/staged.ts", status: "modified", is_staged: true }),
    ],
    get_status_summary: makeStatusSummary({ staged: 1, unstaged: 1 }),
    // The Changes lists are driven by the lightweight per-file stats; the
    // full hunks/lines diff of the opened file is fetched lazily.
    get_diff_stats_workdir: [makeFileDiffStat({ path: "src/a.ts" })],
    get_diff_stats_index: [makeFileDiffStat({ path: "src/staged.ts" })],
    get_diff_file: makeFileDiff({ path: "src/a.ts" }),
  };
}

test.describe("sidebar edit mode", () => {
  test.beforeEach(async ({ page }) => {
    await installBootstrapMocks(page, { activeProject: PROJECT });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("the pencil reveals per-item hide toggles", async ({ page }) => {
    // Hidden items only appear (greyed, with their eye toggle) in edit mode.
    await expect(page.getByTestId("sidebar-hide-graph")).toBeHidden();
    await page.getByTestId("sidebar-edit-toggle").click();
    await expect(page.getByTestId("sidebar-edit-done")).toBeVisible();
    await expect(page.getByTestId("sidebar-hide-graph")).toBeVisible();
  });

  test("hiding an item drops it from the navigation list", async ({ page }) => {
    await expect(page.getByTestId("nav-changes")).toBeVisible();
    await page.getByTestId("sidebar-edit-toggle").click();
    await page.getByTestId("sidebar-hide-changes").click();
    await page.getByTestId("sidebar-edit-done").click();
    await expect(page.getByTestId("nav-changes")).toHaveCount(0);
  });
});

test.describe("changes view", () => {
  test.beforeEach(async ({ page }) => {
    await installBootstrapMocks(page, {
      activeProject: PROJECT,
      extra: changesFixture(),
    });
    await page.goto("/");
    await waitForAppReady(page);
    await clickNav(page, "Changes");
  });

  test("clicking an unstaged file shows its diff", async ({ page }) => {
    await page.getByTestId("file-row-src-a.ts").locator(".file-btn").click();
    await expect(page.locator(".staging-diff-editor")).toBeVisible();
  });

  test("clicking a staged file shows its diff", async ({ page }) => {
    await page.getByTestId("file-row-src-staged.ts").locator(".file-btn").click();
    await expect(page.locator(".staging-diff-editor")).toBeVisible();
  });

  test("a file appearing after an external mutation still resolves its diff", async ({ page }) => {
    // External edit adds src/b.ts: the backend now reports it in both the
    // statuses and the workdir diff; the watcher pipeline emits
    // project-mutated with status_changed.
    await patchMockResponses(page, {
      get_file_statuses: [
        makeFileStatus({ path: "src/a.ts", status: "modified", is_staged: false }),
        makeFileStatus({ path: "src/b.ts", status: "modified", is_staged: false }),
        makeFileStatus({ path: "src/staged.ts", status: "modified", is_staged: true }),
      ],
      get_diff_stats_workdir: [
        makeFileDiffStat({ path: "src/a.ts" }),
        makeFileDiffStat({ path: "src/b.ts" }),
      ],
    });
    await emitEventTargeted(page, "project-mutated", {
      project_path: PROJECT.path,
      kind: { type: "external" },
      flags: { ...NULL_FLAGS, status_changed: true },
    });
    const rowB = page.getByTestId("file-row-src-b.ts");
    await expect(rowB).toBeVisible();
    await rowB.locator(".file-btn").click();
    await expect(page.locator(".staging-diff-editor")).toBeVisible();
  });

  test("clicking a file highlights its row in the list", async ({ page }) => {
    const row = page.getByTestId("file-row-src-a.ts");
    await row.locator(".file-btn").click();
    await expect(page.locator(".staging-diff-editor")).toBeVisible();
    await expect(row).toHaveClass(/selected/);
    // Selecting the other list's file moves the highlight.
    const stagedRow = page.getByTestId("file-row-src-staged.ts");
    await stagedRow.locator(".file-btn").click();
    await expect(stagedRow).toHaveClass(/selected/);
    await expect(row).not.toHaveClass(/selected/);
  });
});
