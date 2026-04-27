/**
 * Unit tests for `SettingSection.svelte`.
 *
 * Verifies:
 * - The title renders.
 * - A non-collapsible section always shows its body.
 * - A collapsible section starts open when `defaultOpen=true` and
 *   hides its body when the toggle is clicked.
 * - A collapsible section starts closed when `defaultOpen=false`.
 * - Clicking the toggle of a collapsible section flips `aria-expanded`.
 */

import { describe, expect, it, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import SettingSection from "../SettingSection.svelte";

afterEach(() => cleanup());

function bodySnippet(html: string) {
  return createRawSnippet(() => ({ render: () => html }));
}

describe("SettingSection", () => {
  it("renders the title text", () => {
    const { container } = render(SettingSection, {
      props: { title: "Appearance" },
    });
    const title = container.querySelector(".bg-setting-section__title");
    expect(title?.textContent).toBe("Appearance");
  });

  it("always shows the body for a non-collapsible section", () => {
    const { queryByTestId } = render(SettingSection, {
      props: {
        title: "Appearance",
        collapsible: false,
        children: bodySnippet("<p>body text</p>"),
      },
    });
    expect(queryByTestId("bg-setting-section-body")).toBeTruthy();
  });

  it("starts open when collapsible + defaultOpen=true, hides body on toggle", async () => {
    const { queryByTestId, getByTestId } = render(SettingSection, {
      props: {
        title: "Appearance",
        collapsible: true,
        defaultOpen: true,
        children: bodySnippet("<p>body text</p>"),
      },
    });
    expect(queryByTestId("bg-setting-section-body")).toBeTruthy();

    await fireEvent.click(getByTestId("bg-setting-section-toggle"));
    expect(queryByTestId("bg-setting-section-body")).toBeNull();
  });

  it("starts closed when collapsible + defaultOpen=false, shows body on toggle", async () => {
    const { queryByTestId, getByTestId } = render(SettingSection, {
      props: {
        title: "Appearance",
        collapsible: true,
        defaultOpen: false,
        children: bodySnippet("<p>body text</p>"),
      },
    });
    expect(queryByTestId("bg-setting-section-body")).toBeNull();

    await fireEvent.click(getByTestId("bg-setting-section-toggle"));
    expect(queryByTestId("bg-setting-section-body")).toBeTruthy();
  });

  it("flips aria-expanded when toggled", async () => {
    const { getByTestId } = render(SettingSection, {
      props: {
        title: "Appearance",
        collapsible: true,
        defaultOpen: true,
        children: bodySnippet("<p>body text</p>"),
      },
    });
    const toggle = getByTestId("bg-setting-section-toggle");
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    await fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
  });
});
