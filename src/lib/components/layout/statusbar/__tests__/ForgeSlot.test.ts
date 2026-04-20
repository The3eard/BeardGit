/**
 * Unit tests for `ForgeSlot.svelte` — renders one pill per authenticated
 * forge provider read from `providerStatus`. Hidden when no provider.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { ProviderStatusResponse } from "$lib/types";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  type Status = {
    providers: Array<{
      kind: "github" | "gitlab";
      instance_url: string;
      user: {
        id: number;
        username: string;
        display_name: string;
        email: string | null;
        avatar_url: string | null;
        profile_url: string;
      };
      project_name: string | null;
    }>;
    active_index: number | null;
  };
  return {
    providerStatus: writable<Status>({ providers: [], active_index: null }),
  };
});

vi.mock("$lib/stores/provider", () => ({
  providerStatus: mocks.providerStatus,
}));

import ForgeSlot from "../ForgeSlot.svelte";

beforeEach(() => {
  mocks.providerStatus.set({
    providers: [],
    active_index: null,
  } as ProviderStatusResponse);
});

afterEach(() => cleanup());

describe("ForgeSlot", () => {
  it("renders nothing when no providers are configured", async () => {
    const { queryByTestId } = render(ForgeSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    expect(queryByTestId("statusbar-forge-slot")).toBeNull();
  });

  it("renders a single pill for one authed provider", async () => {
    mocks.providerStatus.set({
      providers: [
        {
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
        },
      ],
      active_index: 0,
    } as ProviderStatusResponse);
    const { getAllByTestId } = render(ForgeSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    const pills = getAllByTestId("statusbar-forge-pill");
    expect(pills.length).toBe(1);
    expect(pills[0].getAttribute("data-kind")).toBe("github");
    expect(pills[0].getAttribute("data-state")).toBe("authed");
  });

  it("renders two pills when both providers are configured", async () => {
    mocks.providerStatus.set({
      providers: [
        {
          kind: "github",
          instance_url: "https://github.com",
          user: {
            id: 1,
            username: "octocat",
            display_name: "Octocat",
            email: null,
            avatar_url: null,
            profile_url: "",
          },
          project_name: null,
        },
        {
          kind: "gitlab",
          instance_url: "https://gitlab.com",
          user: {
            id: 2,
            username: "gitlabber",
            display_name: "Gitlabber",
            email: null,
            avatar_url: null,
            profile_url: "",
          },
          project_name: null,
        },
      ],
      active_index: 0,
    } as ProviderStatusResponse);
    const { getAllByTestId } = render(ForgeSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    const pills = getAllByTestId("statusbar-forge-pill");
    expect(pills.length).toBe(2);
    expect(pills.map((p) => p.getAttribute("data-kind"))).toEqual([
      "github",
      "gitlab",
    ]);
  });

  it("fires onNavigate('integrations') when a pill is clicked", async () => {
    mocks.providerStatus.set({
      providers: [
        {
          kind: "gitlab",
          instance_url: "https://gitlab.example",
          user: {
            id: 9,
            username: "u",
            display_name: "U",
            email: null,
            avatar_url: null,
            profile_url: "",
          },
          project_name: null,
        },
      ],
      active_index: 0,
    } as ProviderStatusResponse);
    const onNavigate = vi.fn();
    const { getByTestId } = render(ForgeSlot, { props: { onNavigate } });
    await tick();
    await fireEvent.click(getByTestId("statusbar-forge-pill"));
    expect(onNavigate).toHaveBeenCalledWith("integrations");
  });
});
