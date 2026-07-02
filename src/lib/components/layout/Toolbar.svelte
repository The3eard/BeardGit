<script lang="ts">
  import { activeProject, activeRepoStatus } from "$lib/stores/projects";
  import { repoInfo } from "$lib/stores/repo";
  import { fetchRemote, pullRemote, pushRemote, previewPatch, applyPatch } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import { open } from "@tauri-apps/plugin-dialog";
  import PatchPreviewDialog from "../patch/PatchPreviewDialog.svelte";
  import type { PatchPreview } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import { Button } from "$lib/components/ui";
  import { shouldShowSyncBadge, formatSyncBadge } from "$lib/utils/sync-badge";

  // Ahead/behind of the current branch vs its upstream, read from the
  // same `activeRepoStatus` the status bar uses — kept fresh by the
  // mutation-events pipeline (no polling, no new command). Both counts
  // are 0 when there's no upstream or HEAD is detached → no badge, no tint.
  let aheadCount = $derived($activeRepoStatus?.ahead ?? 0);
  let behindCount = $derived($activeRepoStatus?.behind ?? 0);

  let pushTooltip = $derived(
    aheadCount > 0
      ? aheadCount === 1
        ? m.toolbar_push_ahead_one({ count: String(aheadCount) })
        : m.toolbar_push_ahead({ count: String(aheadCount) })
      : m.toolbar_push(),
  );
  let pullTooltip = $derived(
    behindCount > 0
      ? behindCount === 1
        ? m.toolbar_pull_behind_one({ count: String(behindCount) })
        : m.toolbar_pull_behind({ count: String(behindCount) })
      : m.toolbar_pull(),
  );

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
        // `fetchRemote` returns the task-runner's TaskId (a monotonic
        // u64), not a ref count — the background `git fetch` finishes
        // later. Toast just reports spawn; Tasks drawer (Cmd+J) carries
        // the refs-updated summary.
        successToast: () => "Fetched origin",
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
        // `pullRemote` returns a TaskId, not a commit count — see the
        // fetch handler above. Toast reports spawn; Tasks drawer (Cmd+J)
        // carries the final commit summary.
        successToast: () => `Pulled origin/${branch}`,
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
        invoke: () => pushRemote("origin", branch, false),
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
      <Button
        variant="neutral"
        size="sm"
        disabled={fetchInProgress}
        description={m.toolbar_fetch()}
        onclick={handleFetch}
      >{m.toolbar_fetch()}</Button>
      <span class="badge-wrap">
        <Button
          variant="neutral"
          size="sm"
          active={behindCount > 0}
          disabled={pullInProgress || !$repoInfo?.head_branch}
          description={pullTooltip}
          onclick={handlePull}
        >{m.toolbar_pull()}</Button>
        {#if shouldShowSyncBadge(behindCount)}
          <span class="sync-badge" aria-hidden="true">↓{formatSyncBadge(behindCount)}</span>
        {/if}
      </span>
      <span class="badge-wrap">
        <Button
          variant="neutral"
          size="sm"
          testid="push-button"
          active={aheadCount > 0}
          disabled={pushInProgress || !$repoInfo?.head_branch}
          description={pushTooltip}
          onclick={handlePush}
        >{m.toolbar_push()}</Button>
        {#if shouldShowSyncBadge(aheadCount)}
          <span class="sync-badge" aria-hidden="true">↑{formatSyncBadge(aheadCount)}</span>
        {/if}
      </span>
      <Button
        variant="neutral"
        size="sm"
        description={m.patch_apply()}
        onclick={handleApplyPatch}
      >{m.patch_apply()}</Button>
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

  /* The wrapper is sized entirely by the button it contains (inline-flex,
     no padding/border), so the button's geometry is identical whether or
     not a badge is present. The badge is an absolutely-positioned overlay
     — it's out of flow and cannot resize or shift the button. */
  .badge-wrap {
    position: relative;
    display: inline-flex;
  }

  .sync-badge {
    position: absolute;
    top: -6px;
    right: -6px;
    height: 14px;
    min-width: 14px;
    box-sizing: border-box;
    padding: 0 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 7px;
    /* Ring in the toolbar background so the pill reads as detached from
       the button edge it overlaps. */
    border: 1px solid var(--bg-toolbar);
    background: var(--accent-primary);
    color: var(--text-primary);
    font-size: var(--font-size-2xs);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    pointer-events: none;
  }

</style>
