<script lang="ts">
  /**
   * AI Sessions view — split pane with the session list on the left and the
   * selected session's detail (transcript + actions) on the right.
   *
   * Intentionally mirrors `TagView` / `BranchView` / etc. — a thin
   * `SplitView` wrapper with a single `refreshFn`. Listener setup moved
   * to the app-shell init (`+layout.svelte`) so the initial mount of
   * this view stays a zero-work shell-paint (matches the rest of the
   * async-first sections — the view swap paints immediately, data
   * streams in through the refresh callback).
   */
  import SplitView from "../common/SplitView.svelte";
  import AiSessionList from "./AiSessionList.svelte";
  import AiSessionDetail from "./AiSessionDetail.svelte";
  import { refreshAiBackgroundRuns } from "$lib/stores/aiBackground";
  import { refreshSessions } from "$lib/stores/aiSessions";
  import { repoInfo } from "$lib/stores/repo";

  async function refreshAll(): Promise<void> {
    const path = $repoInfo?.path;
    if (path) await refreshSessions(path);
    await refreshAiBackgroundRuns();
  }
</script>

<div class="ai-sessions-view" data-testid="ai-sessions-view">
  <SplitView defaultWidth={380} refreshFn={refreshAll}>
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
