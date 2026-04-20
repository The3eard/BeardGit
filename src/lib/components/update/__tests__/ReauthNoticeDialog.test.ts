/**
 * Unit tests for `ReauthNoticeDialog.svelte`.
 *
 * Verifies the dialog:
 * - renders the macOS variant body when `os="macos"`
 * - renders the Windows variant body when `os="windows"`
 * - always surfaces the apology sentence
 * - fires `onConfirm(false)` when the user clicks **Update now** with the
 *   checkbox unticked
 * - fires `onConfirm(true)` when the checkbox was ticked first
 * - fires `onCancel` when the user clicks **Cancel**
 * - fires `onCancel` when the user presses **Escape**
 * - renders nothing when `open=false`
 */

import { describe, expect, it, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import ReauthNoticeDialog from "../ReauthNoticeDialog.svelte";

afterEach(() => cleanup());

describe("ReauthNoticeDialog", () => {
  it("renders the macOS body variant and the apology sentence", () => {
    const { getByTestId } = render(ReauthNoticeDialog, {
      props: {
        open: true,
        os: "macos",
        onConfirm: vi.fn(),
        onCancel: vi.fn(),
      },
    });

    const body = getByTestId("reauth-body");
    expect(body.textContent).toMatch(/Gatekeeper/i);
    const sorry = getByTestId("reauth-sorry");
    expect(sorry.textContent?.toLowerCase()).toContain("sorry");
    const root = getByTestId("reauth-dialog");
    expect(root.getAttribute("data-os")).toBe("macos");
  });

  it("renders the Windows body variant", () => {
    const { getByTestId } = render(ReauthNoticeDialog, {
      props: {
        open: true,
        os: "windows",
        onConfirm: vi.fn(),
        onCancel: vi.fn(),
      },
    });

    const body = getByTestId("reauth-body");
    expect(body.textContent).toMatch(/SmartScreen/i);
    const root = getByTestId("reauth-dialog");
    expect(root.getAttribute("data-os")).toBe("windows");
  });

  it("fires onConfirm(false) when the checkbox is unticked", async () => {
    const onConfirm = vi.fn();
    const { getByTestId } = render(ReauthNoticeDialog, {
      props: {
        open: true,
        os: "macos",
        onConfirm,
        onCancel: vi.fn(),
      },
    });

    await fireEvent.click(getByTestId("reauth-confirm"));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenCalledWith(false);
  });

  it("fires onConfirm(true) when the dismiss checkbox is ticked first", async () => {
    const onConfirm = vi.fn();
    const { getByTestId } = render(ReauthNoticeDialog, {
      props: {
        open: true,
        os: "macos",
        onConfirm,
        onCancel: vi.fn(),
      },
    });

    const checkbox = getByTestId(
      "reauth-dismiss-checkbox",
    ) as HTMLInputElement;
    await fireEvent.click(checkbox);
    expect(checkbox.checked).toBe(true);

    await fireEvent.click(getByTestId("reauth-confirm"));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenCalledWith(true);
  });

  it("fires onCancel when the user clicks Cancel", async () => {
    const onCancel = vi.fn();
    const { getByTestId } = render(ReauthNoticeDialog, {
      props: {
        open: true,
        os: "macos",
        onConfirm: vi.fn(),
        onCancel,
      },
    });

    await fireEvent.click(getByTestId("reauth-cancel"));

    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("fires onCancel when the user presses Escape", async () => {
    const onCancel = vi.fn();
    render(ReauthNoticeDialog, {
      props: {
        open: true,
        os: "macos",
        onConfirm: vi.fn(),
        onCancel,
      },
    });

    await fireEvent.keyDown(window, { key: "Escape" });

    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("renders nothing when open=false", () => {
    const { queryByTestId } = render(ReauthNoticeDialog, {
      props: {
        open: false,
        os: "macos",
        onConfirm: vi.fn(),
        onCancel: vi.fn(),
      },
    });

    expect(queryByTestId("reauth-dialog")).toBeNull();
  });
});
