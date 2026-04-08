<script lang="ts">
  import type { FileStatus } from "../../types";
  import * as m from "$lib/paraglide/messages";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import { openBlame, blameActiveTab } from "$lib/stores/blame";
  import { cleanPaths } from "$lib/api/tauri";
  import { addGitignorePattern } from "$lib/api/tauri";
  import { refreshStatuses, refreshDiffs } from "$lib/stores/changes";

  let {
    files,
    title,
    onStage,
    onUnstage,
    isStaged = false,
    onFileClick,
    onNavigate,
  }: {
    files: FileStatus[];
    title: string;
    onStage?: (paths: string[]) => void;
    onUnstage?: (paths: string[]) => void;
    isStaged?: boolean;
    onFileClick?: (path: string) => void;
    onNavigate?: (view: string) => void;
  } = $props();

  let contextMenuVisible = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let contextMenuFile = $state<string | null>(null);
  let showDeleteConfirm = $state(false);
  let deleteTargetPath = $state<string | null>(null);

  function statusIcon(status: string): string {
    switch (status) {
      case "new": return "+";
      case "modified": return "~";
      case "deleted": return "-";
      case "renamed": return "R";
      default: return "?";
    }
  }

  function statusColor(status: string): string {
    switch (status) {
      case "new": return "var(--accent-green)";
      case "modified": return "var(--accent-orange)";
      case "deleted": return "var(--accent-red)";
      case "renamed": return "var(--accent-purple)";
      default: return "var(--text-secondary)";
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

    // Untracked file actions: delete + gitignore patterns
    if (!isStaged) {
      const file = files.find(f => f.path === filePath);
      if (file && file.status === "new") {
        items.push({ separator: true });
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
              await addGitignorePattern(p.pattern);
              await Promise.all([refreshStatuses(), refreshDiffs()]);
            },
          });
        }
      }
    }

    return items;
  }

  async function handleConfirmDelete() {
    if (!deleteTargetPath) return;
    await cleanPaths([deleteTargetPath]);
    showDeleteConfirm = false;
    deleteTargetPath = null;
    await Promise.all([refreshStatuses(), refreshDiffs()]);
  }

  function openContextMenu(e: MouseEvent, filePath: string) {
    e.preventDefault();
    contextMenuFile = filePath;
    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
    contextMenuVisible = true;
  }
</script>

<div class="changes-list">
  <div class="list-header">
    <span class="list-title">{title} ({files.length})</span>
    {#if isStaged && onUnstage}
      <button class="action-btn" onclick={() => onUnstage(files.map(f => f.path))}>
        {m.changes_unstage_all()}
      </button>
    {/if}
    {#if !isStaged && onStage}
      <button class="action-btn" onclick={() => onStage(files.map(f => f.path))}>
        {m.changes_stage_all()}
      </button>
    {/if}
  </div>
  <div class="file-list">
    {#each files as file}
      <button
        class="file-item"
        onclick={() => onFileClick?.(file.path)}
        oncontextmenu={(e) => openContextMenu(e, file.path)}
      >
        <span class="status-icon" style="color: {statusColor(file.status)}">
          {statusIcon(file.status)}
        </span>
        <span class="file-path">{file.path}</span>
        {#if isStaged && onUnstage}
          <span class="item-action" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); onUnstage([file.path]); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); onUnstage([file.path]); } }}>&#8722;</span>
        {/if}
        {#if !isStaged && onStage}
          <span class="item-action" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); onStage([file.path]); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); onStage([file.path]); } }}>+</span>
        {/if}
      </button>
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

<style>
  .changes-list { display: flex; flex-direction: column; }
  .list-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 6px 12px; border-bottom: 1px solid var(--border);
  }
  .list-title { font-size: 11px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; }
  .action-btn {
    font-size: 10px; color: var(--accent-blue); background: none;
    border: none; cursor: pointer; padding: 2px 6px;
  }
  .action-btn:hover { text-decoration: underline; }
  .file-list { overflow-y: auto; }
  .file-item {
    display: flex; align-items: center; gap: 8px; padding: 4px 12px;
    width: 100%; background: none; border: none; color: var(--text-primary);
    font-size: 12px; cursor: pointer; text-align: left;
  }
  .file-item:hover { background: rgba(255,255,255,0.04); }
  .status-icon { font-family: var(--font-mono); font-weight: bold; width: 12px; }
  .file-path { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item-action {
    opacity: 0; font-size: 14px; background: none; border: none; line-height: 1;
    color: var(--accent-blue); cursor: pointer; padding: 0 4px;
  }
  .file-item:hover .item-action { opacity: 1; }
</style>
