/**
 * Unit tests for `TwoLineRow.svelte`.
 *
 * Verifies:
 * - Renders all four line-1 slots (leadIcon, keyLabel, title, trailingDate).
 * - Renders the meta slot on line 2.
 * - Omits the `keyLabel` column when the snippet is undefined.
 * - Applies the `two-line-row--selected` modifier iff `selected={true}`.
 * - Omits `trailingDate` column when the snippet is undefined.
 */

import { describe, expect, it, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import TwoLineRow from "../TwoLineRow.svelte";

afterEach(() => cleanup());

const snip = (html: string) =>
  createRawSnippet(() => ({ render: () => html }));

describe("TwoLineRow", () => {
  it("renders all four line-1 slots and the meta line", () => {
    const { container } = render(TwoLineRow, {
      props: {
        leadIcon: snip('<span data-testid="lead">L</span>'),
        keyLabel: snip('<span data-testid="key">#42</span>'),
        title: snip('<span data-testid="title">T</span>'),
        trailingDate: snip('<span data-testid="date">today</span>'),
        meta: snip('<span data-testid="meta">M</span>'),
        selected: false,
      },
    });
    expect(container.querySelector('[data-testid="lead"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="key"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="title"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="date"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="meta"]')).not.toBeNull();
  });

  it("omits the keyLabel column when snippet is undefined", () => {
    const { container } = render(TwoLineRow, {
      props: {
        leadIcon: snip("L"),
        title: snip("T"),
        meta: snip("M"),
        selected: false,
      },
    });
    expect(container.querySelector(".two-line-row__key")).toBeNull();
  });

  it("omits the trailingDate column when snippet is undefined", () => {
    const { container } = render(TwoLineRow, {
      props: {
        leadIcon: snip("L"),
        title: snip("T"),
        meta: snip("M"),
        selected: false,
      },
    });
    expect(container.querySelector(".two-line-row__date")).toBeNull();
  });

  it("applies the selected modifier class when selected is true", () => {
    const { container } = render(TwoLineRow, {
      props: {
        leadIcon: snip("L"),
        title: snip("T"),
        meta: snip("M"),
        selected: true,
      },
    });
    expect(
      container.querySelector(".two-line-row")?.classList.contains("two-line-row--selected"),
    ).toBe(true);
  });

  it("does not apply the selected modifier when selected is false", () => {
    const { container } = render(TwoLineRow, {
      props: {
        leadIcon: snip("L"),
        title: snip("T"),
        meta: snip("M"),
        selected: false,
      },
    });
    expect(
      container.querySelector(".two-line-row")?.classList.contains("two-line-row--selected"),
    ).toBe(false);
  });
});
