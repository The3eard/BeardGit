/**
 * Unit tests for `TabBar.svelte` after the toolbar rework.
 *
 * Covers:
 * - Terminal button is a plain button (no chevron / no dropdown).
 * - AI button is hidden when no providers are installed.
 * - AI button opens a dropdown with one row per provider + a divider
 *   + a "Launch session in background…" row.
 * - Provider rows invoke `openAiTerminalTab` with the right kind.
 * - Background row invokes `requestOpenCreateBackgroundRunDialog`.
 * - Escape and outside-click close the menu.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import { get } from "svelte/store";
import { aiProviders } from "$lib/stores/ai";
import { openTabs, activeTabIndex } from "$lib/stores/tabs";
import {
  openCreateBackgroundRunDialogRequest,
} from "$lib/stores/aiBackground";
import type { AvailableAiProvider } from "$lib/types";

const tabsMocks = vi.hoisted(() => ({
  openTerminalTab: vi.fn().mockResolvedValue(1),
  openStandaloneTerminal: vi.fn().mockResolvedValue(2),
  openAiTerminalTab: vi.fn().mockResolvedValue(3),
}));

vi.mock("$lib/stores/tabs", async () => {
  const actual = await vi.importActual<typeof import("$lib/stores/tabs")>(
    "$lib/stores/tabs",
  );
  return {
    ...actual,
    openTerminalTab: tabsMocks.openTerminalTab,
    openStandaloneTerminal: tabsMocks.openStandaloneTerminal,
    openAiTerminalTab: tabsMocks.openAiTerminalTab,
  };
});

vi.mock("@tauri-apps/api/path", () => ({
  homeDir: vi.fn().mockResolvedValue("/Users/test"),
}));

import TabBar from "../TabBar.svelte";

const PROVIDERS: AvailableAiProvider[] = [
  { kind: "claude_code", binary_path: "/usr/local/bin/claude", version: "1.2.3" },
  { kind: "codex", binary_path: "/usr/local/bin/codex", version: null },
];

beforeEach(() => {
  aiProviders.set([]);
  openTabs.set([]);
  activeTabIndex.set(-1);
  openCreateBackgroundRunDialogRequest.set(0);
  tabsMocks.openTerminalTab.mockClear();
  tabsMocks.openStandaloneTerminal.mockClear();
  tabsMocks.openAiTerminalTab.mockClear();
});

afterEach(() => cleanup());

describe("TabBar — terminal button", () => {
  it("is a plain button (no chevron, no dropdown wrapper)", () => {
    const { container, queryByTestId } = render(TabBar);
    expect(container.querySelector(".terminal-split")).toBeNull();
    expect(container.querySelector(".terminal-right")).toBeNull();
    expect(queryByTestId("toolbar-terminal-menu")).toBeNull();
  });

  it("calls openStandaloneTerminal with the home path when no project tab is active", async () => {
    const { getByTestId } = render(TabBar);
    await fireEvent.click(getByTestId("toolbar-terminal-btn"));
    // handleTerminalClick is async (awaits a dynamic import); waitFor
    // lets microtasks settle before asserting.
    await waitFor(() =>
      expect(tabsMocks.openStandaloneTerminal).toHaveBeenCalledTimes(1),
    );
    expect(tabsMocks.openStandaloneTerminal.mock.calls[0][0]).toBe("/Users/test");
  });
});

describe("TabBar — AI dropdown", () => {
  it("is hidden when no providers are installed", () => {
    const { queryByTestId } = render(TabBar);
    expect(queryByTestId("toolbar-ai-btn")).toBeNull();
  });

  it("renders when at least one provider is installed", () => {
    aiProviders.set(PROVIDERS);
    const { getByTestId } = render(TabBar);
    expect(getByTestId("toolbar-ai-btn")).toBeTruthy();
  });

  it("toggles the menu on click with correct aria-expanded", async () => {
    aiProviders.set(PROVIDERS);
    const { getByTestId, queryByTestId } = render(TabBar);
    const btn = getByTestId("toolbar-ai-btn");
    expect(btn.getAttribute("aria-expanded")).toBe("false");
    expect(queryByTestId("toolbar-ai-menu")).toBeNull();

    await fireEvent.click(btn);
    expect(btn.getAttribute("aria-expanded")).toBe("true");
    expect(queryByTestId("toolbar-ai-menu")).toBeTruthy();

    await fireEvent.click(btn);
    expect(btn.getAttribute("aria-expanded")).toBe("false");
    expect(queryByTestId("toolbar-ai-menu")).toBeNull();
  });

  it("renders one row per provider with role=menuitem and a background row", async () => {
    aiProviders.set(PROVIDERS);
    const { getByTestId, getAllByRole } = render(TabBar);
    await fireEvent.click(getByTestId("toolbar-ai-btn"));
    const items = getAllByRole("menuitem");
    // 2 providers + 1 background row = 3
    expect(items.length).toBe(3);
    expect(getByTestId("toolbar-ai-item-claude_code")).toBeTruthy();
    expect(getByTestId("toolbar-ai-item-codex")).toBeTruthy();
    expect(getByTestId("toolbar-ai-item-background")).toBeTruthy();
  });

  it("calls openAiTerminalTab with the provider kind on provider-row click", async () => {
    aiProviders.set(PROVIDERS);
    const { getByTestId } = render(TabBar);
    await fireEvent.click(getByTestId("toolbar-ai-btn"));
    await fireEvent.click(getByTestId("toolbar-ai-item-claude_code"));
    expect(tabsMocks.openAiTerminalTab).toHaveBeenCalledTimes(1);
    expect(tabsMocks.openAiTerminalTab.mock.calls[0][2]).toBe("claude_code");
  });

  it("triggers the background-run dialog signal on the background row", async () => {
    aiProviders.set(PROVIDERS);
    const prev = get(openCreateBackgroundRunDialogRequest);
    const { getByTestId } = render(TabBar);
    await fireEvent.click(getByTestId("toolbar-ai-btn"));
    await fireEvent.click(getByTestId("toolbar-ai-item-background"));
    expect(get(openCreateBackgroundRunDialogRequest)).toBe(prev + 1);
  });

  it("closes the menu on Escape", async () => {
    aiProviders.set(PROVIDERS);
    const { getByTestId, queryByTestId } = render(TabBar);
    const btn = getByTestId("toolbar-ai-btn");
    await fireEvent.click(btn);
    expect(queryByTestId("toolbar-ai-menu")).toBeTruthy();
    await fireEvent.keyDown(btn, { key: "Escape" });
    expect(queryByTestId("toolbar-ai-menu")).toBeNull();
    expect(btn.getAttribute("aria-expanded")).toBe("false");
  });

  it("closes the menu on outside mousedown", async () => {
    aiProviders.set(PROVIDERS);
    const { getByTestId, queryByTestId } = render(TabBar);
    await fireEvent.click(getByTestId("toolbar-ai-btn"));
    expect(queryByTestId("toolbar-ai-menu")).toBeTruthy();
    // Fire a mousedown on document.body, outside the dropdown wrapper.
    await fireEvent.mouseDown(document.body);
    expect(queryByTestId("toolbar-ai-menu")).toBeNull();
  });
});
