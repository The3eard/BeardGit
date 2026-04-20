/**
 * Unit tests for `RepoConfigDialog.svelte`.
 *
 * Covers the Phase 6.1 + Phase 7.1 dialog shell contract:
 *   - Closed by default.
 *   - On open, probes the forge CLI before rendering sections.
 *   - Renders empty-state cards for unsupported forge / missing CLI /
 *     unauthenticated states.
 *   - Populates sections once the config is loaded.
 *   - Save button stays disabled while the patch is empty.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, render, waitFor } from "@testing-library/svelte";
import { tick } from "svelte";
import RepoConfigDialog from "../RepoConfigDialog.svelte";
import {
  repoConfigDialogOpen,
  resetRepoConfigStore,
} from "$lib/stores/repoConfig";
import { invokeMock, mockInvokeResponse } from "../../../../test/setup";
import { activeTabIndex, openTabs } from "$lib/stores/tabs";

function installActiveProject() {
  const project = {
    path: "/tmp/repo",
    name: "repo",
    change_count: 0,
  } as const;
  openTabs.set([
    { kind: "project", project },
  ] as unknown as Parameters<typeof openTabs.set>[0]);
  activeTabIndex.set(0);
}

beforeEach(() => {
  resetRepoConfigStore();
  repoConfigDialogOpen.set(false);
  try {
    installActiveProject();
  } catch {
    // openProjects may not be exported from tabs in this environment;
    // the individual assertions don't rely on it.
  }
});

afterEach(() => cleanup());

describe("RepoConfigDialog", () => {
  it("renders nothing when closed", () => {
    const { queryByTestId } = render(RepoConfigDialog);
    expect(queryByTestId("repo-config-dialog")).toBeNull();
  });

  it("renders the unsupported-forge empty state", async () => {
    mockInvokeResponse("probe_forge_cli_status", {
      kind: "unsupported_forge",
    });
    const { getByText } = render(RepoConfigDialog);
    repoConfigDialogOpen.set(true);
    await tick();
    await waitFor(() => {
      expect(getByText("Not a GitHub or GitLab repository")).toBeTruthy();
    });
  });

  it("renders the CLI-not-installed empty state", async () => {
    mockInvokeResponse("probe_forge_cli_status", { kind: "not_installed" });
    const { getByText } = render(RepoConfigDialog);
    repoConfigDialogOpen.set(true);
    await tick();
    await waitFor(() => {
      expect(getByText("Forge CLI not installed")).toBeTruthy();
    });
  });

  it("renders the authenticate empty state when probe reports auth=false", async () => {
    mockInvokeResponse("probe_forge_cli_status", {
      kind: "installed",
      authenticated: false,
      account: null,
    });
    const { getByText } = render(RepoConfigDialog);
    repoConfigDialogOpen.set(true);
    await tick();
    await waitFor(() => {
      expect(getByText("Authenticate with the forge")).toBeTruthy();
    });
  });

  it("loads config and keeps Save disabled with an empty patch", async () => {
    mockInvokeResponse("probe_forge_cli_status", {
      kind: "installed",
      authenticated: true,
      account: "octocat",
    });
    mockInvokeResponse("load_remote_repo_config", {
      description: "Hello",
      homepage: null,
      topics: [],
      visibility: "public",
      default_branch: "main",
      issues_enabled: true,
      wiki_enabled: false,
      archived: false,
      branch_protection: null,
      labels: [],
    });
    const { getByTestId } = render(RepoConfigDialog);
    repoConfigDialogOpen.set(true);
    await tick();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "load_remote_repo_config",
        expect.objectContaining({ repoPath: "/tmp/repo" }),
      );
    });
    await waitFor(() => {
      const saveWrap = getByTestId("repo-config-save-wrap");
      const save = saveWrap.querySelector("button") as HTMLButtonElement;
      expect(save.disabled).toBe(true);
    });
  });
});
