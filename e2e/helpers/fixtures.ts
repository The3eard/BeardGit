/**
 * Phase-11 fixture helpers for the reactivity & feedback E2E suite.
 *
 * Wraps `project.ts` with task-specific affordances the new specs need:
 *
 *  - `openAnyFixtureRepo` — same fixture (`simple-repo`) every spec
 *    uses so fixtures run in isolation and don't bleed between specs.
 *  - `fixtureRepoPath` — resolves the active project's absolute path
 *    so specs can shell out with `execSync(\`git -C <path> …\`)`
 *    (used by the external-CLI refresh spec).
 *  - `stageAllAndCommit` — shortest path from a dirty worktree to a
 *    new commit through the UI (mirrors what a user types — staging
 *    button + commit textarea + commit button).
 *  - `openFixtureRepoWithOrigin` — creates an ad-hoc temp repo with
 *    a specific `origin` remote URL so the statusbar provider-filter
 *    spec can assert "GitHub-only → github pill / GitLab-only →
 *    gitlab pill" without mutating the canonical simple-repo.
 *  - `restartApp` — tauri-driver can't reboot the binary mid-session,
 *    so this helper approximates "cold start" by closing every open
 *    project tab and re-opening the fixture. Combined with the
 *    project-cache slice (`ProjectSnapshot.graph_viewport_cache`) it
 *    exercises the cache-first paint path.
 */

import path from "node:path";
import { execSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import {
  FIXTURE_ROOT,
  openFixtureProject,
  closeAllProjects,
  type FixtureName,
} from "./project";

/** Default fixture for specs that only need "any repo with commits". */
const DEFAULT_FIXTURE: FixtureName = "simple-repo";

/** Default fixture absolute path — used by {@link fixtureRepoPath}. */
export function defaultFixturePath(): string {
  return path.join(FIXTURE_ROOT, DEFAULT_FIXTURE);
}

/** Open the default fixture repo (`simple-repo`) through the UI flow. */
export async function openAnyFixtureRepo(): Promise<void> {
  await $("aside.sidebar").waitForExist({ timeout: 10000 });
  await openFixtureProject(DEFAULT_FIXTURE);
}

/**
 * Resolve the active project's path from the running app.
 *
 * Falls back to {@link defaultFixturePath} when no project is active —
 * keeps helpers safe to call before an explicit open.
 */
export async function fixtureRepoPath(): Promise<string> {
  const result = await browser.executeAsync<string | null, []>((done) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const e2e = (window as any).__E2E__;
    if (!e2e || typeof e2e.activeProjectPath !== "function") {
      done(null);
      return;
    }
    try {
      done(e2e.activeProjectPath());
    } catch {
      done(null);
    }
  });
  return result ?? defaultFixturePath();
}

/**
 * Stage every pending change and commit via the UI.
 *
 * Navigates to the Changes view, clicks "Stage all", types the message
 * into the commit textarea, and clicks the commit button. Relies on
 * the Phase-7 `runMutation` wrapper to fire the mutation event that
 * drives the graph refresh — callers assert on `[data-testid="graph-row"]`
 * count after this returns.
 */
export async function stageAllAndCommit(message: string): Promise<void> {
  // Drop a tiny pending change so the commit has something to record.
  // Using a timestamped filename keeps the write idempotent — the
  // canonical fixture is regenerated on the next `setup.sh`, and specs
  // that run back-to-back never collide.
  const tag = Date.now();
  const repoPath = await fixtureRepoPath();
  fs.writeFileSync(
    path.join(repoPath, `e2e-touch-${tag}.txt`),
    `sentinel ${tag}\n`,
    "utf8",
  );

  // Changes view → stage all → commit.
  const nav = await $('[data-testid="nav-changes"]');
  await nav.waitForDisplayed({ timeout: 5000 });
  await nav.click();

  const stageAll = await $('[data-testid="stage-all-btn"]');
  await stageAll.waitForDisplayed({ timeout: 5000 });
  await stageAll.click();

  const msg = await $('[data-testid="commit-message"]');
  await msg.waitForDisplayed({ timeout: 5000 });
  await msg.setValue(message);

  const commit = await $('[data-testid="commit-btn"]');
  await commit.waitForEnabled({ timeout: 5000 });
  await commit.click();
}

/**
 * Create a temporary repo with a specific `origin` remote URL and
 * open it through the UI flow.
 *
 * Keeps the repo in the OS tmpdir (not `e2e/fixtures/`) so the shared
 * `setup.sh` doesn't wipe it between runs and the statusbar provider
 * spec stays side-effect-free.
 *
 * The repo contains a single commit — the statusbar filter only reads
 * `remotes.origin`, so history shape doesn't matter.
 */
export async function openFixtureRepoWithOrigin(
  originUrl: string,
): Promise<string> {
  await $("aside.sidebar").waitForExist({ timeout: 10000 });
  const repoPath = fs.mkdtempSync(path.join(os.tmpdir(), "beardgit-e2e-"));
  execSync(`git -C '${repoPath}' init -q -b main`, { stdio: "ignore" });
  execSync(`git -C '${repoPath}' config user.email test@beardgit.dev`, {
    stdio: "ignore",
  });
  execSync(`git -C '${repoPath}' config user.name Test`, { stdio: "ignore" });
  fs.writeFileSync(path.join(repoPath, "README.md"), "# e2e\n", "utf8");
  execSync(`git -C '${repoPath}' add README.md`, { stdio: "ignore" });
  execSync(`git -C '${repoPath}' commit -q -m init`, { stdio: "ignore" });
  execSync(`git -C '${repoPath}' remote add origin '${originUrl}'`, {
    stdio: "ignore",
  });

  const result = await browser.executeAsync<
    { ok: boolean; error?: string },
    [string]
  >((absPath, done) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const e2e = (window as any).__E2E__;
    if (!e2e || typeof e2e.openProject !== "function") {
      done({ ok: false, error: "window.__E2E__.openProject unavailable" });
      return;
    }
    e2e
      .openProject(absPath)
      .then(() => done({ ok: true }))
      .catch((err: unknown) =>
        done({
          ok: false,
          error: err instanceof Error ? err.message : String(err),
        }),
      );
  }, repoPath);
  if (!result.ok) {
    throw new Error(`openFixtureRepoWithOrigin failed: ${result.error}`);
  }
  await browser.pause(800);
  return repoPath;
}

/**
 * Approximate "restart the app" for cache-first paint assertions.
 *
 * Tauri-driver binds to a single app lifetime, so a real quit+reboot
 * isn't available mid-session. We simulate it by closing every open
 * project tab — the in-memory `viewportCache` survives (it lives on
 * the module) but the actual viewport store is cleared, and reopening
 * the fixture triggers the persisted-cache rehydration path that ships
 * with the Phase-8 disk snapshot.
 *
 * Not a true cold start, but it exercises the same code path: the
 * viewport is `null` when the tab opens, `restoreCachedViewport` runs
 * synchronously, and the skeleton should not appear if the snapshot
 * is warm.
 */
export async function restartApp(): Promise<void> {
  await closeAllProjects();
  await browser.pause(400);
}
