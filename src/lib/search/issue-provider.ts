/**
 * Search / filter provider for issues.
 *
 * Mirrors `mr-provider.ts`: declares the filter tag types understood by the
 * issue search bar and exposes a pure client-side filter that narrows a
 * list of issues by the active tags.
 */

import type { SearchTag, FilterDef } from "./types";
import type { Issue } from "../types";

export const issueFilters: FilterDef[] = [
  { type: "state", label: "State", placeholder: "open|closed" },
  { type: "author", label: "Author", placeholder: "username" },
  { type: "assignee", label: "Assignee", placeholder: "username" },
  { type: "label", label: "Label", placeholder: "label-name" },
  { type: "milestone", label: "Milestone", placeholder: "milestone-title" },
];

/**
 * Filter an issue list locally by the given search tags.
 *
 * Tags are AND-composed. Empty tag list returns the original list unchanged.
 */
export function filterIssuesLocal(items: Issue[], tags: SearchTag[]): Issue[] {
  if (tags.length === 0) return items;
  return items.filter((item) =>
    tags.every((tag) => {
      const q = tag.value.toLowerCase();
      switch (tag.type) {
        case "author":
          return (item.author ?? "").toLowerCase().includes(q);
        case "assignee":
          return item.assignees.some((a) => a.toLowerCase().includes(q));
        case "label":
          return item.labels.some((l) => l.name.toLowerCase().includes(q));
        case "milestone":
          return (item.milestone?.title ?? "").toLowerCase().includes(q);
        case "state":
          return item.state === q;
        case "text":
          return (
            (item.title ?? "").toLowerCase().includes(q) ||
            (item.author ?? "").toLowerCase().includes(q) ||
            String(item.number).includes(tag.value)
          );
        default:
          return true;
      }
    }),
  );
}
