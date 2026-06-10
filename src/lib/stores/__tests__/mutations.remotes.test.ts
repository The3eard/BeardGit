/**
 * Regression test — `dispatchRefresh` must call `refreshRemotes`
 * whenever the incoming flags carry `remotes_changed`.
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("../graph", () => ({ refreshAndReloadGraph: vi.fn() }));
vi.mock("../changes", () => ({ refreshStatuses: vi.fn(), refreshDiffs: vi.fn() }));
vi.mock("../stashes", () => ({ refreshStashes: vi.fn() }));
vi.mock("../worktrees", () => ({ refreshWorktrees: vi.fn() }));
vi.mock("../repoConfig", () => ({ refreshRepoConfig: vi.fn(), repoConfig: { subscribe: vi.fn() } }));
vi.mock("../remotes", () => ({ refreshRemotes: vi.fn() }));

import { dispatchRefresh } from "../mutations";
import { refreshRemotes } from "../remotes";

describe("dispatchRefresh — remotes", () => {
  it("calls refreshRemotes when remotes_changed is true", () => {
    dispatchRefresh({
      refs_changed: false,
      head_changed: false,
      status_changed: false,
      stashes_changed: false,
      worktrees_changed: false,
      remotes_changed: true,
    });
    expect(refreshRemotes).toHaveBeenCalled();
  });

  it("does NOT call refreshRemotes when remotes_changed is false", () => {
    (refreshRemotes as ReturnType<typeof vi.fn>).mockReset();
    dispatchRefresh({
      refs_changed: true,
      head_changed: true,
      status_changed: true,
      stashes_changed: false,
      worktrees_changed: false,
      remotes_changed: false,
    });
    expect(refreshRemotes).not.toHaveBeenCalled();
  });
});
