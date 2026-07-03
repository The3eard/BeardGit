/**
 * Unit tests for `RefPicker.svelte` (spec 10 compare header).
 *
 * The picker is a self-contained autocomplete over branches + tags with
 * free-form SHA entry — no store deps — so these render it directly with
 * props and assert the selection contract:
 * - the input mirrors the committed `value`,
 * - typing filters the branch/tag options,
 * - clicking a suggestion emits that ref via `onSelect`,
 * - pressing Enter on a non-matching typed value emits the raw text (SHA),
 * - Escape closes the list without emitting.
 */
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { tick } from "svelte";
import RefPicker, { type RefOption } from "../RefPicker.svelte";

const options: RefOption[] = [
  { name: "main", kind: "branch" },
  { name: "feature/x", kind: "branch" },
  { name: "v1.0.0", kind: "tag" },
];

afterEach(() => cleanup());

describe("RefPicker", () => {
  it("mirrors the committed value into the input", () => {
    const { getByLabelText } = render(RefPicker, {
      props: { label: "Base", value: "main", options, onSelect: () => {} },
    });
    expect((getByLabelText("Base") as HTMLInputElement).value).toBe("main");
  });

  it("opens the option list on focus and shows branch/tag kinds", async () => {
    const { getByLabelText, getAllByRole } = render(RefPicker, {
      props: { label: "Base", value: null, options, onSelect: () => {} },
    });
    await fireEvent.focus(getByLabelText("Base"));
    await tick();
    const items = getAllByRole("option");
    expect(items).toHaveLength(3);
    expect(items[0].textContent).toContain("branch");
    expect(items[2].textContent).toContain("tag");
  });

  it("filters the options by the typed query", async () => {
    const { getByLabelText, getAllByRole } = render(RefPicker, {
      props: { label: "Base", value: null, options, onSelect: () => {} },
    });
    const input = getByLabelText("Base") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "feat" } });
    await tick();
    const items = getAllByRole("option");
    expect(items).toHaveLength(1);
    expect(items[0].textContent).toContain("feature/x");
  });

  it("emits the selected ref when a suggestion is clicked", async () => {
    const onSelect = vi.fn();
    const { getByLabelText, getAllByRole } = render(RefPicker, {
      props: { label: "Base", value: null, options, onSelect },
    });
    await fireEvent.focus(getByLabelText("Base"));
    await tick();
    // onmousedown drives the selection (fires before blur closes the list).
    await fireEvent.mouseDown(getAllByRole("option")[2]);
    expect(onSelect).toHaveBeenCalledWith("v1.0.0");
  });

  it("emits the raw typed value on Enter for a free-form SHA", async () => {
    const onSelect = vi.fn();
    const { getByLabelText } = render(RefPicker, {
      props: { label: "Compare", value: null, options, onSelect },
    });
    const input = getByLabelText("Compare") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "deadbeef" } });
    await fireEvent.keyDown(input, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("deadbeef");
  });

  it("does not emit and resets the query on Escape", async () => {
    const onSelect = vi.fn();
    const { getByLabelText } = render(RefPicker, {
      props: { label: "Base", value: "main", options, onSelect },
    });
    const input = getByLabelText("Base") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "xyz" } });
    await fireEvent.keyDown(input, { key: "Escape" });
    await tick();
    expect(onSelect).not.toHaveBeenCalled();
    expect(input.value).toBe("main");
  });
});
