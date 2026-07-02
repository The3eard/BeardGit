<script lang="ts">
  import type { FileStatus, FileDiffStat } from "../../types";
  import * as m from "$lib/paraglide/messages";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import FileStatusBadge from "../common/FileStatusBadge.svelte";
  import { openBlame, blameActiveTab } from "$lib/stores/blame";
  import { doStashPush } from "$lib/stores/stashes";
  import { unstagedSelection, stagedSelection } from "$lib/stores/changesSelection";
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
    stats,
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
    /** Per-file add/del stats keyed by path, for the +N/-N row counts. */
    stats?: Map<string, FileDiffStat>;
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

  // Checkbox selection is backed by a store so it PERSISTS across leaving
  // and re-entering the Changes view (see changesSelection.ts). `isStaged`
  // is fixed per instance — it just picks which list's store to read/write.
  let selected = $derived(isStaged ? $stagedSelection : $unstagedSelection);

  function setSelection(next: Set<string>) {
    (isStaged ? stagedSelection : unstagedSelection).set(next);
  }

  let selectedCount = $derived(selected.size);
  let allSelected = $derived(files.length > 0 && selected.size === files.length);
  let someSelected = $derived(selected.size > 0 && selected.size < files.length);

  // Keyboard navigation: `focusIndex` is the arrow-key cursor and
  // `anchorIndex` the fixed end of a Shift range. Both are component-local
  // so the cursor starts fresh each visit (unlike the persisted selection).
  let focusIndex = $state(-1);
  let anchorIndex = $state(-1);
  let listEl = $state<HTMLDivElement | null>(null);

  function toggleFile(path: string, index = -1) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelection(next);
    if (index >= 0) {
      anchorIndex = index;
      focusIndex = index;
    }
  }

  function toggleAll() {
    setSelection(allSelected ? new Set() : new Set(files.map((f) => f.path)));
  }

  function stageSelected() {
    const paths = [...selected];
    setSelection(new Set());
    onStage?.(paths);
  }

  function unstageSelected() {
    const paths = [...selected];
    setSelection(new Set());
    onUnstage?.(paths);
  }

  /** Add every file between two row indices (inclusive) to the selection. */
  function selectRange(a: number, b: number) {
    const lo = Math.min(a, b);
    const hi = Math.max(a, b);
    const next = new Set(selected);
    for (let i = lo; i <= hi; i++) {
      const f = files[i];
      if (f) next.add(f.path);
    }
    setSelection(next);
  }

  function setFocus(index: number) {
    focusIndex = Math.max(0, Math.min(index, files.length - 1));
    const row = listEl?.querySelector<HTMLElement>(`[data-row-index="${focusIndex}"]`);
    row?.scrollIntoView({ block: "nearest" });
  }

  function handleRowClick(e: MouseEvent, index: number) {
    // Shift-click selects the range from the anchor to the clicked row
    // instead of opening the diff.
    if (e.shiftKey && anchorIndex >= 0) {
      e.preventDefault();
      selectRange(anchorIndex, index);
      focusIndex = index;
      listEl?.focus();
      return;
    }
    anchorIndex = index;
    focusIndex = index;
    listEl?.focus();
    onFileClick?.(files[index].path);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (files.length === 0) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const delta = e.key === "ArrowDown" ? 1 : -1;
      const from = focusIndex < 0 ? (delta > 0 ? -1 : files.length) : focusIndex;
      const next = Math.max(0, Math.min(from + delta, files.length - 1));
      if (e.shiftKey) {
        if (anchorIndex < 0) anchorIndex = focusIndex < 0 ? next : focusIndex;
        selectRange(anchorIndex, next);
      } else {
        anchorIndex = next;
      }
      setFocus(next);
    } else if (e.key === " ") {
      e.preventDefault();
      if (focusIndex >= 0 && focusIndex < files.length) {
        toggleFile(files[focusIndex].path, focusIndex);
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (focusIndex >= 0 && focusIndex < files.length) {
        onFileClick?.(files[focusIndex].path);
      }
    }
  }

  // Prune selection to paths that still exist (after stage/unstage/refresh)
  // rather than clearing it, so the selection survives refreshes and
  // view switches. A stale keyboard cursor self-heals on the next arrow.
  $effect(() => {
    const present = new Set(files.map((f) => f.path));
    (isStaged ? stagedSelection : unstagedSelection).update((sel) => {
      let changed = false;
      const nextSel = new Set<string>();
      for (const p of sel) {
        if (present.has(p)) nextSel.add(p);
        else changed = true;
      }
      return changed ? nextSel : sel;
    });
  });

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

    // Stash the checkbox selection, falling back to the right-clicked file
    // when nothing is checked.
    const stashPaths = selected.size > 0 ? [...selected] : [filePath];
    items.push({
      label: m.changes_menu_stash_selected({ count: String(stashPaths.length) }),
      action: () => {
        setSelection(new Set());
        void doStashPush(null, stashPaths);
      },
    });

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
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="file-list" role="list" tabindex="0" bind:this={listEl} onkeydown={handleKeydown}>
    {#each files as file, i}
      {@const stat = stats?.get(file.path)}
      <div
        class="file-item"
        class:selected={file.path === selectedPath}
        class:focused={i === focusIndex}
        role="listitem"
        data-row-index={i}
        data-testid="file-row-{file.path.replace(/\//g, '-')}"
        oncontextmenu={(e) => openContextMenu(e, file.path)}
      >
        <Checkbox
          checked={selected.has(file.path)}
          ariaLabel={file.path}
          onclick={(e) => { e.stopPropagation(); listEl?.focus(); toggleFile(file.path, i); }}
        />
        <button
          class="file-btn"
          onclick={(e) => handleRowClick(e, i)}
        >
          <FileStatusBadge status={file.status} />
          <span class="file-path">{file.path}</span>
          {#if stat}
            {#if stat.binary}
              <span class="file-stat file-stat-binary">{m.diff_binary_short()}</span>
            {:else}
              {#if stat.additions > 0}
                <span class="file-stat file-stat-add">+{stat.additions}</span>
              {/if}
              {#if stat.deletions > 0}
                <span class="file-stat file-stat-del">-{stat.deletions}</span>
              {/if}
            {/if}
          {/if}
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

  /* Keyboard-navigation cursor (arrow keys) — a thin ring, distinct from
     `.selected` (the open file's filled background). */
  .file-item.focused {
    outline: 1px solid var(--accent-primary);
    outline-offset: -1px;
    border-radius: 2px;
  }

  .file-list:focus {
    outline: none;
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

  .file-path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-stat {
    flex-shrink: 0;
    font-size: var(--font-size-2xs);
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .file-stat-add {
    color: var(--accent-green);
  }

  .file-stat-del {
    color: var(--accent-red);
  }

  .file-stat-binary {
    color: var(--text-secondary);
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
