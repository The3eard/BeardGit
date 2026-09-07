import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { activeViewStore } from "./navigation";
import {
  installProviderDisconnectReroute,
  installAiDisabledReroute,
  PROVIDER_VIEWS,
  AI_VIEWS,
} from "./navigation";
import { providerStatus } from "./provider";
import { aiEnabled } from "./ai";

beforeEach(() => {
  providerStatus.set({ providers: [], active_index: null });
  aiEnabled.set(true);
  activeViewStore.set("graph");
});

describe("AI disabled reroute", () => {
  it("routes AI views back to graph when the switch goes off", () => {
    activeViewStore.set("ai-sessions");
    const teardown = installAiDisabledReroute();
    expect(get(activeViewStore)).toBe("ai-sessions");

    aiEnabled.set(false);
    expect(get(activeViewStore)).toBe("graph");
    teardown();
  });

  it("leaves other views alone and does nothing when re-enabled", () => {
    activeViewStore.set("changes");
    const teardown = installAiDisabledReroute();
    aiEnabled.set(false);
    expect(get(activeViewStore)).toBe("changes");
    aiEnabled.set(true);
    expect(get(activeViewStore)).toBe("changes");
    teardown();
  });

  it("exports the canonical AI view list", () => {
    expect(AI_VIEWS).toEqual(["ai-config", "ai-sessions"]);
  });
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
