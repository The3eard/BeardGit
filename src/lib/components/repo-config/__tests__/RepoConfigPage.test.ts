/**
 * Unit tests for `RepoConfigPage.svelte`.
 *
 * Covers the Task 2.3 contract of the repo-settings-in-sidebar plan:
 *   - Renders a `not-supported` card for projects without a GitHub /
 *     GitLab provider.
 *   - Renders the section list synchronously and hydrates the store in
 *     the background.
 *   - Consumes `pendingRepoConfigSection` on mount (deep-link entry).
 *   - Opens the navigation-guard dialog when switching away from a
 *     dirty section.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  render,
  fireEvent,
  screen,
  waitFor,
  cleanup,
} from "@testing-library/svelte";
import { get } from "svelte/store";

vi.mock("$lib/api/tauri", async () => ({
  loadRemoteRepoConfig: vi.fn(),
  applyRemoteRepoConfig: vi.fn(),
  probeForgeCliStatus: vi.fn().mockResolvedValue({
    kind: "installed",
    authenticated: true,
    account: "me",
  }),
}));

import * as tauri from "$lib/api/tauri";
import RepoConfigPage from "../RepoConfigPage.svelte";
import { openTabs, activeTabIndex } from "$lib/stores/tabs";
import { providerStatus } from "$lib/stores/provider";
import { __resetForTests } from "$lib/repo-config/loader";
import {
  repoConfigStore,
  initialRepoConfigState,
} from "$lib/stores/repoConfig";
import {
  repoConfigRoute,
  pendingRepoConfigSection,
  DEFAULT_SECTION,
} from "$lib/stores/repoConfigRoute";
import type { RemoteRepoConfig } from "$lib/types/repoConfig";
import type { ProjectInfo } from "$lib/types";

/** Flip the global provider status to `kind` or `null` for the test. */
function setProvider(kind: "github" | "gitlab" | null): void {
  if (!kind) {
    providerStatus.set({ providers: [], active_index: null });
    return;
  }
  providerStatus.set({
    providers: [
      {
        kind,
        instance_url: kind === "github" ? "https://github.com" : "https://gitlab.com",
        user: {
          id: 1,
          username: "me",
          display_name: "Me",
          email: null,
          avatar_url: null,
          profile_url: "",
        },
        project_name: null,
      },
    ],
    active_index: 0,
  });
}

function mockConfig(): RemoteRepoConfig {
  return {
    description: "orig",
    homepage: null,
    topics: [],
    visibility: "public",
    default_branch: "main",
    issues_enabled: true,
    wiki_enabled: false,
    archived: false,
    branch_protection: null,
    labels: [],
  };
}

function setActiveProject(project: ProjectInfo | null): void {
  if (!project) {
    openTabs.set([]);
    activeTabIndex.set(-1);
    return;
  }
  openTabs.set([{ kind: "project", project }]);
  activeTabIndex.set(0);
}

function makeProject(over: Partial<ProjectInfo> = {}): ProjectInfo {
  return {
    name: "x",
    path: "/x",
    head_branch: "main",
    change_count: 0,
    is_worktree: false,
    ...over,
  };
}

afterEach(() => cleanup());

beforeEach(() => {
  __resetForTests();
  repoConfigStore.set(initialRepoConfigState());
  repoConfigRoute.set({ section: DEFAULT_SECTION });
  pendingRepoConfigSection.set(null);
  setActiveProject(null);
  setProvider(null);
  (tauri.loadRemoteRepoConfig as unknown as { mockReset: () => void }).mockReset();
  (tauri.applyRemoteRepoConfig as unknown as { mockReset: () => void }).mockReset();
});

describe("RepoConfigPage", () => {
  it("renders not-supported card when no provider is connected", async () => {
    setActiveProject(makeProject());
    setProvider(null);
    render(RepoConfigPage);
    expect(await screen.findByText(/no repo settings available/i)).toBeTruthy();
  });

  it("renders the section list immediately and loads config in the background", async () => {
    setActiveProject(makeProject());
    setProvider("github");
    (
      tauri.loadRemoteRepoConfig as unknown as {
        mockResolvedValueOnce: (v: unknown) => void;
      }
    ).mockResolvedValueOnce(mockConfig());

    render(RepoConfigPage);
    expect(screen.getByTestId("bg-cat-nav-general")).toBeTruthy();

    await waitFor(() => {
      expect(get(repoConfigStore).current).not.toBeNull();
    });
  });

  it("consumes pendingRepoConfigSection on mount", async () => {
    setActiveProject(makeProject());
    setProvider("github");
    (
      tauri.loadRemoteRepoConfig as unknown as {
        mockResolvedValueOnce: (v: unknown) => void;
      }
    ).mockResolvedValueOnce(mockConfig());
    pendingRepoConfigSection.set("labels");

    render(RepoConfigPage);
    await waitFor(() => {
      expect(get(repoConfigRoute).section).toBe("labels");
      expect(get(pendingRepoConfigSection)).toBeNull();
    });
  });

  it("opens the guard dialog when switching away from a dirty section", async () => {
    setActiveProject(makeProject());
    setProvider("github");
    (
      tauri.loadRemoteRepoConfig as unknown as {
        mockResolvedValue: (v: unknown) => void;
      }
    ).mockResolvedValue(mockConfig());
    render(RepoConfigPage);
    await waitFor(() => {
      expect(get(repoConfigStore).current).not.toBeNull();
    });

    // Dirty the General slice by mutating current.description.
    repoConfigStore.update((s) =>
      s.current
        ? { ...s, current: { ...s.current, description: "edited" } }
        : s,
    );

    // Attempt to switch to Visibility.
    await fireEvent.click(screen.getByTestId("bg-cat-nav-visibility"));

    expect(await screen.findByTestId("repo-config-guard-body")).toBeTruthy();
  });
});
