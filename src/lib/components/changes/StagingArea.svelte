<script lang="ts">
  import { fileStatuses, stageFiles, unstageFiles, stageAll, unstageAll, commit, commitMessage, refreshStatuses, refreshDiffs } from "../../stores/changes";
  import ChangesList from "./ChangesList.svelte";
  import CleanDialog from "./CleanDialog.svelte";
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { amendCommit, getHeadMessage, createWorkingTreePatch, savePatchToFile } from "$lib/api/tauri";
  import { save } from "@tauri-apps/plugin-dialog";

  let {
    onFileClick,
    onNavigate,
  }: {
    onFileClick?: (path: string, staged: boolean) => void;
    onNavigate?: (view: string) => void;
  } = $props();

  let message = $state("");
  let isAmend = $state(false);
  let savedMessage = $state("");
  let showPatchDialog = $state(false);
  let patchStagedOnly = $state(true);

  onMount(async () => {
    await refreshStatuses();
    await refreshDiffs();
  });

  let staged = $derived($fileStatuses.filter(f => f.is_staged));
  let unstaged = $derived($fileStatuses.filter(f => !f.is_staged));
  let hasUntracked = $derived(unstaged.some(f => f.status === "new"));
  let showCleanDialog = $state(false);

  async function handleAmendToggle() {
    if (isAmend) {
      savedMessage = message;
      try {
        message = await getHeadMessage();
      } catch {
        message = '';
      }
    } else {
      message = savedMessage;
      savedMessage = '';
    }
  }

  async function handleCreatePatch() {
    try {
      const patchText = await createWorkingTreePatch(patchStagedOnly);
      const filePath = await save({
        title: m.patch_save_dialog_title(),
        defaultPath: "changes.patch",
        filters: [{ name: "Patch", extensions: ["patch", "diff"] }],
      });
      if (!filePath) return;
      await savePatchToFile(filePath, patchText);
      showPatchDialog = false;
    } catch (err) {
      alert(m.patch_create_failed({ error: String(err) }));
    }
  }

  async function handleCommit() {
    if (!message.trim()) return;
    if (isAmend) {
      await amendCommit(message);
    } else {
      await commit(message);
    }
    message = "";
    isAmend = false;
  }
</script>

<div class="staging-area">
  <ChangesList
    files={staged}
    title={m.staging_staged()}
    isStaged={true}
    onUnstage={(paths) => unstageFiles(paths)}
    onFileClick={(path) => onFileClick?.(path, true)}
    onNavigate={onNavigate}
  />

  <div class="commit-box">
    <label class="amend-toggle">
      <input type="checkbox" bind:checked={isAmend} onchange={handleAmendToggle} />
      <span>{m.staging_amend_toggle()}</span>
    </label>
    <textarea
      class="commit-input"
      placeholder={m.staging_commit_placeholder()}
      bind:value={message}
      onkeydown={(e) => { if (e.key === 'Enter' && e.metaKey) handleCommit(); }}
    ></textarea>
    <div class="commit-actions-row">
      <button
        class="commit-btn"
        disabled={!message.trim() || (!isAmend && staged.length === 0)}
        onclick={handleCommit}
      >
        {isAmend
          ? m.staging_amend_button()
          : staged.length === 1
            ? m.staging_commit_button_one({ count: String(staged.length) })
            : m.staging_commit_button({ count: String(staged.length) })}
      </button>
      <button class="patch-btn" onclick={() => { showPatchDialog = true; }}>
        {m.patch_create_changes()}
      </button>
    </div>

    {#if showPatchDialog}
      <div class="patch-source-dialog">
        <label class="radio-label">
          <input type="radio" bind:group={patchStagedOnly} value={true} />
          {m.patch_staged_only()}
        </label>
        <label class="radio-label">
          <input type="radio" bind:group={patchStagedOnly} value={false} />
          {m.patch_all_changes()}
        </label>
        <div class="patch-dialog-actions">
          <button class="patch-btn" onclick={handleCreatePatch}>
            {m.patch_create_changes()}
          </button>
          <button class="patch-btn secondary" onclick={() => { showPatchDialog = false; }}>
            {m.patch_cancel()}
          </button>
        </div>
      </div>
    {/if}
  </div>

  <ChangesList
    files={unstaged}
    title={m.staging_unstaged()}
    isStaged={false}
    onStage={(paths) => stageFiles(paths)}
    onFileClick={(path) => onFileClick?.(path, false)}
    onNavigate={onNavigate}
  />

  {#if hasUntracked}
    <div class="clean-row">
      <button class="clean-btn" onclick={() => showCleanDialog = true}>
        {m.clean_button()}
      </button>
    </div>
  {/if}

  {#if showCleanDialog}
    <CleanDialog onClose={() => showCleanDialog = false} />
  {/if}
</div>

<style>
  .staging-area {
    display: flex; flex-direction: column; height: 100%; overflow: hidden;
  }
  .commit-box {
    padding: 8px 12px; border-bottom: 1px solid var(--border);
    display: flex; flex-direction: column; gap: 6px;
  }
  .commit-input {
    width: 100%; min-height: 60px; resize: vertical;
    background: rgba(255,255,255,0.04); border: 1px solid var(--border);
    border-radius: 4px; padding: 6px 8px; color: var(--text-primary);
    font-size: 12px; font-family: inherit; outline: none;
  }
  .commit-input:focus { border-color: var(--accent-blue); }
  .commit-btn {
    align-self: flex-end; padding: 4px 12px; border-radius: 4px;
    background: var(--accent-blue); color: white; border: none;
    font-size: 12px; cursor: pointer;
  }
  .commit-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .commit-btn:hover:not(:disabled) { opacity: 0.9; }
  .amend-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .amend-toggle input[type="checkbox"] {
    margin: 0;
    accent-color: var(--accent-blue);
  }
  .clean-row {
    padding: 8px 12px;
    border-top: 1px solid var(--border);
  }
  .clean-btn {
    padding: 4px 12px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    font-size: 12px;
    cursor: pointer;
  }
  .clean-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }
  .commit-actions-row {
    display: flex;
    gap: 6px;
    align-items: center;
    justify-content: flex-end;
  }
  .patch-btn {
    padding: 4px 12px; border-radius: 4px;
    background: rgba(255,255,255,0.06); border: 1px solid var(--border);
    color: var(--text-primary); font-size: 12px; cursor: pointer;
  }
  .patch-btn:hover { background: rgba(255,255,255,0.1); }
  .patch-btn.secondary { opacity: 0.7; }
  .patch-source-dialog {
    padding: 8px 12px;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 6px;
    margin-top: 4px;
  }
  .radio-label {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
  }
  .patch-dialog-actions {
    display: flex;
    gap: 6px;
    margin-top: 8px;
    justify-content: flex-end;
  }
</style>
