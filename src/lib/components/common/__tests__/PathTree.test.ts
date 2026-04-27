/**
 * PathTree: flat list below threshold, collapsible tree above.
 */
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import PathTree from "$lib/components/common/PathTree.svelte";

afterEach(() => cleanup());

const mkItems = (paths: string[]) => paths.map((p) => ({ path: p, meta: { additions: 1, deletions: 0 } }));

describe("PathTree", () => {
  it("renders a flat list below the threshold", () => {
    const { container } = render(PathTree, {
      items: mkItems(["a.ts", "b.ts"]),
      autoFlattenThreshold: 20,
      selectedPath: null,
    });
    expect(container.querySelectorAll("[data-pathtree-leaf]").length).toBe(2);
    expect(container.querySelector("[data-pathtree-folder]")).toBeNull();
  });

  it("renders a folder tree above the threshold", () => {
    const paths = Array.from({ length: 25 }, (_, i) => `src/dir${i % 3}/f${i}.ts`);
    const { container } = render(PathTree, {
      items: mkItems(paths),
      autoFlattenThreshold: 20,
      selectedPath: null,
    });
    expect(container.querySelectorAll("[data-pathtree-folder]").length).toBeGreaterThan(0);
  });

  it("fires onSelect with the full path when a leaf is clicked", async () => {
    const onSelect = vi.fn();
    const { getByRole } = render(PathTree, {
      items: mkItems(["a.ts"]),
      autoFlattenThreshold: 20,
      selectedPath: null,
      onSelect,
    });
    await fireEvent.click(getByRole("button", { name: /a\.ts/ }));
    expect(onSelect).toHaveBeenCalledWith("a.ts");
  });
});
