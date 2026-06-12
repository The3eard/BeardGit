<script lang="ts">
  import type { FileStatus } from "../../types";
  import * as m from "$lib/paraglide/messages";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import { openBlame, blameActiveTab } from "$lib/stores/blame";
  import { cleanPaths, discardFiles } from "$lib/api/tauri";
  import { addGitignorePattern } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import { Button, Checkbox } from "$lib/components/ui";
  import { activeViewStore } from "$lib/stores/navigation";
  import { openTab as openEditorTab } from "$lib/stores/fileEditor";

  let {
    files,
    title,
    onStage,
    onUnstage,
    isStaged = false,
    selectedPath = null,
    onFileClick,
    onNavigate,
  }: {
    files: FileStatus[];
    title: string;
    onStage?: (paths: string[]) => void;
    onUnstage?: (paths: string[]) => void;
    isStaged?: boolean;
    /** Path whose diff is open in the panel — its row renders highlighted. */
    selectedPath?: string | null;
    onFileClick?: (path: string) => void;
    onNavigate?: (view: string) => void;
  } = $props();

  let contextMenuVisible = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let contextMenuFile = $state<string | null>(null);
  let showDeleteConfirm = $state(false);
  let deleteTargetPath = $state<string | null>(null);
  let showDiscardConfirm = $state(false);
  let discardTargetPath = $state<string | null>(null);
  let discardTargetIsUntracked = $state(false);

  let selected = $state(new Set<string>());

  let selectedCount = $derived(selected.size);
  let allSelected = $derived(files.length > 0 && selected.size === files.length);
  let someSelected = $derived(selected.size > 0 && selected.size < files.length);

  function toggleFile(path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    selected = next;
  }

  function toggleAll() {
    if (allSelected) {
      selected = new Set();
    } else {
      selected = new Set(files.map(f => f.path));
    }
  }

  function stageSelected() {
    const paths = [...selected];
    selected = new Set();
    onStage?.(paths);
  }

  function unstageSelected() {
    const paths = [...selected];
    selected = new Set();
    onUnstage?.(paths);
  }

  // Clear selection when file list changes (after stage/unstage refresh)
  $effect(() => {
    files;
    selected = new Set();
  });

  function statusIcon(status: string): string {
    switch (status) {
      case "new": return "+";
      case "modified": return "~";
      case "deleted": return "-";
      case "renamed": return "R";
      default: return "?";
    }
  }

  /** Generate smart gitignore pattern suggestions from a file path. */
  function buildGitignorePatterns(filePath: string): { label: string; pattern: string }[] {
    const patterns: { label: string; pattern: string }[] = [];
    const parts = filePath.split("/");
    const filename = parts[parts.length - 1];
    const extIdx = filename.lastIndexOf(".");
    const ext = extIdx > 0 ? filename.substring(extIdx + 1) : null;

    // 1. Ignore by filename (anywhere in repo)
    patterns.push({
      label: m.gitignore_menu_filename({ name: filename }),
      pattern: filename,
    });

    // 2. Ignore by extension
    if (ext) {
      patterns.push({
        label: m.gitignore_menu_extension({ ext }),
        pattern: `*.${ext}`,
      });
    }

    // 3. Ignore exact path
    if (parts.length > 1) {
      patterns.push({
        label: m.gitignore_menu_path(),
        pattern: filePath,
      });
    }

    // 4. Ignore parent directory (if file is nested)
    if (parts.length > 1) {
      const dir = parts[0];
      patterns.push({
        label: m.gitignore_menu_directory({ dir }),
        pattern: `${dir}/`,
      });
    }

    return patterns;
  }

  function buildContextMenuItems(filePath: string): MenuItem[] {
    const items: MenuItem[] = [];

    if (!isStaged && onStage) {
      items.push({
        label: m.changes_menu_stage(),
        action: () => onStage!([filePath]),
      });
    }

    if (isStaged && onUnstage) {
      items.push({
        label: m.changes_menu_unstage(),
        action: () => onUnstage!([filePath]),
      });
    }

    items.push({
      label: m.changes_menu_copy_path(),
      action: () => navigator.clipboard.writeText(filePath),
    });

    items.push({
      label: m.editor_open_in_editor(),
      action: () => {
        activeViewStore.set("editor");
        void openEditorTab(filePath);
      },
    });

    items.push({ separator: true });
    items.push({
      label: m.context_blame(),
      action: () => {
        openBlame(filePath);
        onNavigate?.('blame');
      },
    });
    items.push({
      label: m.context_file_history(),
      action: () => {
        openBlame(filePath);
        blameActiveTab.set('history');
        onNavigate?.('blame');
      },
    });

    // Unstaged-only actions: discard, delete (untracked), gitignore patterns
    if (!isStaged) {
      const file = files.find(f => f.path === filePath);
      if (file) {
        items.push({ separator: true });
        items.push({
          label: m.changes_menu_discard(),
          action: () => {
            discardTargetPath = filePath;
            discardTargetIsUntracked = file.status === "new";
            showDiscardConfirm = true;
          },
        });
      }
      if (file && file.status === "new") {
        items.push({
          label: m.changes_menu_delete_file(),
          action: () => {
            deleteTargetPath = filePath;
            showDeleteConfirm = true;
          },
        });
        const patterns = buildGitignorePatterns(filePath);
        for (const p of patterns) {
          items.push({
            label: p.label,
            action: async () => {
              try {
                await runMutation({
                  kind: "gitignore_add",
                  invoke: () => addGitignorePattern(p.pattern),
                  successToast: () => `Added \`${p.pattern}\` to .gitignore`,
                  failureToastPrefix: "Gitignore update failed",
                });
              } catch {
                // runMutation already surfaced the toast.
              }
            },
          });
        }
      }
    }

    return items;
  }

  async function handleConfirmDelete() {
    if (!deleteTargetPath) return;
    const path = deleteTargetPath;
    try {
      await runMutation({
        kind: "clean",
        invoke: () => cleanPaths([path]),
        successToast: () => `Deleted ${path}`,
        failureToastPrefix: "Delete failed",
      });
    } catch {
      // runMutation already surfaced the toast.
    }
    showDeleteConfirm = false;
    deleteTargetPath = null;
  }

  async function handleConfirmDiscard() {
    if (!discardTargetPath) return;
    const path = discardTargetPath;
    const isUntracked = discardTargetIsUntracked;
    try {
      await runMutation<void>({
        kind: "discard",
        invoke: async () => {
          if (isUntracked) await cleanPaths([path]);
          else await discardFiles([path]);
        },
        successToast: () => (isUntracked ? `Deleted ${path}` : `Discarded changes in ${path}`),
        failureToastPrefix: "Discard failed",
      });
    } catch {
      // runMutation already surfaced the toast.
    }
    showDiscardConfirm = false;
    discardTargetPath = null;
    discardTargetIsUntracked = false;
  }

  function openContextMenu(e: MouseEvent, filePath: string) {
    e.preventDefault();
    contextMenuFile = filePath;
    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
    contextMenuVisible = true;
  }
