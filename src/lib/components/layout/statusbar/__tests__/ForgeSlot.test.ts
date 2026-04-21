/**
 * Unit tests for `ForgeSlot.svelte`.
 *
 * ForgeSlot consumes the `projectProvider` derived store and renders
 * exactly one pill (GitHub or GitLab) for the active project — or
 * nothing when no provider is resolved. It is NEVER allowed to render
 * both pills simultaneously (pre-Phase-9 behaviour).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { ConnectedProvider } from "$lib/types";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  type Resolved =
    | { kind: "github" | "gitlab"; provider: ConnectedProvider }
    | null;
  return {
    projectProvider: writable<Resolved>(null),
  };
});

vi.mock("$lib/stores/provider", () => ({
  projectProvider: mocks.projectProvider,
}));

import ForgeSlot from "../ForgeSlot.svelte";

const githubProvider: ConnectedProvider = {
  kind: "github",
  instance_url: "https://github.com",
  user: {
    id: 1,
    username: "octocat",
    display_name: "Octocat",
    email: null,
    avatar_url: null,
    profile_url: "https://github.com/octocat",
  },
  project_name: null,
};

const gitlabProvider: ConnectedProvider = {
  kind: "gitlab",
  instance_url: "https://gitlab.com",
  user: {
    id: 2,
    username: "tanuki",
    display_name: "Tanuki",
    email: null,
    avatar_url: null,
    profile_url: "https://gitlab.com/tanuki",
  },
  project_name: null,
};

beforeEach(() => {
  mocks.projectProvider.set(null);
});

afterEach(() => cleanup());

describe("ForgeSlot", () => {
  it("renders nothing when projectProvider is null", async () => {
    const { queryByTestId } = render(ForgeSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    expect(queryByTestId("statusbar-forge-slot")).toBeNull();
    expect(queryByTestId("statusbar-forge-pill")).toBeNull();
  });

  it("renders a single GitHub pill when projectProvider resolves to github", async () => {
    mocks.projectProvider.set({ kind: "github", provider: githubProvider });
    const { getAllByTestId } = render(ForgeSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    const pills = getAllByTestId("statusbar-forge-pill");
    expect(pills.length).toBe(1);
    expect(pills[0].getAttribute("data-kind")).toBe("github");
    expect(pills[0].getAttribute("data-state")).toBe("authed");
  });

  it("renders a single GitLab pill when projectProvider resolves to gitlab", async () => {
    mocks.projectProvider.set({ kind: "gitlab", provider: gitlabProvider });
    const { getAllByTestId } = render(ForgeSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    const pills = getAllByTestId("statusbar-forge-pill");
    expect(pills.length).toBe(1);
    expect(pills[0].getAttribute("data-kind")).toBe("gitlab");
  });

  it("never renders both pills simultaneously", async () => {
    // Switch the store between both providers in sequence — the second
    // render must REPLACE the first, not accumulate.
    mocks.projectProvider.set({ kind: "github", provider: githubProvider });
    const { getAllByTestId } = render(ForgeSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    expect(getAllByTestId("statusbar-forge-pill").length).toBe(1);

    mocks.projectProvider.set({ kind: "gitlab", provider: gitlabProvider });
    await tick();
    const pills = getAllByTestId("statusbar-forge-pill");
    expect(pills.length).toBe(1);
    expect(pills[0].getAttribute("data-kind")).toBe("gitlab");
  });

  it("fires onNavigate('integrations') when the pill is clicked", async () => {
    mocks.projectProvider.set({ kind: "gitlab", provider: gitlabProvider });
    const onNavigate = vi.fn();
    const { getByTestId } = render(ForgeSlot, { props: { onNavigate } });
    await tick();
    await fireEvent.click(getByTestId("statusbar-forge-pill"));
    expect(onNavigate).toHaveBeenCalledWith("integrations");
  });
});
