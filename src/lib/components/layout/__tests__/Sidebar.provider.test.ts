import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import Sidebar from "../Sidebar.svelte";
import { providerStatus } from "$lib/stores/provider";
import { sidebarLayout } from "$lib/stores/sidebarLayout";

afterEach(() => cleanup());

beforeEach(() => {
  providerStatus.set({ providers: [], active_index: null });
  sidebarLayout.set({ order: [], hidden: [] });
});

describe("Sidebar — provider section visibility", () => {
  it("hides the entire provider section when hasActiveProvider is false", () => {
    const { queryByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    expect(queryByTestId("nav-pipelines")).toBeNull();
    expect(queryByTestId("nav-issues")).toBeNull();
    expect(queryByTestId("nav-merge-requests")).toBeNull();
    expect(queryByTestId("nav-releases")).toBeNull();
    expect(queryByTestId("nav-repo-config")).toBeNull();
  });

  it("shows the provider section when a GitHub provider is active", () => {
    providerStatus.set({
      providers: [
        {
          kind: "github",
          instance_url: "https://api.github.com",
          account: "me",
        } as unknown as import("$lib/types").ConnectedProvider,
      ],
      active_index: 0,
    });
    const { getByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    expect(getByTestId("nav-pipelines")).toBeTruthy();
    expect(getByTestId("nav-issues")).toBeTruthy();
    expect(getByTestId("nav-merge-requests")).toBeTruthy();
    expect(getByTestId("nav-releases")).toBeTruthy();
    expect(getByTestId("nav-repo-config")).toBeTruthy();
  });
});
