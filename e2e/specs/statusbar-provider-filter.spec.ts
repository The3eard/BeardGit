/**
 * Phase 11.5 — Statusbar provider filter.
 *
 * The `ForgeSlot` in the statusbar must render exactly *one* pill
 * reflecting the active project's forge, or nothing when no forge
 * can be inferred. The decision is driven by the `projectProvider`
 * derived store (repo-config override → origin-URL heuristic → null).
 *
 * Test matrix (plan Phase 11.5):
 *  - A repo whose `origin` is `git@github.com:…` → one `github` pill.
 *  - A repo whose `origin` is `git@gitlab.com:…` → one `gitlab` pill.
 *  - (Implicit from both assertions: never both pills at once.)
 *
 * Each test provisions its own throwaway repo in `$TMPDIR` via
 * `openFixtureRepoWithOrigin` so the canonical fixtures aren't
 * mutated.
 */
import { expect } from "@wdio/globals";
import { openFixtureRepoWithOrigin } from "../helpers/fixtures";

describe("statusbar provider filter", () => {
  it("shows only the github pill for a github-origin repo", async () => {
    await openFixtureRepoWithOrigin("git@github.com:test/demo.git");
    // Give the repo-config + provider derivations a tick to settle.
    const firstPill = await $('[data-testid="statusbar-forge-pill"]');
    await firstPill.waitForDisplayed({ timeout: 5000 });

    const pillCount = await $$('[data-testid="statusbar-forge-pill"]').length;
    expect(pillCount).toBe(1);
    const kind = await $('[data-testid="statusbar-forge-pill"]').getAttribute(
      "data-kind",
    );
    expect(kind).toBe("github");
  });

  it("shows only the gitlab pill for a gitlab-origin repo", async () => {
    await openFixtureRepoWithOrigin("git@gitlab.com:test/demo.git");
    const firstPill = await $('[data-testid="statusbar-forge-pill"]');
    await firstPill.waitForDisplayed({ timeout: 5000 });

    const pillCount = await $$('[data-testid="statusbar-forge-pill"]').length;
    expect(pillCount).toBe(1);
    const kind = await $('[data-testid="statusbar-forge-pill"]').getAttribute(
      "data-kind",
    );
    expect(kind).toBe("gitlab");
  });
});
