<script lang="ts">
  import { onMount } from "svelte";
  import { debounce } from "../../utils/debounce";
  import ContextMenu from "../common/ContextMenu.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import List from "../common/List.svelte";
  import BranchTreeNode from "./BranchTreeNode.svelte";
  import RenameBranchDialog from "./RenameBranchDialog.svelte";
  import { IconButton } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import type { BranchTreeNode as TreeNode } from "./branch-tree";
  import type { InitialSource } from "./suggest-local-name";
  import { parseRemoteBranch } from "./parse-remote-branch";
  import {
    branches,
    branchesLoading,
    selectedBranchName,
    localBranches,
    remoteBranches,
    selectBranch,
    refreshBranches,
    doCheckout,
    doDeleteBranch,
    doMergeBranch,
  } from "../../stores/branches";
  import { remotes, refreshRemotes } from "../../stores/remotes";
  import { openCreateBranchDialog } from "../../stores/createBranchDialog";
  import { rebaseBranch, pushRemote, deleteRemoteBranch } from "../../api/tauri";
  import { runMutation } from "../../api/runMutation";
  import type { BranchInfo } from "../../types";

  /**
   * Build a folder-tree from a flat branch list.
   * Branches with "/" in their name are nested under folder nodes.
   */
  function buildTree(branchList: BranchInfo[]): TreeNode[] {
    const root: TreeNode[] = [];
    const childMaps = new WeakMap<TreeNode[], Map<string, TreeNode>>();

    function getMap(children: TreeNode[]): Map<string, TreeNode> {
      let map = childMaps.get(children);
      if (!map) {
        map = new Map();
        childMaps.set(children, map);
      }
      return map;
    }

    for (const branch of branchList) {
      const parts = branch.name.split("/");
      let current = root;

      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const isLeaf = i === parts.length - 1;
        const key = `${part}:${isLeaf ? "leaf" : "folder"}`;
        const map = getMap(current);

        let existing = map.get(key);
        if (!existing) {
          existing = {
            name: part,
            fullPath: isLeaf ? branch.name : parts.slice(0, i + 1).join("/"),
            isFolder: !isLeaf,
            isHead: isLeaf && branch.is_head,
            isRemote: branch.is_remote,
            oid: isLeaf ? branch.oid : "",
            ahead: isLeaf ? branch.ahead : 0,
            behind: isLeaf ? branch.behind : 0,
            children: [],
          };
          current.push(existing);
          map.set(key, existing);
        }
        if (!isLeaf) {
          current = existing.children;
        }
      }
    }
    return root;
  }

  let filterInput = $state("");
  let filterValue = $state("");

  const applyFilter = debounce((value: string) => {
    filterValue = value;
  }, 150);

  function onFilterInput(value: string) {
    filterInput = value;
    applyFilter(value);
  }

  // Hoist the lowercased filter once per keystroke so the per-branch
  // `.includes()` doesn't re-lowercase the needle for every haystack
  // entry — at 5k branches that's a measurable win over the debounce
  // window and lets `$derived` short-circuit quickly when the filter
  // is empty (returns the same array reference, so downstream
  // `buildTree` derivations don't re-run on no-op updates).
  let filteredLocal = $derived.by(() => {
    if (!filterValue) return $localBranches;
    const needle = filterValue.toLowerCase();
    return $localBranches.filter((b) => b.name.toLowerCase().includes(needle));
  });

  let filteredRemote = $derived.by(() => {
    if (!filterValue) return $remoteBranches;
    const needle = filterValue.toLowerCase();
    return $remoteBranches.filter((b) => b.name.toLowerCase().includes(needle));
  });

  let localTree = $derived(buildTree(filteredLocal));
  let remoteTree = $derived(buildTree(filteredRemote));

  let localCollapsed = $state(false);
  let remoteCollapsed = $state(false);

  // Context menu state
  let menuVisible = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let contextBranch = $state("");
  let contextOid = $state("");
  let contextIsRemote = $state(false);
  let confirmDelete = $state<string | null>(null);
  let forceDelete = $state(false);
  let confirmRebase = $state<string | null>(null);
  let confirmForcePush = $state<{ remote: string; branch: string } | null>(null);
  /**
   * Remote-branch deletion confirmation. The two fields hold the parsed
   * `<remote>/<branch>` split so the modal can show a clear "delete X
   * on Y" message and the IPC call passes the right pieces. `null`
   * means the dialog is closed.
   */
  let confirmDeleteRemote = $state<{
    remote: string;
    branch: string;
    fullPath: string;
  } | null>(null);

  // Rename dialog state — mounted locally so it has access to branch context
  let renameDialogOpen = $state(false);
  let renameTarget = $state("");

  function openRenameDialog(name: string) {
    renameTarget = name;
    renameDialogOpen = true;
  }

  /**
   * Trigger the remote branch deletion task. The task streams output
   * through the standard task drawer and the watcher picks up the
   * pruned `refs/remotes/<remote>/<branch>` to refresh the list.
   */
  async function doDeleteRemoteBranch(remote: string, branch: string) {
    try {
      await runMutation({
        kind: "remote_branch_delete",
        invoke: () => deleteRemoteBranch(remote, branch),
        successToast: () => `Deleted ${remote}/${branch}`,
        failureToastPrefix: "Remote branch delete failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    }
  }

  /**
   * Push `branch` to `remote`. When `force` is true the operation is
   * guarded by `--force-with-lease` on the Rust side.
   */
  async function doPush(remote: string, branch: string, force: boolean) {
    try {
      await runMutation({
        kind: force ? "push_force" : "push",
        invoke: () => pushRemote(remote, branch, force),
        successToast: () => `Pushed ${remote}/${branch}`,
        failureToastPrefix: force ? "Force-push failed" : "Push failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    }
  }

  /**
   * Build the "Push" context-menu item.
   * Single remote → fires directly. Multiple remotes → submenu.
   */
  function pushMenuItem(): MenuItem {
    const rs = $remotes;
    if (rs.length === 1) {
      const r = rs[0].name;
      return { label: `Push → ${r}`, action: () => doPush(r, contextBranch, false) };
    }
    return {
      label: "Push",
      children: rs.map((r) => ({
        label: r.name,
        action: () => doPush(r.name, contextBranch, false),
      })),
    };
  }

  /**
   * Build the "Push (force-with-lease)" context-menu item.
   * Always a submenu so force-push never happens on a single click.
   */
  function forcePushMenuItem(): MenuItem {
    return {
      label: "Push (force-with-lease)",
      children: $remotes.map((r) => ({
        label: r.name,
        action: () => {
          confirmForcePush = { remote: r.name, branch: contextBranch };
        },
      })),
    };
  }

  let menuItems: MenuItem[] = $derived.by(() => {
    const items: MenuItem[] = [];
    if (!contextIsRemote) {
      items.push({ label: "Checkout", action: () => doCheckout(contextBranch) });
    }
    items.push({
      label: "New branch from here",
      action: () =>
        openCreateBranchDialog({ kind: "ref", name: contextBranch, oid: contextOid }),
    });
    if (!contextIsRemote) {
      items.push({ label: "Rename", action: () => openRenameDialog(contextBranch) });
    }
    items.push({ label: "Merge into current", action: () => doMergeBranch(contextBranch) });
    items.push({
      label: m.branch_rebase_onto(),
      action: () => {
        confirmRebase = contextBranch;
      },
    });
    if (!contextIsRemote) {
      items.push({
        label: "Delete",
        action: () => {
          forceDelete = false;
          confirmDelete = contextBranch;
        },
      });
    }
    if (contextIsRemote) {
      const parsed = parseRemoteBranch(contextBranch);
      if (parsed) {
        items.push({
          label: "Delete on remote",
          action: () => {
            confirmDeleteRemote = { ...parsed, fullPath: contextBranch };
          },
        });
      }
    }
    if (!contextIsRemote && $remotes.length > 0) {
      items.push({ separator: true });
      items.push(pushMenuItem());
      items.push(forcePushMenuItem());
    }
    return items;
  });

  function handleContextMenu(e: MouseEvent, node: TreeNode) {
    if (node.isFolder) return;
    e.preventDefault();
    contextBranch = node.fullPath;
    contextOid = node.oid;
    contextIsRemote = node.isRemote;
    menuX = e.clientX;
    menuY = e.clientY;
    menuVisible = true;
  }

  function handleRefresh() {
    filterInput = "";
    filterValue = "";
    refreshBranches();
  }

  // Seed the remotes store on mount so the first right-click has data
  // without waiting for a project-mutated event.
  onMount(() => {
    void refreshRemotes();
  });

  // Required by List type signature but unused — trees are rendered via customContent.
  function getKey(_item: BranchInfo): string {
    return "";
  }
</script>

<div class="branch-list" data-testid="branch-list">
<List
  items={[] as BranchInfo[]}
  loading={$branchesLoading}
  title="BRANCHES"
  selectedKey={$selectedBranchName}
  {getKey}
>
  {#snippet headerActions()}
    <IconButton
      icon={"\uF067"}
      description={m.tooltip_new_branch()}
      testid="branch-new-btn"
      onclick={() => openCreateBranchDialog({ kind: "head" })}
    />
    <IconButton
      icon={"\uF021"}
      description={m.tooltip_refresh()}
      loading={$branchesLoading}
      onclick={handleRefresh}
    />
  {/snippet}

  {#snippet afterHeader()}
    <div class="filter-row">
      <input
        type="text"
        class="filter-input"
        placeholder="Filter branches…"
        value={filterInput}
        oninput={(e) => onFilterInput(e.currentTarget.value)}
        data-testid="branch-filter"
      />
    </div>
  {/snippet}

  {#snippet customContent()}
    <!-- LOCAL section -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="section-header"
      role="button"
      tabindex="0"
      onclick={() => (localCollapsed = !localCollapsed)}
      onkeydown={(e) => {
        if (e.key === "Enter" || e.key === " ") localCollapsed = !localCollapsed;
      }}
    >
      <span class="section-chevron nf" class:collapsed={localCollapsed}>{""}</span>
      <span class="section-label">LOCAL</span>
      <span class="section-count">{$localBranches.length}</span>
    </div>

    {#if !localCollapsed}
      {#if $branchesLoading && $branches.length === 0}
        <div class="list-loading">
          <div class="spinner"></div>
        </div>
      {:else if localTree.length === 0}
        <EmptyState title={m.branches_no_local()} />
      {:else}
        {#each localTree as node (node.fullPath)}
          <BranchTreeNode
            {node}
            depth={0}
            selected={$selectedBranchName}
            onSelect={selectBranch}
            onContext={handleContextMenu}
          />
        {/each}
      {/if}
    {/if}

    <!-- REMOTE section -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="section-header"
      role="button"
      tabindex="0"
      onclick={() => (remoteCollapsed = !remoteCollapsed)}
      onkeydown={(e) => {
        if (e.key === "Enter" || e.key === " ") remoteCollapsed = !remoteCollapsed;
      }}
    >
      <span class="section-chevron nf" class:collapsed={remoteCollapsed}>{""}</span>
      <span class="section-label">REMOTE</span>
      <span class="section-count">{$remoteBranches.length}</span>
    </div>

    {#if !remoteCollapsed}
      {#if remoteTree.length === 0}
        <EmptyState title={m.branches_no_remote()} />
      {:else}
        {#each remoteTree as node (node.fullPath)}
          <BranchTreeNode
            {node}
            depth={0}
            selected={$selectedBranchName}
            onSelect={selectBranch}
            onContext={handleContextMenu}
          />
        {/each}
      {/if}
    {/if}
  {/snippet}
</List>
</div>

<ContextMenu
  items={menuItems}
  x={menuX}
  y={menuY}
  visible={menuVisible}
  onClose={() => (menuVisible = false)}
/>

<RenameBranchDialog
  open={renameDialogOpen}
  currentName={renameTarget}
  onClose={() => (renameDialogOpen = false)}
/>

{#if confirmDelete !== null}
  <ConfirmDialog
    title="Delete Branch"
    detail={confirmDelete}
    message={`Are you sure you want to delete branch "${confirmDelete}"? This action cannot be undone.`}
    confirmLabel={forceDelete ? "Force Delete" : "Delete"}
    destructive={true}
    checkboxLabel="Force delete (allow unmerged commits)"
    bind:checkboxChecked={forceDelete}
    onConfirm={async () => {
      const target = confirmDelete!;
      const force = forceDelete;
      // Close the dialog before the IPC fires so success/failure
      // toasts sit against the underlying list rather than on top.
      confirmDelete = null;
      try {
        await doDeleteBranch(target, force);
      } catch {
        // `runMutation` already surfaced a sticky failure toast.
      }
    }}
    onCancel={() => (confirmDelete = null)}
  />
{/if}

{#if confirmRebase !== null}
  <ConfirmDialog
    title={m.branch_rebase_onto()}
    detail={confirmRebase}
    message={m.branch_rebase_confirm({ branch: confirmRebase })}
    confirmLabel={m.branch_rebase_onto()}
    destructive={false}
    onConfirm={async () => {
      const target = confirmRebase!;
      try {
        await runMutation({
          kind: "rebase",
          invoke: () => rebaseBranch(target),
          successToast: () => `Rebased onto ${target}`,
          failureToastPrefix: "Rebase failed",
          trackAsTask: true,
        });
      } catch {
        // runMutation already surfaced the toast.
      }
      confirmRebase = null;
    }}
    onCancel={() => (confirmRebase = null)}
  />
{/if}

{#if confirmForcePush !== null}
  <ConfirmDialog
    title="Force-push with lease"
    detail={`${confirmForcePush.remote}/${confirmForcePush.branch}`}
    message={`Force-push ${confirmForcePush.branch} to ${confirmForcePush.remote} (with --force-with-lease)? This rewrites history on the remote.`}
    confirmLabel="Force-push"
    destructive={true}
    onConfirm={async () => {
      const { remote, branch } = confirmForcePush!;
      confirmForcePush = null;
      await doPush(remote, branch, true);
    }}
    onCancel={() => (confirmForcePush = null)}
  />
{/if}

{#if confirmDeleteRemote !== null}
  <ConfirmDialog
    title="Delete branch on remote"
    detail={confirmDeleteRemote.fullPath}
    message={`This will delete branch "${confirmDeleteRemote.branch}" on remote "${confirmDeleteRemote.remote}". The branch is removed for everyone using this remote and cannot be undone from BeardGit.`}
    confirmLabel="Delete on remote"
    destructive={true}
    onConfirm={async () => {
      const { remote, branch } = confirmDeleteRemote!;
      confirmDeleteRemote = null;
      await doDeleteRemoteBranch(remote, branch);
    }}
    onCancel={() => (confirmDeleteRemote = null)}
  />
{/if}

<style>
  .branch-list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
  }

  .filter-row {
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
  }

  .filter-input {
    width: 100%;
    padding: 4px 8px;
    font-size: 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    box-sizing: border-box;
  }

  .filter-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    cursor: pointer;
    user-select: none;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .section-header:hover {
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
  }

  .section-chevron {
    font-size: 9px;
    color: var(--text-secondary);
    transition: transform 0.15s;
    display: inline-block;
  }

  .section-chevron.collapsed {
    transform: rotate(0deg);
  }

  .section-chevron:not(.collapsed) {
    transform: rotate(90deg);
  }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    flex: 1;
  }

  .section-count {
    font-size: 10px;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    padding: 1px 6px;
    border-radius: 10px;
  }

  .list-loading {
    display: flex;
    justify-content: center;
    padding: 16px;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

</style>
