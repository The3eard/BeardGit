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

  it("respects sidebarLayout.order in normal mode", async () => {
    const { sidebarLayout } = await import("$lib/stores/sidebarLayout");
    sidebarLayout.set({
      order: ["ai-sessions", "graph"],
      hidden: [],
    });
    const { container } = render(Sidebar, { props: { activeView: "graph" } });
    const items = container.querySelectorAll("[data-testid^='nav-']");
    // The Navigation section is the first nav; find the first two
    // buttons and confirm the order.
    const testIds = Array.from(items).map((el) => el.getAttribute("data-testid"));
    const aiIdx = testIds.indexOf("nav-ai-sessions");
    const graphIdx = testIds.indexOf("nav-graph");
    expect(aiIdx).toBeGreaterThan(-1);
    expect(graphIdx).toBeGreaterThan(-1);
    expect(aiIdx).toBeLessThan(graphIdx);
  });
});
