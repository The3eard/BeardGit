<script lang="ts">
  /**
   * AI Sessions view — two-section sidebar (Active + Conversations) on
   * the left, detail pane on the right.
   *
   * Keeps the `SplitView` shell used by `TagView` / `BranchView` / etc.
   * so the view swap paints instantly. The `refreshFn` runs both list
   * refreshes in parallel so the SplitView spinner hides only once both
   * sections are fresh.
   *
   * Listener setup lives on the app shell (`+page.svelte`) so the
   * initial mount of this view stays a zero-work shell-paint (matches
   * the rest of the async-first sections).
   */
  import SplitView from "../common/SplitView.svelte";
  import AiSessionList from "./AiSessionList.svelte";
  import AiSessionDetail from "./AiSessionDetail.svelte";
  import { refreshConversations } from "$lib/stores/aiConversations";
  import { refreshAiBackgroundRuns } from "$lib/stores/aiBackground";
  import { repoInfo } from "$lib/stores/repo";

  async function refreshAll(): Promise<void> {
    const path = $repoInfo?.path;
    await Promise.all([
      refreshConversations(path),
      refreshAiBackgroundRuns(),
    ]);
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
