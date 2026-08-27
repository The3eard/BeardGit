/**
 * The AI Config tree has to make one CLAUDE.md distinguishable from another.
 *
 * It did not. `relativePath` fell back to the bare filename for anything
 * outside a `.claude/` directory, so every project CLAUDE.md rendered as an
 * identical row labelled "CLAUDE.md" — twelve of them in this repo, with
 * nothing to tell `src/lib/stores/` from `crates/git-engine/`, and no
 * grouping either, because `buildTree` splits on `/` and a bare filename has
 * no segments to nest under.
 *
 * That only became visible once discovery started finding nested files; the
 * fallback was written when the panel listed a root CLAUDE.md and nothing
 * else.
 */
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { AiConfigFile } from "$lib/types";

import AiConfigFileTree from "../AiConfigFileTree.svelte";
import { configFiles } from "$lib/stores/aiConfig";
import { repoInfo } from "$lib/stores/repo";

const ROOT = "/repos/demo";

function md(relative: string): AiConfigFile {
  return {
    path: `${ROOT}/${relative}`,
    kind: "instructions",
    scope: "project",
  } as AiConfigFile;
}

afterEach(() => {
  cleanup();
  configFiles.set([]);
  repoInfo.set(null);
});

async function mount(files: AiConfigFile[]) {
  repoInfo.set({
    path: ROOT,
    head_branch: "main",
    head_oid: null,
    branch_count: 1,
  } as never);
  configFiles.set(files);
  const r = render(AiConfigFileTree, {
    props: { onSelectFile: () => {}, onCreateFile: () => {} },
  });
  await tick();
  return r;
}

describe("AiConfigFileTree — telling one CLAUDE.md from another", () => {
  it("nests project CLAUDE.md files under their directories", async () => {
    const { container } = await mount([
      md("CLAUDE.md"),
      md("crates/git-engine/CLAUDE.md"),
      md("src/lib/stores/CLAUDE.md"),
    ]);

    const folders = [...container.querySelectorAll(".folder-name")].map(
      (n) => n.textContent?.trim(),
    );
    // The directory names are the only thing that distinguishes these rows,
    // so they have to reach the DOM.
    expect(folders).toContain("crates");
    expect(folders).toContain("git-engine");
    expect(folders).toContain("src");
    expect(folders).toContain("stores");
  });

  it("keeps each file addressable by its own absolute path", async () => {
    const { container } = await mount([
      md("crates/git-engine/CLAUDE.md"),
      md("src/lib/stores/CLAUDE.md"),
    ]);

    // `title` carries the full path — the tooltip a user checks when two
    // rows look alike — and it must differ per row.
    const titles = [...container.querySelectorAll(".tree-leaf")].map((n) =>
      n.getAttribute("title"),
    );
    expect(titles).toHaveLength(2);
    expect(new Set(titles).size).toBe(2);
    expect(titles).toContain(`${ROOT}/crates/git-engine/CLAUDE.md`);
    expect(titles).toContain(`${ROOT}/src/lib/stores/CLAUDE.md`);
  });

  it("still strips the .claude/ prefix for files inside it", async () => {
    const { container } = await mount([
      {
        path: `${ROOT}/.claude/agents/reviewer.md`,
        kind: "agent",
        scope: "project",
      } as AiConfigFile,
    ]);

    const folders = [...container.querySelectorAll(".folder-name")].map(
      (n) => n.textContent?.trim(),
    );
    // Grouped under `agents`, not under `.claude` — the scope header already
    // says whose `.claude` this is.
    expect(folders).toContain("agents");
    expect(folders).not.toContain(".claude");
  });

  it("falls back to the filename for a path outside the repo", async () => {
    repoInfo.set(null);
    configFiles.set([
      {
        path: "/somewhere/else/CLAUDE.md",
        kind: "instructions",
        scope: "project",
      } as AiConfigFile,
    ]);
    const { container } = render(AiConfigFileTree, {
      props: { onSelectFile: () => {}, onCreateFile: () => {} },
    });
    await tick();

    const names = [...container.querySelectorAll(".file-name")].map((n) =>
      n.textContent?.trim(),
    );
    expect(names).toEqual(["CLAUDE.md"]);
  });
});
