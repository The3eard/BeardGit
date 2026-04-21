/**
 * Unit tests for `ProtectionSection.svelte`.
 *
 * Verifies:
 *   - GitLab / null forge renders the "not supported" Card.
 *   - GitHub forge + no selected branch hides the rules form.
 *   - Selecting a branch loads protection via `get_branch_protection`.
 *   - Saving emits `set_branch_protection` with the mutated rule set.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import ProtectionSection from "../ProtectionSection.svelte";
import { branches } from "$lib/stores/branches";
import { invokeMock, mockInvokeResponse } from "../../../../test/setup";

beforeEach(() => {
  branches.set([
    { name: "main", is_remote: false, is_head: true, upstream: null },
  ] as unknown as Parameters<typeof branches.set>[0]);
});

afterEach(() => cleanup());

describe("ProtectionSection", () => {
  it("renders a 'not supported' card when the forge is GitLab", () => {
    const { getByText } = render(ProtectionSection, {
      props: { forge: "gitlab", repoPath: "/tmp/repo" },
    });
    expect(getByText("Not supported on this provider")).toBeTruthy();
  });

  it("renders a 'not supported' card when the forge is null", () => {
    const { getByText } = render(ProtectionSection, {
      props: { forge: null, repoPath: "/tmp/repo" },
    });
    expect(getByText("Not supported on this provider")).toBeTruthy();
  });

  it("hides rules form until a branch is selected", () => {
    const { queryByTestId } = render(ProtectionSection, {
      props: { forge: "github", repoPath: "/tmp/repo" },
    });
    expect(queryByTestId("protect-require-pr")).toBeNull();
  });

  it("loads protection for the selected branch via the Tauri command", async () => {
    mockInvokeResponse("get_branch_protection", {
      require_pull_request: true,
      required_approvals: 2,
      require_status_checks: false,
      status_check_contexts: [],
      require_up_to_date: false,
      require_conversation_resolution: false,
      enforce_admins: false,
    });
    const { getByTestId } = render(ProtectionSection, {
      props: { forge: "github", repoPath: "/tmp/repo" },
    });
    const select = getByTestId(
      "repo-config-protection-branch",
    ) as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: "main" } });
    await tick();
    const pr = getByTestId("protect-require-pr") as HTMLInputElement;
    expect(pr.checked).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith(
      "get_branch_protection",
      expect.objectContaining({ repoPath: "/tmp/repo", branch: "main" }),
    );
  });

  it("emits set_branch_protection when Apply is pressed", async () => {
    mockInvokeResponse("get_branch_protection", null);
    mockInvokeResponse("set_branch_protection", undefined);
    const { getByTestId, container } = render(ProtectionSection, {
      props: { forge: "github", repoPath: "/tmp/repo" },
    });
    const select = getByTestId(
      "repo-config-protection-branch",
    ) as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: "main" } });
    await tick();
    // Flip the PR requirement so Save emits a payload.
    const pr = getByTestId("protect-require-pr") as HTMLInputElement;
    await fireEvent.change(pr, { target: { checked: true } });
    // Find the Apply button by its label text.
    const applyBtn = Array.from(
      container.querySelectorAll("button"),
    ).find((b) => b.textContent?.includes("Apply")) as HTMLButtonElement;
    expect(applyBtn).toBeTruthy();
    await fireEvent.click(applyBtn);
    expect(invokeMock).toHaveBeenCalledWith(
      "set_branch_protection",
      expect.objectContaining({
        repoPath: "/tmp/repo",
        branch: "main",
        rules: expect.objectContaining({ require_pull_request: true }),
      }),
    );
  });
});
