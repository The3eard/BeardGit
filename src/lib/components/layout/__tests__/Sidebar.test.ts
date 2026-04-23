import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import Sidebar from "../Sidebar.svelte";
import { providerStatus } from "$lib/stores/provider";

afterEach(() => cleanup());

beforeEach(() => {
  providerStatus.set({ providers: [], active_index: null });
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
