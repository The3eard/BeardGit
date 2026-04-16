<script lang="ts">
  import type { PatchPreview } from "../../types";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    preview: PatchPreview;
    patchPath: string;
    onApply: (threeWay: boolean) => void;
    onClose: () => void;
  }

  let { preview, patchPath, onApply, onClose }: Props = $props();

  let fileName = $derived(patchPath.split("/").pop() || patchPath.split("\\").pop() || patchPath);
</script>

<div class="backdrop" onclick={onClose} role="presentation"></div>
<div class="dialog" role="dialog" aria-label={m.patch_preview_title()}>
  <h3 class="dialog-title">{m.patch_preview_title()}</h3>

  <div class="patch-file-name">{fileName}</div>

  <div class="status-badge" class:clean={preview.applies_cleanly} class:conflict={!preview.applies_cleanly}>
    {preview.applies_cleanly ? m.patch_applies_cleanly() : m.patch_has_conflicts()}
  </div>

  <div class="summary-row">
    <span>{m.patch_files_changed({ count: preview.total_files })}</span>
    <span class="insertions">{m.patch_insertions({ count: preview.total_insertions })}</span>
    <span class="deletions">{m.patch_deletions({ count: preview.total_deletions })}</span>
  </div>

  <div class="file-list">
    {#each preview.stats as stat}
      <div class="file-row">
        <span class="file-path">{stat.path}</span>
        <span class="insertions">+{stat.insertions}</span>
        <span class="deletions">-{stat.deletions}</span>
      </div>
    {/each}
  </div>

  <div class="dialog-actions">
    {#if preview.applies_cleanly}
      <button class="action-btn primary" onclick={() => onApply(false)}>
        {m.patch_apply_button()}
      </button>
    {:else}
      <button class="action-btn primary" onclick={() => onApply(true)}>
        {m.patch_apply_3way()}
      </button>
    {/if}
    <button class="action-btn secondary" onclick={onClose}>
      {m.patch_cancel()}
    </button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions */

  .dialog {
    border-radius: 8px;
    padding: 20px;
    min-width: 400px;
    max-width: 600px;
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .dialog-title {
    margin-bottom: 12px;
    font-size: 15px;
  }

  .patch-file-name {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 8px;
    word-break: break-all;
  }

  .status-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    margin-bottom: 12px;
  }

  .status-badge.clean {
    background: rgba(63, 185, 80, 0.15);
    color: var(--accent-green);
  }

  .status-badge.conflict {
    background: rgba(248, 81, 73, 0.15);
    color: var(--accent-red);
  }

  .summary-row {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 8px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    margin-bottom: 16px;
  }

  .file-row {
    display: flex;
    gap: 8px;
    padding: 3px 0;
    font-size: 12px;
  }

  .file-path {
    flex: 1;
    color: var(--text-primary);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .insertions {
    color: var(--accent-green);
  }

  .deletions {
    color: var(--accent-red);
  }

  .action-btn {
    padding: 6px 14px;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    background: none;
    color: var(--text-primary);
  }

  .action-btn.primary {
    background: var(--accent-blue);
    color: #ffffff;
    border-color: var(--accent-blue);
  }

  .action-btn.secondary:hover {
    background: rgba(255, 255, 255, 0.05);
  }
</style>