</script>

<div class="changes-list" data-testid={isStaged ? "changes-list-staged" : "changes-list-unstaged"}>
  <div class="list-header">
    <div class="header-left">
      <Checkbox
        checked={allSelected}
        indeterminate={someSelected}
        disabled={files.length === 0}
        ariaLabel={m.changes_select_all()}
        onclick={toggleAll}
      />
      <span class="list-title">{title}</span>
      <span class="file-count">{files.length}</span>
    </div>
    {#if isStaged && onUnstage}
      {#if selectedCount > 0}
        <Button variant="neutral" size="sm" testid="unstage-selected-btn" onclick={unstageSelected}>
          {m.changes_unstage_selected({ count: String(selectedCount) })}
        </Button>
      {:else}
        <Button variant="neutral" size="sm" testid="unstage-all-btn" onclick={() => onUnstage(files.map(f => f.path))}>
          {m.changes_unstage_all()}
        </Button>
      {/if}
    {/if}
    {#if !isStaged && onStage}
      {#if selectedCount > 0}
        <Button variant="primary" size="sm" testid="stage-selected-btn" onclick={stageSelected}>
          {m.changes_stage_selected({ count: String(selectedCount) })}
        </Button>
      {:else}
        <Button variant="primary" size="sm" testid="stage-all-btn" onclick={() => onStage(files.map(f => f.path))}>
          {m.changes_stage_all()}
        </Button>
      {/if}
    {/if}
  </div>
  <div class="file-list" role="list">
    {#each files as file}
      <div
        class="file-item"
        class:selected={file.path === selectedPath}
        role="listitem"
        data-testid="file-row-{file.path.replace(/\//g, '-')}"
        oncontextmenu={(e) => openContextMenu(e, file.path)}
      >
        <Checkbox
          checked={selected.has(file.path)}
          ariaLabel={file.path}
          onclick={(e) => { e.stopPropagation(); toggleFile(file.path); }}
        />
        <button
          class="file-btn"
          onclick={() => onFileClick?.(file.path)}
        >
          <span class="status-badge status-{file.status}">
            {statusIcon(file.status)}
          </span>
          <span class="file-path">{file.path}</span>
        </button>
        {#if isStaged && onUnstage}
          <span class="item-action" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); onUnstage([file.path]); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); onUnstage([file.path]); } }}>&#8722;</span>
        {/if}
        {#if !isStaged && onStage}
          <span class="item-action" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); onStage([file.path]); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); onStage([file.path]); } }}>+</span>
        {/if}
      </div>
    {/each}
  </div>
