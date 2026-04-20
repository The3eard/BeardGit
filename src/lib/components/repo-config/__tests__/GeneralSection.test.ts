/**
 * Unit tests for `GeneralSection.svelte`.
 *
 * Verifies the description / homepage / topics editor mutates the
 * `repoConfigStore.current` via the shared `updateCurrent` helper, and
 * that the topic chip input follows Enter-to-add / Backspace-to-remove
 * keyboard semantics.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import GeneralSection from "../GeneralSection.svelte";
import {
  repoConfigStore,
  resetRepoConfigStore,
  setLoadedConfig,
} from "$lib/stores/repoConfig";
import type { RemoteRepoConfig } from "$lib/types/repoConfig";

function loaded(): RemoteRepoConfig {
  return {
    description: "A repo",
    homepage: "https://example.com",
    topics: ["alpha"],
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

describe("GeneralSection", () => {
  it("renders the description, homepage and existing chips", () => {
    const { getByTestId, queryAllByTestId } = render(GeneralSection);
    expect(
      (getByTestId("repo-config-description") as HTMLTextAreaElement).value,
    ).toBe("A repo");
    expect(
      (getByTestId("repo-config-homepage") as HTMLInputElement).value,
    ).toBe("https://example.com");
    expect(queryAllByTestId("repo-config-topic-chip").length).toBe(1);
  });

  it("adds a chip when the user types and presses Enter", async () => {
    const { getByTestId, queryAllByTestId } = render(GeneralSection);
    const input = getByTestId("repo-config-topic-input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "beta" } });
    await fireEvent.keyDown(input, { key: "Enter" });
    expect(queryAllByTestId("repo-config-topic-chip").length).toBe(2);
    expect(get(repoConfigStore).current?.topics).toContain("beta");
  });

  it("removes the last chip when Backspace is pressed on empty input", async () => {
    const { getByTestId, queryAllByTestId } = render(GeneralSection);
    const input = getByTestId("repo-config-topic-input") as HTMLInputElement;
    await fireEvent.keyDown(input, { key: "Backspace" });
    expect(queryAllByTestId("repo-config-topic-chip").length).toBe(0);
    expect(get(repoConfigStore).current?.topics).toEqual([]);
  });

  it("edits the description via input events", async () => {
    const { getByTestId } = render(GeneralSection);
    const textarea = getByTestId(
      "repo-config-description",
    ) as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: "Refreshed" } });
    expect(get(repoConfigStore).current?.description).toBe("Refreshed");
  });

  it("maps an empty homepage input to `null` in the store", async () => {
    const { getByTestId } = render(GeneralSection);
    const input = getByTestId("repo-config-homepage") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "" } });
    expect(get(repoConfigStore).current?.homepage).toBeNull();
  });
});
