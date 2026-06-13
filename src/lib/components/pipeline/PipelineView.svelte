<!--
  PipelineView — Resizable split view for pipeline list + detail/log.

  Wraps PipelineList (left) and PipelineDetail/JobLog (right) in a SplitView.
  When a job is selected, the right pane splits vertically: PipelineDetail on
  top and JobLog below, with a draggable resize handle between them.
-->
<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import PipelineList from "./PipelineList.svelte";
  import PipelineDetail from "./PipelineDetail.svelte";
  import JobLog from "./JobLog.svelte";
  import { IconButton } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";
  import { loadCiRuns } from "../../stores/provider";

  let showJobLog = $state(false);
  let detailHeight = $state(250);

  function handleJobSelect(_jobId: number) {
    showJobLog = true;
  }

  function closeJobLog() {
    showJobLog = false;
  }

  function startVerticalResize(e: MouseEvent) {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = detailHeight;
    // Measure the right pane at drag start: the detail may grow up to
    // 80% of it, so the job log always keeps ~20%.
    const containerHeight =
      (e.currentTarget as HTMLElement).parentElement?.clientHeight ??
      window.innerHeight;

    function onMouseMove(e: MouseEvent) {
      const delta = e.clientY - startY;
      // Min: 80px (enough for first row of jobs), max: 80% of the pane
      const minH = 80;
      const maxH = containerHeight * 0.8;
      detailHeight = Math.max(minH, Math.min(maxH, startHeight + delta));
    }

    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    }

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  async function refresh() {
    await loadCiRuns();
  }
</script>

<SplitView refreshFn={refresh} defaultWidth={420}>
  {#snippet left()}
    <PipelineList />
  {/snippet}
  {#snippet right()}
    <div class="pipeline-right">
      {#if showJobLog}
        <div class="pipelines-detail" style="height: {detailHeight}px">
          <PipelineDetail onSelectJob={handleJobSelect} />
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="vertical-resize-handle" onmousedown={startVerticalResize}></div>
        <div class="pipelines-log">
          <div class="log-header">
            <IconButton icon={"\uF00D"} description={m.tooltip_close_log()} onclick={closeJobLog} />
          </div>
          <div class="log-content">
            <JobLog />
          </div>
        </div>
      {:else}
        <PipelineDetail onSelectJob={handleJobSelect} />
      {/if}
    </div>
  {/snippet}
</SplitView>

<style>
  .pipeline-right {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .pipelines-detail {
    flex-shrink: 0;
    min-height: 80px;
    overflow: auto;
  }

  .vertical-resize-handle {
    height: 4px;
    cursor: ns-resize;
    background: transparent;
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    transition: background 0.15s;
  }

  .vertical-resize-handle:hover {
    background: var(--accent-primary);
  }

  .pipelines-log {
    flex: 1;
    overflow: hidden;
    min-height: 60px;
    display: flex;
    flex-direction: column;
  }

  .log-header {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 2px 8px;
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .log-content {
    flex: 1;
    overflow: hidden;
  }
</style>
