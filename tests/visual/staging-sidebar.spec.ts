/**
 * Functional regression tests for the Changes view + sidebar reorder.
 *
 * Covers the post-v0.2.0 bug reports:
 *  - sidebar edit-mode reorder (mouse drag + keyboard) applies and persists;
 *  - clicking a file in the changes lists highlights the row;
 *  - a file that appears after an external mutation still resolves its diff
 *    (the staged/unstaged FileDiff stores must refresh with the statuses).
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
      makeFileStatus({ path: "src/a.ts", status: "M", is_staged: false }),
      makeFileStatus({ path: "src/staged.ts", status: "M", is_staged: true }),
    ],
    get_status_summary: makeStatusSummary({ staged: 1, unstaged: 1 }),
    get_diff_workdir: [makeFileDiff({ path: "src/a.ts" })],
    get_diff_index: [makeFileDiff({ path: "src/staged.ts" })],
  };
}

async function navOrder(page: Page): Promise<string[]> {
  return page.$$eval('[data-testid^="nav-"]', (els) =>
    els
      .map((e) => e.getAttribute("data-testid")!)
      .filter((id) => id !== "nav-settings"),
  );
}

test.describe("sidebar reorder (edit mode)", () => {
  test.beforeEach(async ({ page }) => {
    await installBootstrapMocks(page, { activeProject: PROJECT });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("keyboard ArrowDown moves the focused item", async ({ page }) => {
    await page.getByTestId("sidebar-edit-toggle").click();
    const before = await navOrder(page);
    await page.getByTestId("sidebar-reorder-graph").focus();
    await page.keyboard.press("ArrowDown");
    const after = await navOrder(page);
    expect(after.indexOf("nav-graph")).toBe(before.indexOf("nav-graph") + 1);
  });

  test("mouse drag moves the item to the hovered row", async ({ page }) => {
    await page.getByTestId("sidebar-edit-toggle").click();
    await page.getByTestId("nav-graph").dragTo(page.getByTestId("nav-editor"));
    const after = await navOrder(page);
    // graph lands AT editor's old slot (index 2 of the default order).
    expect(after.indexOf("nav-graph")).toBe(2);
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
        makeFileStatus({ path: "src/a.ts", status: "M", is_staged: false }),
        makeFileStatus({ path: "src/b.ts", status: "M", is_staged: false }),
        makeFileStatus({ path: "src/staged.ts", status: "M", is_staged: true }),
      ],
      get_diff_workdir: [
        makeFileDiff({ path: "src/a.ts" }),
        makeFileDiff({ path: "src/b.ts" }),
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
