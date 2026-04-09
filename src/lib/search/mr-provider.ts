import type { SearchTag, FilterDef } from "./types";
import type { MrPr } from "../types";

export const mrFilters: FilterDef[] = [
  { type: "state", label: "State", placeholder: "open|closed|merged" },
  { type: "author", label: "Author", placeholder: "username" },
  { type: "branch", label: "Branch", placeholder: "branch-name" },
  { type: "label", label: "Label", placeholder: "label-name" },
];

export function filterMrPrLocal(items: MrPr[], tags: SearchTag[]): MrPr[] {
  if (tags.length === 0) return items;

  return items.filter(item => {
    return tags.every(tag => {
      const q = tag.value.toLowerCase();
      switch (tag.type) {
        case "author":
          return (item.author ?? "").toLowerCase().includes(q);
        case "branch":
          return (item.source_branch ?? "").toLowerCase().includes(q) ||
                 (item.target_branch ?? "").toLowerCase().includes(q);
        case "label":
          return item.labels.some(l => l.toLowerCase().includes(q));
        case "text": {
          return (item.title ?? "").toLowerCase().includes(q) ||
                 (item.author ?? "").toLowerCase().includes(q) ||
                 (item.source_branch ?? "").toLowerCase().includes(q) ||
                 String(item.number).includes(tag.value);
        }
        default:
          return true;
      }
    });
  });
}