</div>

<ContextMenu
  items={contextMenuFile ? buildContextMenuItems(contextMenuFile) : []}
  x={contextMenuX}
  y={contextMenuY}
  visible={contextMenuVisible}
  onClose={() => (contextMenuVisible = false)}
/>

{#if showDeleteConfirm && deleteTargetPath}
  <ConfirmDialog
    title={m.changes_menu_delete_confirm_title()}
    message={m.changes_menu_delete_confirm_message({ path: deleteTargetPath })}
    destructive={true}
    onConfirm={handleConfirmDelete}
    onCancel={() => { showDeleteConfirm = false; deleteTargetPath = null; }}
  />
{/if}

{#if showDiscardConfirm && discardTargetPath}
  <ConfirmDialog
    title={discardTargetIsUntracked
      ? m.changes_menu_delete_confirm_title()
      : m.changes_menu_discard_confirm_title()}
    message={discardTargetIsUntracked
      ? m.changes_menu_delete_confirm_message({ path: discardTargetPath })
      : m.changes_menu_discard_confirm_message({ path: discardTargetPath })}
    destructive={true}
    onConfirm={handleConfirmDiscard}
    onCancel={() => { showDiscardConfirm = false; discardTargetPath = null; discardTargetIsUntracked = false; }}
  />
{/if}

<style>
  .changes-list {
    display: flex;
    flex-direction: column;
  }

  .list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .list-title {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 500;
  }

  .file-count {
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
    background: var(--overlay-hover);
    padding: 1px 6px;
    border-radius: 8px;
    font-variant-numeric: tabular-nums;
    min-width: 18px;
    text-align: center;
  }

  .file-list {
    overflow-y: auto;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 12px;
    width: 100%;
    border-left: 2px solid transparent;
    transition: background 0.1s ease, border-color 0.1s ease;
  }

  .file-item:hover {
    background: var(--overlay-hover);
    border-left-color: var(--accent-primary);
  }

  /* Row whose diff is open in the panel. Mirrors the selected style of
     `common/FileChangeList.svelte` so both file lists read the same. */
  .file-item.selected {
    background: var(--overlay-accent-blue);
    border-left-color: var(--accent-primary);
  }

  .file-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    text-align: left;
    padding: 2px 0;
  }

  .status-badge {
    font-family: var(--font-mono);
    font-size: var(--font-size-2xs);
    font-weight: 700;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    line-height: 1;
  }

  .status-new {
    color: var(--accent-green);
    background: var(--overlay-accent-green);
  }

  .status-modified {
    color: var(--accent-orange);
    background: var(--overlay-accent-orange);
  }

  .status-deleted {
    color: var(--accent-red);
    background: var(--overlay-accent-red);
  }

  .status-renamed {
    color: var(--accent-purple);
    background: var(--overlay-accent-purple);
  }

  .file-path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-action {
    opacity: 0;
    font-size: var(--font-size-sm);
    font-weight: 600;
    background: var(--overlay-hover);
    border: none;
    border-radius: 4px;
    line-height: 1;
    color: var(--accent-primary);
    cursor: pointer;
    padding: 2px 6px;
    flex-shrink: 0;
    transition: opacity 0.15s ease, background 0.15s ease;
  }

  .file-item:hover .item-action {
    opacity: 1;
  }

  .item-action:hover {
    background: var(--overlay-accent-blue);
  }
</style>
