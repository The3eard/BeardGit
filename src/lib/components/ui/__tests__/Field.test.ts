/**
 * Unit tests for `Field.svelte`.
 *
 * Verifies:
 * - The label's `for` attribute matches the control id.
 * - A description renders when provided, and is absent otherwise.
 * - An error message renders with role=alert when `error` is set.
 * - The root element gains the `bg-field--error` class when errored.
 */

import { describe, expect, it, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import Field from "../Field.svelte";

afterEach(() => cleanup());

function inputSnippet(id: string) {
  return createRawSnippet(() => ({
    render: () => `<input id="${id}" data-testid="field-input" />`,
  }));
}

describe("Field", () => {
  it("associates the label with the control via the `for` attribute", () => {
    const { container } = render(Field, {
      props: {
        label: "Endpoint",
        for: "endpoint-input",
        children: inputSnippet("endpoint-input"),
      },
    });
    const label = container.querySelector("label") as HTMLLabelElement;
    expect(label.getAttribute("for")).toBe("endpoint-input");
  });

  it("renders a description when provided", () => {
    const { container } = render(Field, {
      props: { label: "Endpoint", description: "Leave empty for defaults" },
    });
    const description = container.querySelector(".bg-field__description");
    expect(description?.textContent).toBe("Leave empty for defaults");
  });

  it("omits the description when not provided", () => {
    const { container } = render(Field, {
      props: { label: "Endpoint" },
    });
    expect(container.querySelector(".bg-field__description")).toBeNull();
  });

  it("renders the error message with role=alert when error is set", () => {
    const { queryByTestId } = render(Field, {
      props: { label: "Endpoint", error: "Required" },
    });
    const err = queryByTestId("bg-field-error");
    expect(err).toBeTruthy();
    expect(err!.getAttribute("role")).toBe("alert");
    expect(err!.textContent).toBe("Required");
  });

  it("adds the error modifier class to the root when error is set", () => {
    const { container } = render(Field, {
      props: { label: "Endpoint", error: "Required" },
    });
    const root = container.querySelector(".bg-field");
    expect(root?.classList.contains("bg-field--error")).toBe(true);
  });
});
