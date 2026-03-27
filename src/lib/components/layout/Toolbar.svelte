<script lang="ts">
  import { activeProject } from "$lib/stores/projects";
  import { repoInfo } from "$lib/stores/repo";
  import { fetchRemote, pullRemote, pushRemote } from "$lib/api/tauri";
  import * as m from "$lib/paraglide/messages";

  let fetchInProgress = $state(false);
  let pullInProgress = $state(false);
  let pushInProgress = $state(false);

  async function handleFetch() {
    if (fetchInProgress) return;
    fetchInProgress = true;
    try {
      await fetchRemote("origin");
    } finally {
      fetchInProgress = false;
    }
  }

  async function handlePull() {
    if (pullInProgress || !$repoInfo?.head_branch) return;
    pullInProgress = true;
    try {
      await pullRemote("origin", $repoInfo.head_branch);
    } finally {
      pullInProgress = false;
    }
  }

  async function handlePush() {
    if (pushInProgress || !$repoInfo?.head_branch) return;
    pushInProgress = true;
    try {
      await pushRemote("origin", $repoInfo.head_branch);
    } finally {
      pushInProgress = false;
    }
  }
</script>

<header class="toolbar">
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
    {/if}
  </div>
</header>

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
    -webkit-app-region: drag;
    user-select: none;
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: no-drag;
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
    -webkit-app-region: no-drag;
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
