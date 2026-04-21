<script lang="ts">
  import { activeProject } from "$lib/stores/projects";
  import { repoInfo } from "$lib/stores/repo";
  import { fetchRemote, pullRemote, pushRemote, previewPatch, applyPatch } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import { open } from "@tauri-apps/plugin-dialog";
  import PatchPreviewDialog from "../patch/PatchPreviewDialog.svelte";
  import type { PatchPreview } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

  let fetchInProgress = $state(false);
  let pullInProgress = $state(false);
  let pushInProgress = $state(false);
  let patchPreview = $state<PatchPreview | null>(null);
  let patchPath = $state("");
  let applyInProgress = $state(false);

  async function handleFetch() {
    if (fetchInProgress) return;
    fetchInProgress = true;
    try {
      await runMutation({
        kind: "fetch",
        invoke: () => fetchRemote("origin"),
        successToast: (n) => `Fetched origin — ${n} ref${n === 1 ? "" : "s"}`,
        failureToastPrefix: "Fetch failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      fetchInProgress = false;
    }
  }

  async function handlePull() {
    if (pullInProgress || !$repoInfo?.head_branch) return;
    const branch = $repoInfo.head_branch;
    pullInProgress = true;
    try {
      await runMutation({
        kind: "pull",
        invoke: () => pullRemote("origin", branch),
        successToast: (n) =>
          `Pulled origin/${branch} — ${n} commit${n === 1 ? "" : "s"}`,
        failureToastPrefix: "Pull failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      pullInProgress = false;
    }
  }

  async function handlePush() {
    if (pushInProgress || !$repoInfo?.head_branch) return;
    const branch = $repoInfo.head_branch;
    pushInProgress = true;
    try {
      await runMutation({
        kind: "push",
        invoke: () => pushRemote("origin", branch),
        successToast: () => `Pushed to origin/${branch}`,
        failureToastPrefix: "Push failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      pushInProgress = false;
    }
  }

  async function handleApplyPatch() {
    try {
      const selected = await open({
        title: m.patch_open_dialog_title(),
        filters: [{ name: "Patch", extensions: ["patch", "diff"] }],
        multiple: false,
      });
      if (!selected) return;
      const filePath = typeof selected === "string" ? selected : selected;
      patchPath = filePath;
      patchPreview = await previewPatch(filePath);
    } catch (err) {
      alert(m.patch_apply_failed({ error: String(err) }));
    }
  }

  async function handleConfirmApply(threeWay: boolean) {
    if (applyInProgress) return;
    applyInProgress = true;
    try {
      await runMutation({
        kind: "patch_apply",
        invoke: () => applyPatch(patchPath, threeWay),
        successToast: () => "Patch applied",
        failureToastPrefix: "Patch apply failed",
      });
      patchPreview = null;
      patchPath = "";
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      applyInProgress = false;
    }
  }
</script>

<header class="toolbar" data-tauri-drag-region>
  <div class="toolbar-left">
    <!-- Repo name and branch are now in the tab bar -->
  </div>

  <div class="toolbar-right">
    {#if $activeProject}
      <button
        class="toolbar-btn action-btn"
        disabled={fetchInProgress}
        title={m.toolbar_fetch()}
        onclick={handleFetch}
      >
        {m.toolbar_fetch()}
      </button>
      <button
        class="toolbar-btn action-btn"
        disabled={pullInProgress || !$repoInfo?.head_branch}
        title={m.toolbar_pull()}
        onclick={handlePull}
      >
        {m.toolbar_pull()}
      </button>
      <button
        class="toolbar-btn action-btn"
        disabled={pushInProgress || !$repoInfo?.head_branch}
        title={m.toolbar_push()}
        onclick={handlePush}
      >
        {m.toolbar_push()}
      </button>
      <button
        class="toolbar-btn action-btn"
        title={m.patch_apply()}
        onclick={handleApplyPatch}
      >
        {m.patch_apply()}
      </button>
    {/if}
  </div>
</header>

{#if patchPreview}
  <PatchPreviewDialog
    preview={patchPreview}
    patchPath={patchPath}
    onApply={handleConfirmApply}
    onClose={() => { patchPreview = null; }}
  />
{/if}

<style>
  .toolbar {
    height: 44px;
    min-height: 44px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    gap: 8px;
    user-select: none;
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .toolbar-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .toolbar-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
  }

  .toolbar-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .action-btn {
    min-width: 50px;
    text-align: center;
  }
</style>
