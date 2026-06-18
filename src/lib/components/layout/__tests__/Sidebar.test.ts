import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import Sidebar from "../Sidebar.svelte";
import { providerStatus } from "$lib/stores/provider";

afterEach(() => cleanup());

beforeEach(async () => {
  providerStatus.set({ providers: [], active_index: null });
  const { sidebarLayout } = await import("$lib/stores/sidebarLayout");
  sidebarLayout.set({ order: [], hidden: [] });
});

describe("Sidebar — repo-config entry", () => {
  it("is hidden when no provider is active", () => {
    providerStatus.set({ providers: [], active_index: null });
    const { queryByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    expect(queryByTestId("nav-repo-config")).toBeNull();
  });

  it("renders when the active provider is github", () => {
    providerStatus.set({
      providers: [
        {
          kind: "github",
          instance_url: "https://github.com",
          account: "me",
        } as unknown as import("$lib/types").ConnectedProvider,
      ],
      active_index: 0,
    });
    const { getByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    expect(getByTestId("nav-repo-config")).toBeTruthy();
  });

  it("renders when the active provider is gitlab", () => {
    providerStatus.set({
      providers: [
        {
          kind: "gitlab",
          instance_url: "https://gitlab.com",
          account: "me",
        } as unknown as import("$lib/types").ConnectedProvider,
      ],
      active_index: 0,
    });
    const { getByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    expect(getByTestId("nav-repo-config")).toBeTruthy();
  });
});

describe("Sidebar — nav layout integration", () => {
  it("hides items listed in sidebarLayout.hidden in normal mode", async () => {
    const { sidebarLayout } = await import("$lib/stores/sidebarLayout");
    sidebarLayout.set({ order: [], hidden: ["bisect"] });
    const { queryByTestId } = render(Sidebar, { props: { activeView: "graph" } });
    expect(queryByTestId("nav-bisect")).toBeNull();
    expect(queryByTestId("nav-graph")).toBeTruthy();
  });

  it("renders items in fixed task-group order (workspace before history)", async () => {
    const { sidebarLayout } = await import("$lib/stores/sidebarLayout");
    // The saved order is intentionally ignored now — groups are fixed.
    sidebarLayout.set({ order: ["ai-sessions", "graph"], hidden: [] });
    const { container } = render(Sidebar, { props: { activeView: "graph" } });
    const items = container.querySelectorAll("[data-testid^='nav-']");
    const testIds = Array.from(items).map((el) => el.getAttribute("data-testid"));
    const graphIdx = testIds.indexOf("nav-graph"); // Workspace group
    const branchesIdx = testIds.indexOf("nav-branches"); // History group
    expect(graphIdx).toBeGreaterThan(-1);
    expect(branchesIdx).toBeGreaterThan(-1);
    expect(graphIdx).toBeLessThan(branchesIdx);
  });

  it("drops a group header when all its items are hidden", async () => {
    const { sidebarLayout } = await import("$lib/stores/sidebarLayout");
    // Hide the whole Advanced group (worktrees/submodules/bisect).
    sidebarLayout.set({ order: [], hidden: ["worktrees", "submodules", "bisect"] });
    const { queryByText } = render(Sidebar, { props: { activeView: "graph" } });
    expect(queryByText("Advanced")).toBeNull();
    expect(queryByText("Workspace")).toBeTruthy();
  });
});
