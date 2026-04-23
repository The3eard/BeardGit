/**
 * Unit tests for `List.svelte` — covers the new `refreshing` prop and
 * confirms the existing behaviours (loading spinner on empty, loading bar
 * when items + loading) still hold.
 */

import { describe, expect, it, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { createRawSnippet, type Component } from "svelte";
import ListComponent from "../List.svelte";

// Cast to Component<any> to satisfy svelte-check's generic inference.
// The runtime behaviour is identical — only the type-checker gets the hint.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const List = ListComponent as unknown as Component<any>;

afterEach(() => cleanup());

type TestItem = { id: string; label: string };

const rowSnippet = createRawSnippet<[{ item: TestItem; selected: boolean }]>(
  (getArgs) => ({
    render: () => {
      const { item } = getArgs();
      return `<span data-testid="row-${item.id}">${item.label}</span>`;
    },
  }),
);

const baseProps: {
  items: TestItem[];
  loading: boolean;
  title: string;
  selectedKey: string | null;
  getKey: (item: TestItem) => string;
  row: typeof rowSnippet;
} = {
  items: [
    { id: "a", label: "A" },
    { id: "b", label: "B" },
  ],
  loading: false,
  title: "Demo",
  selectedKey: null,
  getKey: (i: TestItem) => i.id,
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
