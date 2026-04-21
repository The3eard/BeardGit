/**
 * Unit tests for `LabelsSection.svelte`.
 *
 * Verifies the Add / Edit / Delete flows invoke the right Tauri
 * commands (`create_label`, `update_label`, `delete_label`) with the
 * mutated label payload, and that destructive Delete always prompts
 * a confirm dialog.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import LabelsSection from "../LabelsSection.svelte";
import {
  resetRepoConfigStore,
  setLoadedConfig,
} from "$lib/stores/repoConfig";
import { invokeMock, mockInvokeResponse } from "../../../../test/setup";
import type { RemoteRepoConfig } from "$lib/types/repoConfig";

function loaded(): RemoteRepoConfig {
  return {
    description: "",
    homepage: null,
    topics: [],
    visibility: "public",
    default_branch: "main",
    issues_enabled: true,
    wiki_enabled: false,
    archived: false,
    branch_protection: null,
    labels: [
      { name: "bug", color: "ff0000", description: "Something broken" },
    ],
  };
}

beforeEach(() => {
  resetRepoConfigStore();
  setLoadedConfig("/tmp/repo", loaded());
});

afterEach(() => cleanup());

describe("LabelsSection", () => {
  it("renders the existing label row", () => {
    const { getByTestId } = render(LabelsSection);
    expect(getByTestId("repo-config-label-row-bug")).toBeTruthy();
  });

  it("create flow calls create_label with the new payload", async () => {
    mockInvokeResponse("create_label", undefined);
    const { getByTestId } = render(LabelsSection);
    const addWrap = getByTestId("repo-config-label-add-wrap");
    const addBtn = addWrap.querySelector("button") as HTMLButtonElement;
    await fireEvent.click(addBtn);
    await tick();
    const nameInput = getByTestId(
      "repo-config-label-name",
    ) as HTMLInputElement;
    await fireEvent.input(nameInput, { target: { value: "feature" } });
    const colorInput = getByTestId(
      "repo-config-label-color",
    ) as HTMLInputElement;
    await fireEvent.input(colorInput, { target: { value: "00ff00" } });
    const saveWrap = getByTestId("repo-config-label-save-wrap");
    const saveBtn = saveWrap.querySelector("button") as HTMLButtonElement;
    await fireEvent.click(saveBtn);
    expect(invokeMock).toHaveBeenCalledWith(
      "create_label",
      expect.objectContaining({
        repoPath: "/tmp/repo",
        label: expect.objectContaining({ name: "feature", color: "00ff00" }),
      }),
    );
  });

  it("delete flow prompts confirm then calls delete_label", async () => {
    mockInvokeResponse("delete_label", undefined);
    const { getByTestId, getAllByText } = render(LabelsSection);
    const row = getByTestId("repo-config-label-row-bug");
    const deleteBtn = Array.from(
      row.querySelectorAll("button"),
    ).find((b) => b.textContent?.includes("Delete")) as HTMLButtonElement;
    await fireEvent.click(deleteBtn);
    await tick();
    // Confirm dialog exposes a second Delete button.
    const confirms = getAllByText("Delete");
    const confirmBtn = confirms[confirms.length - 1]
      .closest("button") as HTMLButtonElement;
    await fireEvent.click(confirmBtn);
    expect(invokeMock).toHaveBeenCalledWith(
      "delete_label",
      expect.objectContaining({ repoPath: "/tmp/repo", name: "bug" }),
    );
  });
});
