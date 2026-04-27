import type { SearchTag, FilterDef } from "./types";
import type { CiRun } from "../types";
import * as api from "../api/tauri";

export const ciFilters: FilterDef[] = [
  { type: "branch", label: "Branch", placeholder: "branch-name" },
  { type: "status", label: "Status", placeholder: "success|failed|running" },
  { type: "source", label: "Source", placeholder: "push|pull_request|schedule" },
];

export function filterCiRunsLocal(items: CiRun[], tags: SearchTag[]): CiRun[] {
  if (tags.length === 0) return items;

  return items.filter(r => {
    return tags.every(tag => {
      const ref = (r.ref_name ?? "").toLowerCase();
      const status = (r.status ?? "").toLowerCase();
      const source = (r.source ?? "").toLowerCase();

      switch (tag.type) {
        case "branch":
          return ref.includes(tag.value.toLowerCase());
        case "status":
          return status === tag.value.toLowerCase();
        case "source":
          return source.includes(tag.value.toLowerCase());
        case "text": {
          const q = tag.value.toLowerCase();
          return ref.includes(q) || status.includes(q) || String(r.display_id).includes(tag.value);
        }
        default:
          return true;
      }
    });
  });
}

export async function filterCiRunsRemote(tags: SearchTag[]): Promise<CiRun[]> {
  let branch: string | undefined;
  let source: string | undefined;
  let status: string | undefined;

  for (const tag of tags) {
    if (tag.type === "branch") branch = tag.value;
    if (tag.type === "source") source = tag.value;
    if (tag.type === "status") status = tag.value.toLowerCase();
  }

  let results = await api.listCiRuns(branch, source, status, 30);

  if (results.length === 0 && branch) {
    const all = await api.listCiRuns(undefined, source, status, 50);
    results = all.filter(r => (r.ref_name ?? "").toLowerCase().includes(branch!.toLowerCase()));
  }

  return results;
}
