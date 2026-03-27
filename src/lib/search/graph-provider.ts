import type { SearchTag, FilterDef } from "./types";
import type { LayoutNode, GraphViewport } from "../types";
import { searchCommits } from "../api/tauri";

export const graphFilters: FilterDef[] = [
  { type: "user", label: "Author", placeholder: "author-name" },
  { type: "branch", label: "Branch", placeholder: "branch-name" },
  { type: "commit", label: "Commit message", placeholder: "message-text" },
  { type: "sha", label: "SHA", placeholder: "sha-prefix" },
];

export function filterGraphLocal(nodes: LayoutNode[], tags: SearchTag[]): LayoutNode[] {
  if (tags.length === 0) return nodes;

  return nodes.filter(node => {
    return tags.every(tag => {
      switch (tag.type) {
        case "user":
          return node.author.toLowerCase().includes(tag.value.toLowerCase());
        case "branch":
          return node.refs.some(r => r.toLowerCase().includes(tag.value.toLowerCase()));
        case "commit":
          return node.summary.toLowerCase().includes(tag.value.toLowerCase());
        case "sha":
          return node.oid.toLowerCase().startsWith(tag.value.toLowerCase());
        case "text":
          return node.summary.toLowerCase().includes(tag.value.toLowerCase()) ||
                 node.author.toLowerCase().includes(tag.value.toLowerCase()) ||
                 node.oid.toLowerCase().startsWith(tag.value.toLowerCase());
        default:
          return true;
      }
    });
  });
}

export async function filterGraphRemote(tags: SearchTag[]): Promise<GraphViewport | null> {
  if (tags.length === 0) return null;

  let branch: string | undefined;
  let author: string | undefined;
  let message: string | undefined;
  let sha: string | undefined;

  for (const tag of tags) {
    switch (tag.type) {
      case "branch": branch = tag.value; break;
      case "user": author = tag.value; break;
      case "commit": message = tag.value; break;
      case "sha": sha = tag.value; break;
      case "text":
        // Plain text -- search in message
        message = tag.value;
        break;
    }
  }

  return searchCommits(branch, author, message, sha, 500);
}
