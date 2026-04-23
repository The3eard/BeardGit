/**
 * Unit tests for `List.svelte` — covers the new `refreshing` prop and
 * confirms the existing behaviours (loading spinner on empty, loading bar
 * when items + loading) still hold.
 */

import { describe, expect, it, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import List from "../List.svelte";

afterEach(() => cleanup());

const rowSnippet = createRawSnippet<[{ item: { id: string; label: string }; selected: boolean }]>(
  (getArgs) => ({
    render: () => {
      const { item } = getArgs();
      return `<span data-testid="row-${item.id}">${item.label}</span>`;
    },
  }),
);

const baseProps = {
  items: [
    { id: "a", label: "A" },
    { id: "b", label: "B" },
  ],
  loading: false,
  title: "Demo",
  selectedKey: null,
  getKey: (i: { id: string }) => i.id,
  row: rowSnippet,
};

describe("List.refreshing", () => {
  it("renders the loading bar when refreshing=true and items exist", () => {
    const { getByTestId } = render(List, {
      props: { ...baseProps, refreshing: true },
    });
    expect(getByTestId("list-loading-bar")).toBeTruthy();
  });

  it("does NOT render the loading bar when refreshing=true but items is empty", () => {
    const { queryByTestId } = render(List, {
      props: { ...baseProps, items: [], refreshing: true },
    });
    expect(queryByTestId("list-loading-bar")).toBeNull();
  });

  it("renders the loading bar for the legacy loading=true path with items", () => {
    const { getByTestId } = render(List, {
      props: { ...baseProps, loading: true },
    });
    expect(getByTestId("list-loading-bar")).toBeTruthy();
  });

  it("renders the centred spinner when loading=true and items is empty", () => {
    const { container } = render(List, {
      props: { ...baseProps, items: [], loading: true },
    });
    expect(container.querySelector(".list-loading .spinner")).not.toBeNull();
    expect(container.querySelector('[data-testid="list-loading-bar"]')).toBeNull();
  });
});
