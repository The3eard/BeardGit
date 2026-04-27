import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("$lib/api/tauri", () => ({
  openProject: vi.fn(),
  getOpenProjects: vi.fn().mockResolvedValue([]),
  closeProject: vi.fn(),
  switchProject: vi.fn(),
  getActiveProjectIndex: vi.fn().mockResolvedValue(null),
  restoreProjects: vi.fn().mockResolvedValue([]),
  getBranches: vi.fn().mockResolvedValue([]),
  getStatusSummary: vi.fn(),
  detectProject: vi.fn(),
}));

vi.mock("$lib/stores/initRepoDialog", () => ({
  requestOpenInitRepoDialog: vi.fn(),
  closeInitRepoDialog: vi.fn(),
}));

import { openProject } from "$lib/api/tauri";
import { requestOpenInitRepoDialog } from "$lib/stores/initRepoDialog";
import { openProjectTab } from "../projects";

describe("openProjectTab not-a-repo handling", () => {
  beforeEach(() => vi.clearAllMocks());

  it("opens the init dialog when backend rejects with not_a_repo", async () => {
    (openProject as unknown as ReturnType<typeof vi.fn>).mockRejectedValueOnce({
      kind: "not_a_repo",
      path: "/tmp/foo",
    });
    await openProjectTab("/tmp/foo");
    expect(requestOpenInitRepoDialog).toHaveBeenCalledWith("/tmp/foo");
  });

  it("rethrows on other errors", async () => {
    (openProject as unknown as ReturnType<typeof vi.fn>).mockRejectedValueOnce({
      kind: "other",
      message: "boom",
    });
    await expect(openProjectTab("/tmp/foo")).rejects.toBeDefined();
    expect(requestOpenInitRepoDialog).not.toHaveBeenCalled();
  });
});
