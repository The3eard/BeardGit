<script lang="ts">
  /**
   * AI Sessions view — split pane with the session list on the left and the
   * selected session's detail (transcript + actions) on the right.
   */
  import { onMount } from "svelte";
  import SplitView from "../common/SplitView.svelte";
  import AiSessionList from "./AiSessionList.svelte";
  import AiSessionDetail from "./AiSessionDetail.svelte";
  import {
    refreshAiBackgroundRuns,
    startAiBackgroundListeners,
  } from "$lib/stores/aiBackground";
  import { refreshSessions } from "$lib/stores/aiSessions";
  import { repoInfo } from "$lib/stores/repo";

  onMount(() => {
    startAiBackgroundListeners();
    void refreshAiBackgroundRuns().catch(() => {});
    const path = $repoInfo?.path;
    if (path) void refreshSessions(path).catch(() => {});
  });
</script>

<div class="ai-sessions-view" data-testid="ai-sessions-view">
  <SplitView
    defaultWidth={380}
    refreshFn={async () => {
      const path = $repoInfo?.path;
      if (path) await refreshSessions(path);
      await refreshAiBackgroundRuns();
    }}
  >
    {#snippet left()}<AiSessionList />{/snippet}
    {#snippet right()}<AiSessionDetail />{/snippet}
  </SplitView>
</div>

<style>
  .ai-sessions-view {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
</style>
