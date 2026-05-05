<!--
  FileEditorPanel.svelte — shell for the in-app mini-editor view.

  Two-pane layout: file tree on the left, tabs / toolbar / editor on the
  right. Owns the dialog state for new-file / new-folder / rename /
  delete flows so the tree stays presentational.

  Lifecycle:
   - On mount, refreshes the workdir tree and restores any tabs
     persisted from the previous session for the active project.
   - Persists tabs to localStorage on project switch (the parent route
     drives this via `onProjectSwitch`); the panel itself only handles
     restore-on-mount so a cold open of the editor view on the same
     project re-hydrates the same tabs.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import ConfirmDialog from "$lib/components/common/ConfirmDialog.svelte";
  import SplitView from "$lib/components/common/SplitView.svelte";
  import { editorPrefs } from "$lib/stores/editorPrefs";
  import {
    createPath,
    deletePath,
    persistTabsForProject,
    refreshTree,
    renamePath,
    restoreTabsForProject,
  } from "$lib/stores/fileEditor";
  import { activeProject } from "$lib/stores/projects";
  import type { WorkdirTreeEntry } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import EditorPane from "./EditorPane.svelte";
  import EditorTabs from "./EditorTabs.svelte";
  import EditorToolbar from "./EditorToolbar.svelte";
  import FileTreeView from "./FileTreeView.svelte";
  import PathDialog from "./PathDialog.svelte";

  /** Whether the workdir tree should hide gitignored entries. */
  let respectGitignore = $derived(
    $editorPrefs?.respect_gitignore_in_tree ?? false,
  );

  // Dialog state.
  let newFileOpen = $state(false);
  let newFolderOpen = $state(false);
  let renameOpen = $state(false);
  let dialogParent = $state("");
  let renameTarget = $state<WorkdirTreeEntry | null>(null);

  let deleteTarget = $state<WorkdirTreeEntry | null>(null);

  /** Current project path — persistence + refresh trigger. */
  let projectPath = $derived($activeProject?.path ?? null);

  // Re-load tree + tabs whenever the active project changes.
  let lastLoadedProject: string | null = null;
  $effect(() => {
    const path = projectPath;
    if (path && path !== lastLoadedProject) {
      lastLoadedProject = path;
      void refreshTree(respectGitignore);
      void restoreTabsForProject(path);
    }
  });

  // Re-pull the tree when the gitignore preference flips.
  $effect(() => {
    if (projectPath) {
      void refreshTree(respectGitignore);
    }
  });

  onMount(() => {
    return () => {
      // Persist on teardown so navigating away from the editor view
      // captures the latest tab set even when the user doesn't switch
      // project tabs.
      if (projectPath) persistTabsForProject(projectPath);
    };
  });

  function openNewFile(parentDir: string) {
    dialogParent = parentDir;
    newFileOpen = true;
  }
  function openNewFolder(parentDir: string) {
    dialogParent = parentDir;
    newFolderOpen = true;
  }
  function openRename(entry: WorkdirTreeEntry) {
    renameTarget = entry;
    renameOpen = true;
  }
  function openDelete(entry: WorkdirTreeEntry) {
    deleteTarget = entry;
  }

  /** Build a repo-relative path from a parent directory and a leaf name. */
  function joinPath(parent: string, leaf: string): string {
    const trimmed = leaf.trim();
    if (parent === "") return trimmed;
    return `${parent}/${trimmed}`;
  }

  /** Rebuild a path with the renamed leaf, keeping the original parent. */
  function siblingPath(currentPath: string, newLeaf: string): string {
    const idx = currentPath.lastIndexOf("/");
    if (idx < 0) return newLeaf;
    return `${currentPath.slice(0, idx + 1)}${newLeaf}`;
  }

  async function confirmNewFile(name: string) {
    const target = joinPath(dialogParent, name);
    try {
      await createPath(target, false, respectGitignore);
      newFileOpen = false;
    } catch {
      // runMutation already surfaced the toast; keep the dialog open
      // so the user can edit and retry.
    }
  }
  async function confirmNewFolder(name: string) {
    const target = joinPath(dialogParent, name);
    try {
      await createPath(target, true, respectGitignore);
      newFolderOpen = false;
    } catch {
      // runMutation already surfaced the toast.
    }
  }
  async function confirmRename(name: string) {
    if (!renameTarget) return;
    const target = siblingPath(renameTarget.path, name);
    try {
      await renamePath(renameTarget.path, target, respectGitignore);
      renameOpen = false;
      renameTarget = null;
    } catch {
      // runMutation already surfaced the toast.
    }
  }
  async function confirmDelete() {
    if (!deleteTarget) return;
    const target = deleteTarget;
    try {
      await deletePath(target.path, respectGitignore);
    } catch {
      // runMutation already surfaced the toast.
    }
    deleteTarget = null;
  }
</script>

{#if !projectPath}
  <div class="empty">
    <p>{m.editor_no_project_open()}</p>
  </div>
{:else}
  <div class="file-editor">
    <SplitView refreshFn={() => {}} defaultWidth={280}>
      {#snippet left()}
        <FileTreeView
          {respectGitignore}
          onNewFile={openNewFile}
          onNewFolder={openNewFolder}
          onRename={openRename}
          onDelete={openDelete}
        />
      {/snippet}
      {#snippet right()}
        <div class="right-pane">
          <EditorTabs />
          <EditorToolbar />
          <EditorPane />
        </div>
      {/snippet}
    </SplitView>
  </div>
{/if}

<PathDialog
  open={newFileOpen}
  mode="new-file"
  parentDir={dialogParent}
  onConfirm={confirmNewFile}
  onClose={() => (newFileOpen = false)}
/>

<PathDialog
  open={newFolderOpen}
  mode="new-folder"
  parentDir={dialogParent}
  onConfirm={confirmNewFolder}
  onClose={() => (newFolderOpen = false)}
/>

<PathDialog
  open={renameOpen}
  mode="rename"
  targetPath={renameTarget?.path ?? ""}
  onConfirm={confirmRename}
  onClose={() => {
    renameOpen = false;
    renameTarget = null;
  }}
/>

{#if deleteTarget}
  <ConfirmDialog
    title={m.editor_delete_confirm_title({ name: deleteTarget.path })}
    message={m.editor_delete_confirm_body()}
    destructive
    onConfirm={confirmDelete}
    onCancel={() => (deleteTarget = null)}
  />
{/if}

<style>
  .file-editor {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  .right-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }
  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 13px;
    padding: 24px;
  }
</style>
