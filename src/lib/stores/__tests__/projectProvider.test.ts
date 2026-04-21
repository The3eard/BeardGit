/**
 * Unit tests for `projectProvider` — the single-provider derived store
 * that feeds the statusbar forge slot.
 *
 * Selection rules (applied in order):
 *   1. If `repoConfig.provider === "github"` → GitHub (if connected).
 *   2. Else if `repoConfig.provider === "gitlab"` → GitLab (if connected).
 *   3. Else inspect `activeProject.remotes[origin]`:
 *        - `github.com` in URL → GitHub
 *        - `gitlab.com` or `gitlab.<domain>` → GitLab
 *   4. Otherwise → `null`.
 *
 * The matrix below covers the full
 * `(repoConfig: none/github/gitlab) × (remote: none/github/gitlab)` grid
 * so every rule path is exercised at least once.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";

import {
  projectProvider,
  providerStatus,
} from "../provider";
import { repoConfig } from "../repoConfig";
import { openTabs, activeTabIndex } from "../tabs";
import type { ProjectInfo, ProviderStatusResponse } from "../../types";

/**
 * Drive the `activeProject` derived store by seeding the underlying
 * tabs state — `activeProject` is itself a derived(openTabs,
 * activeTabIndex) chain so writing to the writables produces a project
 * tab in the active slot.
 */
function setActiveProject(project: (ProjectInfo & { remotes?: Array<{ name: string; url: string }> }) | null): void {
  if (!project) {
    openTabs.set([]);
    activeTabIndex.set(-1);
    return;
  }
  openTabs.set([{ kind: "project", project: project as ProjectInfo }]);
  activeTabIndex.set(0);
}

const githubProvider = {
  kind: "github" as const,
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

const gitlabProvider = {
  kind: "gitlab" as const,
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

const bothProviders: ProviderStatusResponse = {
  providers: [githubProvider, gitlabProvider],
  active_index: 0,
};

const githubRemote = {
  path: "/r",
  name: "r",
  head_branch: null,
  change_count: 0,
  remotes: [{ name: "origin", url: "git@github.com:x/y.git" }],
};

const gitlabRemote = {
  path: "/r",
  name: "r",
  head_branch: null,
  change_count: 0,
  remotes: [{ name: "origin", url: "https://gitlab.com/x/y.git" }],
};

const noRemote = {
  path: "/r",
  name: "r",
  head_branch: null,
  change_count: 0,
  remotes: [],
};

beforeEach(() => {
  providerStatus.set(bothProviders);
  repoConfig.set(null);
  setActiveProject(null);
});

describe("projectProvider", () => {
  it("null when no active project", () => {
    expect(get(projectProvider)).toBeNull();
  });

  // --- Matrix: (repoConfig × remote) -------------------------------------

  it("[none × none] → null when neither repoConfig nor remote hint exists", () => {
    setActiveProject(noRemote as never);
    expect(get(projectProvider)).toBeNull();
  });

  it("[none × github-remote] → github via URL heuristic", () => {
    setActiveProject(githubRemote as never);
    expect(get(projectProvider)?.kind).toBe("github");
  });

  it("[none × gitlab-remote] → gitlab via URL heuristic", () => {
    setActiveProject(gitlabRemote as never);
    expect(get(projectProvider)?.kind).toBe("gitlab");
  });

  it("[github × none] → github when repoConfig explicitly selects it", () => {
    setActiveProject(noRemote as never);
    repoConfig.set({ provider: "github" });
    expect(get(projectProvider)?.kind).toBe("github");
  });

  it("[github × github-remote] → github (repoConfig wins, same result)", () => {
    setActiveProject(githubRemote as never);
    repoConfig.set({ provider: "github" });
    expect(get(projectProvider)?.kind).toBe("github");
  });

  it("[github × gitlab-remote] → github (repoConfig overrides remote hint)", () => {
    setActiveProject(gitlabRemote as never);
    repoConfig.set({ provider: "github" });
    expect(get(projectProvider)?.kind).toBe("github");
  });

  it("[gitlab × none] → gitlab when repoConfig explicitly selects it", () => {
    setActiveProject(noRemote as never);
    repoConfig.set({ provider: "gitlab" });
    expect(get(projectProvider)?.kind).toBe("gitlab");
  });

  it("[gitlab × github-remote] → gitlab (repoConfig overrides remote hint)", () => {
    setActiveProject(githubRemote as never);
    repoConfig.set({ provider: "gitlab" });
    expect(get(projectProvider)?.kind).toBe("gitlab");
  });

  it("[gitlab × gitlab-remote] → gitlab (repoConfig wins, same result)", () => {
    setActiveProject(gitlabRemote as never);
    repoConfig.set({ provider: "gitlab" });
    expect(get(projectProvider)?.kind).toBe("gitlab");
  });

  // --- Additional self-hosted / edge coverage ----------------------------

  it("detects self-hosted gitlab by hostname", () => {
    setActiveProject({
      ...noRemote,
      remotes: [{ name: "origin", url: "https://gitlab.acme.com/x/y.git" }],
    });
    expect(get(projectProvider)?.kind).toBe("gitlab");
  });

  it("does NOT infer gitlab for custom-domain self-hosted (no `gitlab.` in host)", () => {
    setActiveProject({
      ...noRemote,
      remotes: [{ name: "origin", url: "https://code.example.org/x/y.git" }],
    });
    expect(get(projectProvider)).toBeNull();
  });

  it("ignores non-origin remotes when inferring", () => {
    setActiveProject({
      ...noRemote,
      remotes: [{ name: "upstream", url: "git@github.com:x/y.git" }],
    });
    expect(get(projectProvider)).toBeNull();
  });

  it("returns null when heuristic matches but provider is not connected", () => {
    providerStatus.set({ providers: [], active_index: null });
    setActiveProject(githubRemote as never);
    expect(get(projectProvider)).toBeNull();
  });
});
