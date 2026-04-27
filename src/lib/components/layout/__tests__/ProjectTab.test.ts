/**
 * Unit tests for `ProjectTab.svelte` after the cog + right-click
 * context menu were removed. Sidebar is now the sole entry point
 * for Repo settings.
 */
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import ProjectTab from "../ProjectTab.svelte";
import type { ProjectInfo } from "$lib/types";

afterEach(() => cleanup());

function makeProject(overrides: Partial<ProjectInfo> = {}): ProjectInfo {
  return {
    name: "demo",
    path: "/tmp/demo",
    change_count: 0,
    ...overrides,
  } as ProjectInfo;
}

describe("ProjectTab — no cog, no custom context menu", () => {
  it("does not render the settings cog on the active tab", () => {
    const { queryByTestId } = render(ProjectTab, {
      props: {
        project: makeProject(),
        isActive: true,
        index: 0,
        onSwitch: () => {},
        onClose: () => {},
      },
    });
    expect(queryByTestId("project-tab-settings")).toBeNull();
  });

  it("does not open a custom context menu on right-click", async () => {
    const { container, queryByTestId } = render(ProjectTab, {
      props: {
        project: makeProject(),
        isActive: true,
        index: 0,
        onSwitch: () => {},
        onClose: () => {},
      },
    });
    const root = container.firstElementChild as Element;
    await fireEvent.contextMenu(root);
    expect(queryByTestId("project-tab-context-menu")).toBeNull();
    expect(queryByTestId("project-tab-context-settings")).toBeNull();
  });

  it("calls onSwitch when clicked on an inactive tab", async () => {
    const onSwitch = vi.fn();
    const { container } = render(ProjectTab, {
      props: {
        project: makeProject(),
        isActive: false,
        index: 3,
        onSwitch,
        onClose: () => {},
      },
    });
    const root = container.firstElementChild as HTMLElement;
    await fireEvent.click(root);
    expect(onSwitch).toHaveBeenCalledWith(3);
  });

  it("calls onClose on middle-click", async () => {
    const onClose = vi.fn();
    const { container } = render(ProjectTab, {
      props: {
        project: makeProject(),
        isActive: true,
        index: 1,
        onSwitch: () => {},
        onClose,
      },
    });
    const root = container.firstElementChild as HTMLElement;
    // auxclick fires for middle-button in the real browser; dispatch a
    // synthetic one with button=1 so the component's guard triggers.
    await fireEvent(
      root,
      new MouseEvent("auxclick", { button: 1, bubbles: true }),
    );
    expect(onClose).toHaveBeenCalledWith(1);
  });
});
