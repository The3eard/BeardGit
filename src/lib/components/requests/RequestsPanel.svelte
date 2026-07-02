<!--
  Top-level shell for the Requests view.

  Two-column layout: a left tree (envs + collections) and a main area
  that splits into a request editor on top of a response viewer when a
  source is selected. When nothing is selected, `SeedPrompt` invites
  the user to create their first request (Phase 9 placeholder — the
  real workflow lands in subsequent phases).

  Borders use the canonical `--border` token, matching the rest of the
  app's panel separators (sidebar, list panels, repo-config view).

  External-edit detection: `.beardgit/requests/` is *not* tracked by
  the repo's git-mutation watcher (the `watcher` crate cares about
  `refs/`, `HEAD`, and the working tree's status, not about arbitrary
  files inside `.beardgit/`). To keep the tree fresh when the user
  deletes/renames the folder from a terminal — or any other external
  edit lands underneath — we run a 10 s `setInterval` while the panel
  is mounted that re-fetches `requests_list_project`. When the file
  count or paths change we bump `treeReloadSignal` so `CollectionsTree`
  re-renders, and if the file backing `currentSource` has disappeared
  we null the source so the editor switches back to `SeedPrompt`
  rather than displaying a stale doc. The interval used to be 2 s,
  but external edits are infrequent and the IPC noise was disruptive
  during dev — 10 s keeps the UX responsive without spamming the
  command tree.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { requestsListProject } from "$lib/api/tauri";
  import type { RequestTreeNode } from "$lib/types/requests";
  import CollectionsTree from "./CollectionsTree.svelte";
  import EnvSwitcher from "./EnvSwitcher.svelte";
  import RequestEditor from "./RequestEditor.svelte";
  import ResponseViewer from "./ResponseViewer.svelte";
  import SeedPrompt from "./SeedPrompt.svelte";
  import { currentSource, treeReloadSignal } from "./stores";
  import { activeProject } from "$lib/stores/projects";
  import { get } from "svelte/store";

  /** Tree node shape mirrored from the backend (see `$lib/types/requests`). */
  type Node = RequestTreeNode;

  /** Last-seen flat list of project file rel_paths, used to detect external edits. */
  let lastSeenPaths = "";

  /**
   * Flatten a tree into a stable comma-joined list of file `rel_path`s
   * so we can compare two snapshots with a single string equality.
   */
  function flatten(nodes: Node[]): string[] {
    const out: string[] = [];
    const walk = (ns: Node[]) => {
      for (const n of ns) {
        if (n.kind === "file") out.push(n.rel_path);
        if (n.children?.length) walk(n.children);
      }
    };
    walk(nodes);
    return out.sort();
  }

  /**
   * Poll the project requests tree once. If anything changed, bump the
   * reload signal so `CollectionsTree` re-renders. When the file backing
   * `currentSource` is gone, also null the selection so we don't keep
   * a stale request displayed in the editor.
   */
  async function pollProjectTree(): Promise<void> {
    const path = get(activeProject)?.path ?? "";
    if (!path) {
      if (lastSeenPaths !== "") {
        lastSeenPaths = "";
        treeReloadSignal.update((n) => n + 1);
      }
      return;
    }
    let project: Node[] = [];
    try {
      project = await requestsListProject(path);
    } catch {
      // If the listing fails (e.g. .beardgit/ disappeared), treat it as empty.
      project = [];
    }
    const flat = flatten(project);
    const joined = flat.join("|");
    if (joined !== lastSeenPaths) {
      lastSeenPaths = joined;
      treeReloadSignal.update((n) => n + 1);
      // If the active source is project-local and its file is gone, clear it.
      const src = get(currentSource);
      if (src?.kind === "project" && !flat.includes(src.path)) {
        currentSource.set(null);
      }
    }
  }

  let pollHandle: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    // Seed the snapshot so we don't trigger a spurious bump on the first tick.
    void pollProjectTree();
    pollHandle = setInterval(() => {
      void pollProjectTree();
    }, 10000);
  });

  onDestroy(() => {
    if (pollHandle !== null) {
      clearInterval(pollHandle);
      pollHandle = null;
    }
  });
</script>

<div class="requests-panel">
  <aside class="requests-tree">
    <EnvSwitcher />
    <CollectionsTree />
  </aside>
  <main class="requests-main">
    {#if $currentSource}
      <section class="requests-editor"><RequestEditor /></section>
      <section class="requests-response"><ResponseViewer /></section>
    {:else}
      <SeedPrompt />
    {/if}
  </main>
</div>

<style>
  .requests-panel {
    display: grid;
    grid-template-columns: 280px 1fr;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .requests-tree {
    border-right: 1px solid var(--border);
    overflow: auto;
    background: var(--bg-primary);
  }

  .requests-main {
    display: grid;
    grid-template-rows: 1fr 1fr;
    overflow: hidden;
  }

  .requests-editor,
  .requests-response {
    overflow: auto;
    border-bottom: 1px solid var(--border);
  }

  .requests-response {
    border-bottom: 0;
  }
</style>
