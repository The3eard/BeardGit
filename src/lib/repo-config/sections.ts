/**
 * Provider-driven catalogue of sections rendered by `RepoConfigPage`.
 *
 * One file, one function. Future provider-specific sections (e.g.
 * GitHub Actions permissions, GitLab CI variables) branch off of
 * `kind` inside `sectionsForProvider` — consumers never know about
 * the fork.
 */

import * as m from "$lib/paraglide/messages";

/** Canonical section ids — also the keys used by the hash route. */
export const SECTION_IDS = [
  "general",
  "visibility",
  "features",
  "protection",
  "labels",
] as const;

export type SectionId = (typeof SECTION_IDS)[number];

export interface Section {
  id: SectionId;
  label: string;
  icon: string;
}

/**
 * Sections supported for `kind`. MVP returns the same five for both
 * providers; divergence goes here.
 */
export function sectionsForProvider(kind: "github" | "gitlab"): Section[] {
  // kind is accepted for forward-compatibility; list is identical today.
  void kind;
  return [
    { id: "general",    label: m.repo_config_general(),    icon: "" }, // nf-fa-cog
    { id: "visibility", label: m.repo_config_visibility(), icon: "" }, // nf-fa-eye
    { id: "features",   label: m.repo_config_features(),   icon: "" }, // nf-fa-plug
    { id: "protection", label: m.repo_config_protection(), icon: "" }, // nf-fa-lock
    { id: "labels",     label: m.repo_config_labels(),     icon: "" }, // nf-fa-tag
  ];
}

/** Type guard — turns an arbitrary string into a `SectionId` or null. */
export function toSectionId(raw: string | null | undefined): SectionId | null {
  if (!raw) return null;
  return (SECTION_IDS as readonly string[]).includes(raw)
    ? (raw as SectionId)
    : null;
}
