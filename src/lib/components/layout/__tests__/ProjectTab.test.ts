/**
 * Unit tests for `ProjectTab.svelte` — focus on the new repo-config
 * entry points added in Phase 6.7:
 *   - Active tab shows a settings cog that opens the Repo-config
 *     dialog and stops click propagation.
 *   - Right-click opens a context menu with a "Repo settings" item.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import ProjectTab from "../ProjectTab.svelte";
import { repoConfigDialogOpen } from "$lib/stores/repoConfig";
import type { ProjectInfo } from "$lib/types";

const project = {
  path: "/tmp/repo",
  name: "repo",
  change_count: 0,
} as unknown as ProjectInfo;

beforeEach(() => {
  repoConfigDialogOpen.set(false);
});

afterEach(() => cleanup());

describe("ProjectTab — repo config entry points", () => {
  it("clicking the cog on the active tab opens the dialog", async () => {
    const { getByTestId } = render(ProjectTab, {
      props: {
        project,
        isActive: true,
        index: 0,
        onSwitch: () => {},
        onClose: () => {},
      },
    });
    const cog = getByTestId("project-tab-settings") as HTMLButtonElement;
    await fireEvent.click(cog);
    expect(get(repoConfigDialogOpen)).toBe(true);
  });

  it("hides the cog on inactive tabs to keep the chrome tight", () => {
    const { queryByTestId } = render(ProjectTab, {
      props: {
        project,
        isActive: false,
        index: 1,
        onSwitch: () => {},
        onClose: () => {},
      },
    });
    expect(queryByTestId("project-tab-settings")).toBeNull();
  });

  it("right-clicking the tab reveals a 'Repo settings' context menu", async () => {
    const { container, getByTestId } = render(ProjectTab, {
      props: {
        project,
        isActive: true,
        index: 0,
        onSwitch: () => {},
        onClose: () => {},
      },
    });
    const tab = container.querySelector(".project-tab") as HTMLElement;
    await fireEvent.contextMenu(tab);
    const menu = getByTestId("project-tab-context-menu");
    expect(menu).toBeTruthy();
    const item = getByTestId("project-tab-context-settings");
    await fireEvent.click(item);
    expect(get(repoConfigDialogOpen)).toBe(true);
  });
});
