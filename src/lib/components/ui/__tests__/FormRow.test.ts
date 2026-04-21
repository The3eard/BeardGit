/**
 * Unit tests for `FormRow.svelte`.
 *
 * Verifies:
 * - The label's `for` attribute matches the control `id` when passed.
 * - Helper text renders when `helperText` is provided.
 * - Helper text is absent when not provided.
 * - The control slot renders its children.
 */

import { describe, expect, it, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import FormRow from "../FormRow.svelte";

afterEach(() => cleanup());

function inputSnippet(id: string) {
  return createRawSnippet(() => ({
    render: () => `<input id="${id}" data-testid="row-input" />`,
  }));
}

describe("FormRow", () => {
  it("associates the label with the control via the `for` attribute", () => {
    const { container } = render(FormRow, {
      props: {
        label: "Theme",
        for: "theme-select",
        children: inputSnippet("theme-select"),
      },
    });
    const label = container.querySelector("label") as HTMLLabelElement;
    expect(label.getAttribute("for")).toBe("theme-select");
    const input = container.querySelector(
      '[data-testid="row-input"]',
    ) as HTMLInputElement;
    expect(input.getAttribute("id")).toBe("theme-select");
  });

  it("renders helper text when provided", () => {
    const { queryByTestId } = render(FormRow, {
      props: { label: "Theme", helperText: "Persisted per user" },
    });
    const helper = queryByTestId("bg-form-row-helper");
    expect(helper).toBeTruthy();
    expect(helper!.textContent).toBe("Persisted per user");
  });

  it("omits helper text when not provided", () => {
    const { queryByTestId } = render(FormRow, {
      props: { label: "Theme" },
    });
    expect(queryByTestId("bg-form-row-helper")).toBeNull();
  });

  it("renders the control slot children", () => {
    const { container } = render(FormRow, {
      props: {
        label: "Theme",
        children: inputSnippet("theme-select"),
      },
    });
    expect(
      container.querySelector('[data-testid="row-input"]'),
    ).toBeTruthy();
  });
});
