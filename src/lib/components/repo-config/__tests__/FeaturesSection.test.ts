/**
 * Unit tests for `FeaturesSection.svelte`.
 *
 * Verifies the Issues / Wiki toggles + default-branch dropdown
 * write straight through to `repoConfigStore.current`.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import FeaturesSection from "../FeaturesSection.svelte";
import {
  repoConfigStore,
  resetRepoConfigStore,
  setLoadedConfig,
} from "$lib/stores/repoConfig";
import { branches } from "$lib/stores/branches";
import type { RemoteRepoConfig } from "$lib/types/repoConfig";

function loaded(): RemoteRepoConfig {
  return {
    description: "",
    homepage: null,
    topics: [],
    visibility: "public",
    default_branch: "main",
    issues_enabled: true,
    wiki_enabled: true,
    archived: false,
    branch_protection: null,
    labels: [],
  };
}

beforeEach(() => {
  resetRepoConfigStore();
  setLoadedConfig("/tmp/repo", loaded());
  branches.set([
    { name: "main", is_remote: false, is_head: true, upstream: null },
    { name: "develop", is_remote: false, is_head: false, upstream: null },
  ] as unknown as Parameters<typeof branches.set>[0]);
});

afterEach(() => cleanup());

describe("FeaturesSection", () => {
  it("toggling Issues writes false to the store", async () => {
    const { getByTestId } = render(FeaturesSection);
    const input = getByTestId("repo-config-issues") as HTMLInputElement;
    expect(input.checked).toBe(true);
    await fireEvent.change(input, { target: { checked: false } });
    expect(get(repoConfigStore).current?.issues_enabled).toBe(false);
  });

  it("toggling Wiki writes false to the store", async () => {
    const { getByTestId } = render(FeaturesSection);
    const input = getByTestId("repo-config-wiki") as HTMLInputElement;
    await fireEvent.change(input, { target: { checked: false } });
    expect(get(repoConfigStore).current?.wiki_enabled).toBe(false);
  });

  it("changing the default branch writes the new value to the store", async () => {
    const { getByTestId } = render(FeaturesSection);
    const select = getByTestId(
      "repo-config-default-branch",
    ) as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: "develop" } });
    expect(get(repoConfigStore).current?.default_branch).toBe("develop");
  });
});
