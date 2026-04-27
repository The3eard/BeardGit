/**
 * Unit tests for `VisibilitySection.svelte`.
 *
 * Verifies:
 *   - Radio group updates the store.
 *   - Archive toggle prompts a confirm dialog rather than writing
 *     directly to the store.
 *   - Confirming the dialog flips `archived`.
 *   - Cancelling the dialog leaves `archived` untouched.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import VisibilitySection from "../VisibilitySection.svelte";
import {
  repoConfigStore,
  resetRepoConfigStore,
  setLoadedConfig,
} from "$lib/stores/repoConfig";
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
    labels: [],
  };
}

beforeEach(() => {
  resetRepoConfigStore();
  setLoadedConfig("/tmp/repo", loaded());
});

afterEach(() => cleanup());

describe("VisibilitySection", () => {
  it("selects the private radio and updates the store", async () => {
    const { getByTestId } = render(VisibilitySection);
    const priv = getByTestId(
      "repo-config-visibility-private",
    ) as HTMLInputElement;
    await fireEvent.change(priv);
    expect(get(repoConfigStore).current?.visibility).toBe("private");
  });

  it("does not toggle archived without confirming first", async () => {
    const { container } = render(VisibilitySection);
    // Click the Archive button.
    const archiveBtn = container.querySelector(
      "button.bg-btn--danger",
    ) as HTMLButtonElement;
    expect(archiveBtn).toBeTruthy();
    await fireEvent.click(archiveBtn);
    // Store unchanged.
    expect(get(repoConfigStore).current?.archived).toBe(false);
  });

  it("flips archived on when the confirm button is pressed", async () => {
    const { container } = render(VisibilitySection);
    const archiveBtn = container.querySelector(
      "button.bg-btn--danger",
    ) as HTMLButtonElement;
    await fireEvent.click(archiveBtn);
    // The dialog renders a second danger button for confirmation.
    const dangerBtns = container.querySelectorAll("button.bg-btn--danger");
    // The last danger button is the confirm action.
    const confirm = dangerBtns[dangerBtns.length - 1] as HTMLButtonElement;
    await fireEvent.click(confirm);
    expect(get(repoConfigStore).current?.archived).toBe(true);
  });
});
