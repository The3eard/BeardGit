<script lang="ts">
  import { fileStatuses, stageFiles, unstageFiles, stageAll, unstageAll, commit, commitMessage, refreshStatuses, refreshDiffs } from "../../stores/changes";
  import ChangesList from "./ChangesList.svelte";
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";

  let { onFileClick }: { onFileClick?: (path: string, staged: boolean) => void } = $props();

  let message = $state("");

  onMount(async () => {
    await refreshStatuses();
    await refreshDiffs();
  });

  let staged = $derived($fileStatuses.filter(f => f.is_staged));
  let unstaged = $derived($fileStatuses.filter(f => !f.is_staged));

  async function handleCommit() {
    if (!message.trim()) return;
    await commit(message, "Adolfo Fuentes", "adolfofuentes@metricool.com");
    message = "";
  }
</script>

<div class="staging-area">
  <ChangesList
    files={staged}
    title={m.staging_staged()}
    isStaged={true}
    onUnstage={(paths) => unstageFiles(paths)}
    onFileClick={(path) => onFileClick?.(path, true)}
  />

  <div class="commit-box">
    <textarea
      class="commit-input"
      placeholder={m.staging_commit_placeholder()}
      bind:value={message}
      onkeydown={(e) => { if (e.key === 'Enter' && e.metaKey) handleCommit(); }}
    ></textarea>
    <button
      class="commit-btn"
      disabled={!message.trim() || staged.length === 0}
      onclick={handleCommit}
    >
      {staged.length === 1 ? m.staging_commit_button_one({ count: String(staged.length) }) : m.staging_commit_button({ count: String(staged.length) })}
    </button>
  </div>

  <ChangesList
    files={unstaged}
    title={m.staging_unstaged()}
    isStaged={false}
    onStage={(paths) => stageFiles(paths)}
    onFileClick={(path) => onFileClick?.(path, false)}
  />
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
</style>
