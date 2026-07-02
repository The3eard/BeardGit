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
    // open_project now rejects with an IpcError; not_a_repo carries the
    // attempted path in `message`.
    (openProject as unknown as ReturnType<typeof vi.fn>).mockRejectedValueOnce({
      code: "not_a_repo",
      message: "/tmp/foo",
    });
    await openProjectTab("/tmp/foo");
    expect(requestOpenInitRepoDialog).toHaveBeenCalledWith("/tmp/foo");
  });

  it("rethrows on other errors", async () => {
    (openProject as unknown as ReturnType<typeof vi.fn>).mockRejectedValueOnce({
      code: "open_failed",
      message: "boom",
    });
    await expect(openProjectTab("/tmp/foo")).rejects.toBeDefined();
    expect(requestOpenInitRepoDialog).not.toHaveBeenCalled();
  });
});
