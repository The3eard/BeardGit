import { afterEach, describe, it, expect, vi } from "vitest";
import { cleanup, render, fireEvent } from "@testing-library/svelte";
import SectionFooter from "../SectionFooter.svelte";

describe("SectionFooter", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders nothing when not dirty", () => {
    const { container } = render(SectionFooter, {
      dirty: false,
      saving: false,
      onSave: () => {},
      onDiscard: () => {},
    });
    expect(container.querySelector("[data-testid='section-footer']")).toBeNull();
  });

  it("renders save + discard when dirty", () => {
    const { getByTestId } = render(SectionFooter, {
      dirty: true,
      saving: false,
      onSave: () => {},
      onDiscard: () => {},
    });
    expect(getByTestId("section-footer")).toBeTruthy();
    expect(getByTestId("section-footer-save")).toBeTruthy();
    expect(getByTestId("section-footer-discard")).toBeTruthy();
  });

  it("calls onSave when Save is clicked", async () => {
    const onSave = vi.fn();
    const { getByTestId } = render(SectionFooter, {
      dirty: true,
      saving: false,
      onSave,
      onDiscard: () => {},
    });
    await fireEvent.click(getByTestId("section-footer-save"));
    expect(onSave).toHaveBeenCalledOnce();
  });

  it("calls onDiscard when Discard is clicked", async () => {
    const onDiscard = vi.fn();
    const { getByTestId } = render(SectionFooter, {
      dirty: true,
      saving: false,
      onSave: () => {},
      onDiscard,
    });
    await fireEvent.click(getByTestId("section-footer-discard"));
    expect(onDiscard).toHaveBeenCalledOnce();
  });
});
