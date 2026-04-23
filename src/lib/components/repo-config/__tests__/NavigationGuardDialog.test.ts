import { describe, it, expect, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import NavigationGuardDialog from "../NavigationGuardDialog.svelte";

afterEach(() => cleanup());

describe("NavigationGuardDialog", () => {
  it("does not render its body when open=false", () => {
    const { queryByTestId } = render(NavigationGuardDialog, {
      open: false,
      sectionLabel: "General",
      saving: false,
      onSave: () => {},
      onDiscard: () => {},
      onCancel: () => {},
    });
    expect(queryByTestId("repo-config-guard-save")).toBeNull();
  });

  it("wires onSave / onDiscard / onCancel", async () => {
    const onSave = vi.fn();
    const onDiscard = vi.fn();
    const onCancel = vi.fn();
    const { getByTestId } = render(NavigationGuardDialog, {
      open: true,
      sectionLabel: "General",
      saving: false,
      onSave,
      onDiscard,
      onCancel,
    });
    await fireEvent.click(getByTestId("repo-config-guard-save"));
    await fireEvent.click(getByTestId("repo-config-guard-discard"));
    await fireEvent.click(getByTestId("repo-config-guard-cancel"));
    expect(onSave).toHaveBeenCalledOnce();
    expect(onDiscard).toHaveBeenCalledOnce();
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("renders the section label in the title", () => {
    const { getByText } = render(NavigationGuardDialog, {
      open: true,
      sectionLabel: "Protection",
      saving: false,
      onSave: () => {},
      onDiscard: () => {},
      onCancel: () => {},
    });
    expect(getByText(/Protection/)).toBeTruthy();
  });
});
