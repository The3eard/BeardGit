/**
 * Unit tests for `Card.svelte`.
 *
 * Verifies:
 * - Renders a title and description when the props are set.
 * - The `actions` slot renders in the header-right.
 * - A card without any header props falls back to a plain rounded
 *   container (no header element emitted).
 * - The body slot renders its children.
 */

import { describe, expect, it, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import Card from "../Card.svelte";

afterEach(() => cleanup());

function textSnippet(text: string) {
  return createRawSnippet(() => ({
    render: () => `<span>${text}</span>`,
  }));
}

describe("Card", () => {
  it("renders title + description in the header", () => {
    const { container } = render(Card, {
      props: { title: "Theme", description: "Controls look and feel" },
    });
    const title = container.querySelector(".bg-card__title");
    const description = container.querySelector(".bg-card__description");
    expect(title?.textContent).toBe("Theme");
    expect(description?.textContent).toBe("Controls look and feel");
  });

  it("renders the actions slot in the header-right", () => {
    const { container } = render(Card, {
      props: {
        title: "Theme",
        actions: textSnippet("Reset"),
      },
    });
    const actions = container.querySelector(".bg-card__actions");
    expect(actions).toBeTruthy();
    expect(actions!.textContent).toContain("Reset");
  });

  it("falls back to a plain container when no header props are provided", () => {
    const { container } = render(Card, { props: {} });
    const card = container.querySelector(".bg-card");
    const header = container.querySelector(".bg-card__header");
    expect(card).toBeTruthy();
    expect(header).toBeNull();
  });

  it("renders the default slot (body) when children is provided", () => {
    const { container } = render(Card, {
      props: {
        children: textSnippet("body text"),
      },
    });
    const body = container.querySelector(".bg-card__body");
    expect(body).toBeTruthy();
    expect(body!.textContent).toContain("body text");
  });
});
