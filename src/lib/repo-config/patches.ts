/**
 * Per-section slices of the repo-config patch flow.
 *
 * Each section owns a fixed subset of `RemoteRepoConfig` fields (see
 * `sectionFields`). The helpers here project the shared
 * `{ before, current }` pair into a section-scoped patch and dirty
 * check. Protection + Labels use their own Tauri commands and never
 * participate in the patch — they return empty/false from these
 * helpers, which is what the navigation guard and footer rely on.
 */

import type {
  RemoteRepoConfig,
  RemoteRepoConfigPatch,
} from "$lib/types/repoConfig";
import { buildPatch } from "$lib/stores/repoConfig";
import type { SectionId } from "./sections";

/**
 * Which fields of `RemoteRepoConfig` each section is allowed to edit.
 * Protection and Labels edit their own out-of-band surfaces — no
 * patch-flow fields.
 */
export function sectionFields(id: SectionId): ReadonlyArray<keyof RemoteRepoConfig> {
  switch (id) {
    case "general":
      return ["description", "homepage", "topics"];
    case "visibility":
      return ["visibility", "archived"];
    case "features":
      return ["default_branch", "issues_enabled", "wiki_enabled"];
    case "protection":
    case "labels":
      return [];
  }
}

/**
 * Empty patch skeleton — every key `undefined` except `homepage` and
 * `topics_added/removed`, which use their empty defaults.
 */
function emptyPatch(): RemoteRepoConfigPatch {
  return {
    description: undefined,
    homepage: { kind: "unchanged" },
    topics_added: [],
    topics_removed: [],
    visibility: undefined,
    default_branch: undefined,
    issues_enabled: undefined,
    wiki_enabled: undefined,
    archive: undefined,
  };
}

/**
 * Restrict the full diff of `before → current` to the fields owned by
 * `section`. Non-owned fields are returned as "unchanged".
 */
export function sectionPatchFor(
  section: SectionId,
  before: RemoteRepoConfig,
  current: RemoteRepoConfig,
): RemoteRepoConfigPatch {
  if (section === "protection" || section === "labels") return emptyPatch();

  const full = buildPatch(before, current);
  const out = emptyPatch();

  if (section === "general") {
    out.description = full.description;
    out.homepage = full.homepage;
    out.topics_added = full.topics_added;
    out.topics_removed = full.topics_removed;
  } else if (section === "visibility") {
    out.visibility = full.visibility;
    out.archive = full.archive;
  } else if (section === "features") {
    out.default_branch = full.default_branch;
    out.issues_enabled = full.issues_enabled;
    out.wiki_enabled = full.wiki_enabled;
  }

  return out;
}

/** True when the section has at least one edited field. */
export function isSectionDirty(
  section: SectionId,
  before: RemoteRepoConfig,
  current: RemoteRepoConfig,
): boolean {
  if (section === "protection" || section === "labels") return false;
  const p = sectionPatchFor(section, before, current);
  return (
    p.description !== undefined ||
    p.homepage.kind !== "unchanged" ||
    p.topics_added.length > 0 ||
    p.topics_removed.length > 0 ||
    p.visibility !== undefined ||
    p.default_branch !== undefined ||
    p.issues_enabled !== undefined ||
    p.wiki_enabled !== undefined ||
    p.archive !== undefined
  );
}

/**
 * Copy `before`'s values for the active section's fields back into
 * `draft`. Used by the Discard button and the guard's Discard option.
 */
export function resetSectionFields(
  section: SectionId,
  draft: RemoteRepoConfig,
  before: RemoteRepoConfig,
): void {
  for (const key of sectionFields(section)) {
    if (key === "topics") {
      draft.topics = [...before.topics];
    } else {
      // Runtime copy across a heterogeneous union — the compile-time
      // key list guarantees safety. `any` is confined to this line.
      (draft as unknown as Record<string, unknown>)[key] =
        (before as unknown as Record<string, unknown>)[key];
    }
  }
}
