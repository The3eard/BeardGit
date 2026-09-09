/**
 * SubmoduleList: what the rows show, and which operation a row's menu runs.
 *
 * The `parent` argument is the reason this file exists. Every write operation
 * on a nested submodule has to run in the superproject that registers it, and
 * the only thing between the row and the right repository is the component
 * passing `sub.parent` through. Nothing else would notice if it stopped: the
 * command would simply fail against the root.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import type { SubmoduleInfo } from "$lib/types";

// Spies only: `vi.hoisted` runs before the imports, so the stores themselves
// are created inside the mock factory below.
const spies = vi.hoisted(() => ({
  refreshSubmodules: vi.fn(),
  initSubmodule: vi.fn(),
  updateSubmodule: vi.fn(),
  updateAllSubmodules: vi.fn(),
  deinitSubmodule: vi.fn(),
  addSubmodule: vi.fn(),
  removeSubmodule: vi.fn(),
  getSubmoduleAbsPath: vi.fn(),
  openProjectTab: vi.fn(),
}));

vi.mock("../../../stores/submodules", async () => {
  const { writable: w } = await import("svelte/store");
  return {
    submodules: w<SubmoduleInfo[]>([]),
    submodulesLoading: w(false),
    ...spies,
  };
});

vi.mock("../../../stores/projects", () => ({
  openProjectTab: spies.openProjectTab,
}));

import SubmoduleList from "../SubmoduleList.svelte";
import { submodules, submodulesLoading } from "../../../stores/submodules";

function submodule(over: Partial<SubmoduleInfo> = {}): SubmoduleInfo {
  return {
    name: "sub",
    path: "libs/sub",
    url: "https://example.com/sub.git",
    oid: "aaaaaaaaaaaa",
    registered_oid: "aaaaaaaaaaaa",
    status: "clean",
    depth: 0,
    parent: null,
    ...over,
  };
}

const NESTED = submodule({
  name: "inner",
  path: "libs/sub/inner",
  url: "https://example.com/inner.git",
  status: "uninitialized",
  oid: null,
  depth: 1,
  parent: "libs/sub",
});

/** Row element for `path`, as the List renders it. */
function row(container: HTMLElement, path: string): HTMLElement {
  const testid = `submodule-row-${path.replace(/\//g, "-")}`;
  const inner = container.querySelector(`[data-testid="${testid}"]`);
  if (!inner) throw new Error(`no row for ${path}`);
  return inner as HTMLElement;
}

/** Open a row's context menu and return the labels it rendered. */
async function openMenu(container: HTMLElement, path: string): Promise<string[]> {
  await fireEvent.contextMenu(row(container, path).closest(".list-row")!);
  return [...container.querySelectorAll("*")]
    .filter((el) => el.children.length === 0)
    .map((el) => el.textContent?.trim() ?? "")
    .filter(Boolean);
}

/** Click a context-menu entry by its exact label. */
async function clickMenuItem(container: HTMLElement, label: string) {
  const hit = [...container.querySelectorAll("*")].find(
    (el) => el.children.length === 0 && el.textContent?.trim() === label,
  );
  if (!hit) throw new Error(`no menu item labelled "${label}"`);
  await fireEvent.click(hit);
}

beforeEach(() => {
  vi.clearAllMocks();
  submodules.set([]);
  submodulesLoading.set(false);
});

afterEach(() => cleanup());

describe("rows", () => {
  it("renders one row per submodule with its status", () => {
    submodules.set([submodule(), NESTED]);

    const { container, getByText } = render(SubmoduleList);

    expect(row(container, "libs/sub")).toBeTruthy();
    expect(row(container, "libs/sub/inner")).toBeTruthy();
    expect(getByText("CLEAN")).toBeTruthy();
    expect(getByText("UNINIT")).toBeTruthy();
  });

  it("indents a nested submodule and drops its parent's prefix", () => {
    submodules.set([submodule(), NESTED]);

    const { container } = render(SubmoduleList);

    expect(row(container, "libs/sub").style.paddingLeft).toBe("0px");
    const nested = row(container, "libs/sub/inner");
    expect(nested.style.paddingLeft).toBe("16px");
    // The indent, plus the parent sitting right above it, says whose it is;
    // the row shows the local path and keeps the full one in `title`.
    expect(nested.querySelector(".sub-path")?.textContent).toBe("inner");
    expect(nested.querySelector(".sub-path")?.getAttribute("title")).toBe("libs/sub/inner");
  });

  it("offers Update All only when the repository has submodules", () => {
    const empty = render(SubmoduleList);
    expect(empty.queryByText("Update All")).toBeNull();
    cleanup();

    submodules.set([submodule()]);
    expect(render(SubmoduleList).getByText("Update All")).toBeTruthy();
  });
});

describe("context menu", () => {
  it("offers Update but not Initialize for a checked-out submodule", async () => {
    submodules.set([submodule()]);
    const { container } = render(SubmoduleList);

    const labels = await openMenu(container, "libs/sub");

    expect(labels).toContain("Update");
    expect(labels).toContain("Open in Tab");
    expect(labels).not.toContain("Initialize");
  });

  it("offers Initialize but not Update for an uninitialized submodule", async () => {
    submodules.set([submodule({ status: "uninitialized", oid: null })]);
    const { container } = render(SubmoduleList);

    const labels = await openMenu(container, "libs/sub");

    expect(labels).toContain("Initialize");
    expect(labels).not.toContain("Update");
    expect(labels).not.toContain("Open in Tab");
  });
});

describe("nested submodules carry their parent", () => {
  it("updates a nested submodule inside its parent superproject", async () => {
    submodules.set([submodule(), { ...NESTED, status: "outdated", oid: "bbbbbbbbbbbb" }]);
    const { container } = render(SubmoduleList);

    await openMenu(container, "libs/sub/inner");
    await clickMenuItem(container, "Update");

    expect(spies.updateSubmodule).toHaveBeenCalledWith("libs/sub/inner", "libs/sub");
  });

  it("initializes a nested submodule inside its parent superproject", async () => {
    submodules.set([submodule(), NESTED]);
    const { container } = render(SubmoduleList);

    await openMenu(container, "libs/sub/inner");
    await clickMenuItem(container, "Initialize");

    expect(spies.initSubmodule).toHaveBeenCalledWith("libs/sub/inner", "libs/sub");
  });

  it("passes no parent for a top-level submodule", async () => {
    submodules.set([submodule()]);
    const { container } = render(SubmoduleList);

    await openMenu(container, "libs/sub");
    await clickMenuItem(container, "Update");

    expect(spies.updateSubmodule).toHaveBeenCalledWith("libs/sub", null);
  });
});
