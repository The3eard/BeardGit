import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { activeViewStore } from "./navigation";
import {
  installProviderDisconnectReroute,
  PROVIDER_VIEWS,
} from "./navigation";
import { providerStatus } from "./provider";

beforeEach(() => {
  providerStatus.set({ providers: [], active_index: null });
  activeViewStore.set("graph");
});

describe("provider disconnect reroute", () => {
  it("routes provider-scoped views back to graph when the provider disconnects", () => {
    // Start connected on the pipelines view.
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
    activeViewStore.set("pipelines");

    const teardown = installProviderDisconnectReroute();
    expect(get(activeViewStore)).toBe("pipelines");

    providerStatus.set({ providers: [], active_index: null });
    expect(get(activeViewStore)).toBe("graph");

    teardown();
  });

  it("leaves non-provider views alone on disconnect", () => {
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
    activeViewStore.set("branches");

    const teardown = installProviderDisconnectReroute();
    providerStatus.set({ providers: [], active_index: null });
    expect(get(activeViewStore)).toBe("branches");
    teardown();
  });

  it("exports the canonical provider view list", () => {
    expect(PROVIDER_VIEWS).toEqual([
      "pipelines",
      "issues",
      "merge-requests",
      "releases",
      "repo-config",
    ]);
  });
});
