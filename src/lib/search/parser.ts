import type { SearchTag } from "./types";

let nextId = 0;

export function parseInput(input: string, availableFilters: string[]): SearchTag[] {
  const tags: SearchTag[] = [];
  const parts = input.split(";").map(s => s.trim()).filter(Boolean);

  for (const part of parts) {
    const colonIndex = part.indexOf(":");
    if (colonIndex > 0) {
      const type = part.slice(0, colonIndex).toLowerCase().trim();
      const value = part.slice(colonIndex + 1).trim();
      if (value && availableFilters.includes(type)) {
        tags.push({
          id: `tag-${nextId++}`,
          type,
          value,
          display: `${type}:${value}`,
        });
        continue;
      }
    }
    // Plain text search
    if (part.trim()) {
      tags.push({
        id: `tag-${nextId++}`,
        type: "text",
        value: part.trim(),
        display: part.trim(),
      });
    }
  }
  return tags;
}
