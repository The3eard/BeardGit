import { describe, it, expect } from "vitest";
import type { RemoteRepoConfig } from "$lib/types/repoConfig";
import {
  isSectionDirty,
  sectionPatchFor,
  resetSectionFields,
} from "../patches";

function makeConfig(overrides: Partial<RemoteRepoConfig> = {}): RemoteRepoConfig {
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
    ...overrides,
  };
}

describe("isSectionDirty", () => {
  it("is false when before === current", () => {
    const c = makeConfig();
    expect(isSectionDirty("general", c, c)).toBe(false);
    expect(isSectionDirty("visibility", c, c)).toBe(false);
    expect(isSectionDirty("features", c, c)).toBe(false);
  });

  it("detects a general-section description change", () => {
    const before = makeConfig();
    const current = makeConfig({ description: "new" });
    expect(isSectionDirty("general", before, current)).toBe(true);
    expect(isSectionDirty("visibility", before, current)).toBe(false);
  });

  it("detects a visibility-section archive flip", () => {
    const before = makeConfig();
    const current = makeConfig({ archived: true });
    expect(isSectionDirty("visibility", before, current)).toBe(true);
    expect(isSectionDirty("general", before, current)).toBe(false);
  });

  it("detects a features-section default-branch change", () => {
    const before = makeConfig();
    const current = makeConfig({ default_branch: "develop" });
    expect(isSectionDirty("features", before, current)).toBe(true);
  });

  it("always returns false for protection + labels", () => {
    const before = makeConfig();
    const current = makeConfig({ labels: [{ name: "bug", color: "red", description: null }] });
    expect(isSectionDirty("protection", before, current)).toBe(false);
    expect(isSectionDirty("labels", before, current)).toBe(false);
  });
});

describe("sectionPatchFor", () => {
  it("emits only the fields owned by general", () => {
    const before = makeConfig();
    const current = makeConfig({
      description: "x",
      default_branch: "develop", // features, should NOT appear
    });
    const patch = sectionPatchFor("general", before, current);
    expect(patch.description).toBe("x");
    expect(patch.default_branch).toBeUndefined();
  });

  it("emits only visibility-owned fields", () => {
    const before = makeConfig();
    const current = makeConfig({
      visibility: "private",
      archived: true,
      description: "x", // general, should NOT appear
    });
    const patch = sectionPatchFor("visibility", before, current);
    expect(patch.visibility).toBe("private");
    expect(patch.archive).toBe(true);
    expect(patch.description).toBeUndefined();
  });

  it("emits only features-owned fields", () => {
    const before = makeConfig();
    const current = makeConfig({
      issues_enabled: false,
      wiki_enabled: true,
      default_branch: "develop",
      description: "x",
    });
    const patch = sectionPatchFor("features", before, current);
    expect(patch.issues_enabled).toBe(false);
    expect(patch.wiki_enabled).toBe(true);
    expect(patch.default_branch).toBe("develop");
    expect(patch.description).toBeUndefined();
  });

  it("returns an empty (all-unchanged) patch for protection + labels", () => {
    const before = makeConfig();
    const current = makeConfig({ description: "x", archived: true });
    expect(sectionPatchFor("protection", before, current).description).toBeUndefined();
    expect(sectionPatchFor("labels",     before, current).archive).toBeUndefined();
  });
});

describe("resetSectionFields", () => {
  it("restores only the active section's fields on the draft", () => {
    const before = makeConfig({ description: "orig" });
    const draft  = makeConfig({
      description: "edited",
      visibility: "private",
    });
    resetSectionFields("general", draft, before);
    expect(draft.description).toBe("orig"); // restored
    expect(draft.visibility).toBe("private"); // untouched
  });
});
