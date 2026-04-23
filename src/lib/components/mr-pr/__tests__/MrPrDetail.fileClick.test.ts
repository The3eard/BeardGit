/**
 * Clicking a file row fires the `onFileClick` callback with the exact
 * path and applies the `.selected` class to the clicked row.
 */
import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import MrPrDetail from "$lib/components/mr-pr/MrPrDetail.svelte";
import { mrPrDetail, mrPrDiffFiles, selectedPrFilePath } from "$lib/stores/mr-pr";

// Stub Tauri plugins used by MrPrDetail
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

describe("MrPrDetail file rows", () => {
  it("fires onFileClick with the clicked path", async () => {
    mrPrDetail.set({
      summary: {
        number: 1, title: "x", state: "open", author: "a",
        source_branch: "s", target_branch: "t", url: "u", draft: false,
        labels: [], reviewers: [], created_at: "", updated_at: "",
        additions: null, deletions: null, changed_files: null,
        base_sha: "b", head_sha: "h", head_repo_url: null,
      },
      body: "", comments: [], review_status: "pending", mergeable: null,
    });
    mrPrDiffFiles.set([
      { path: "a.ts", old_path: null, status: "modified", additions: 1, deletions: 0, patch: null },
    ]);
    const onFileClick = vi.fn();
    const { getByRole } = render(MrPrDetail, { onFileClick });
    await fireEvent.click(getByRole("button", { name: /a\.ts/ }));
    expect(onFileClick).toHaveBeenCalledWith("a.ts");
  });

  it("highlights the row whose path equals selectedPrFilePath", () => {
    selectedPrFilePath.set("b.ts");
    mrPrDiffFiles.set([
      { path: "a.ts", old_path: null, status: "modified", additions: 1, deletions: 0, patch: null },
      { path: "b.ts", old_path: null, status: "modified", additions: 1, deletions: 0, patch: null },
    ]);
    const { getAllByRole } = render(MrPrDetail, { onFileClick: () => {} });
    const rows = getAllByRole("button").filter((b) => b.className.includes("file-row"));
    expect(rows.find((r) => r.textContent?.includes("b.ts"))!.className).toMatch(/selected/);
  });
});
