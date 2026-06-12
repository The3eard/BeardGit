<script lang="ts">
  import type { ProjectInfo, LinkedSegment, ProjectSnapshot } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import TabTooltip from "./TabTooltip.svelte";
  import TabStatusStrip from "./TabStatusStrip.svelte";
  import { BRAND_COLORS } from "$lib/ui/brand-colors";
  import {
    getSnapshotForHover,
    refreshProjectSnapshot,
    projectSnapshots,
  } from "$lib/stores/project-cache";
  import IconButton from "$lib/components/ui/IconButton.svelte";

  interface Props {
    project: ProjectInfo;
    segments: LinkedSegment[];
    activeSegmentIndex: number;
    isActiveTab: boolean;
    onSwitchSegment: (segmentIndex: number) => void;
    onCloseProject: () => void;
    onCloseSegment: (segmentIndex: number) => void;
  }

  let {
    project,
    segments,
    activeSegmentIndex,
    isActiveTab,
    onSwitchSegment,
    onCloseProject,
    onCloseSegment,
  }: Props = $props();

  let statusColor = $derived(
    isActiveTab && activeSegmentIndex === -1
      ? "var(--accent-primary)"
      : project.change_count > 0
        ? "var(--accent-orange)"
        : "var(--accent-green)"
  );

  function handleProjectClick() {
    onSwitchSegment(-1);
  }

  function handleSegmentClick(index: number) {
    onSwitchSegment(index);
  }

  function handleCloseProject(e: MouseEvent) {
    e.stopPropagation();
    onCloseProject();
  }

  function handleCloseSegment(e: MouseEvent, index: number) {
    e.stopPropagation();
    onCloseSegment(index);
  }

  function handleProjectMiddleClick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
      onCloseProject();
    }
  }

  function handleSegmentMiddleClick(e: MouseEvent, index: number) {
    if (e.button === 1) {
      e.preventDefault();
      onCloseSegment(index);
    }
  }

  /** Extract the short label from terminal title (e.g. "Terminal" from "Terminal · BeardGit") */
  function shortLabel(title: string): string {
    return title.includes(" · ") ? title.split(" · ")[0] : title;
  }

  let hoverSnapshot = $state<ProjectSnapshot | null>(null);
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;
  let tooltipX = $state(0);
  let tooltipY = $state(0);

  function handleMouseEnter(e: MouseEvent) {
    const target = e.currentTarget as HTMLElement;
    hoverTimer = setTimeout(async () => {
      const rect = target.getBoundingClientRect();
      tooltipX = rect.left;
      tooltipY = rect.bottom + 4;
      try {
        hoverSnapshot = await getSnapshotForHover(project.path);
      } catch {
        hoverSnapshot = null;
      }
    }, 300);
  }

  function handleMouseLeave() {
    if (hoverTimer) {
      clearTimeout(hoverTimer);
      hoverTimer = null;
    }
    hoverSnapshot = null;
  }

  // Mirror of ProjectTab.svelte: force a fresh snapshot from a temp
  // repo handle so the strip is correct regardless of what the cache
  // file contains. Watcher-driven saveCurrentSnapshot writes to
  // `projectSnapshots[path]` so subsequent updates flow reactively.
  $effect(() => {
    if (!isActiveTab) void refreshProjectSnapshot(project.path);
  });

  let stripSnapshot = $derived($projectSnapshots[project.path] ?? null);
</script>

<div class="composite-tab" role="tab" tabindex="0" onmouseenter={handleMouseEnter} onmouseleave={handleMouseLeave}>
  <!-- Project segment (always first) -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="segment segment-first"
    class:active={isActiveTab && activeSegmentIndex === -1}
    class:dimmed={isActiveTab && activeSegmentIndex !== -1}
    onclick={handleProjectClick}
    onauxclick={handleProjectMiddleClick}
    onkeydown={(e) => { if (e.key === "Enter") handleProjectClick(); }}
  >
    <span class="status-dot" style="background: {statusColor}"></span>
    <span class="segment-name">{project.name}</span>
    <!-- Status chips are for monitoring OTHER repos at a glance. The
         active tab never shows them — even while a terminal/worktree
         segment has focus, the user is already "in" this repo (its
         live status lives in the status bar). -->
    {#if !isActiveTab}
      <TabStatusStrip snapshot={stripSnapshot} />
    {/if}
    <IconButton tone="danger" size="xs" icon={""} description={m.tab_close()} onclick={handleCloseProject} />
  </div>

  <!-- Dynamic segments -->
  {#each segments as segment, i (segment.type === "terminal" ? `t-${segment.info.sessionId}` : `w-${segment.path}`)}
    <div class="divider"></div>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="segment"
      class:segment-last={i === segments.length - 1}
      class:active={isActiveTab && activeSegmentIndex === i}
      class:dimmed={isActiveTab && activeSegmentIndex !== i}
      onclick={() => handleSegmentClick(i)}
      onauxclick={(e) => handleSegmentMiddleClick(e, i)}
      onkeydown={(e) => { if (e.key === "Enter") handleSegmentClick(i); }}
    >
      {#if segment.type === "worktree"}
        <span class="worktree-icon nf">{"\uE728"}</span>
        <span class="segment-name">{segment.branch}</span>
      {:else if segment.info.provider === "claude_code"}
        <svg class="brand-icon" viewBox="0 0 1200 1200" style:fill={BRAND_COLORS.anthropic}>
          <path d="M233.96 800.21L468.64 668.54l3.95-11.44-3.95-6.36h-11.44l-39.22-2.42-134.09-3.62-116.3-4.83L54.93 633.83l-26.58-5.96L0 592.75l2.74-17.48 23.84-16.03 34.15 2.98 75.46 5.15 113.24 7.81 82.15 4.83 121.69 12.65 19.33 0 2.74-7.81-6.6-4.83-5.16-4.83L346.39 495.79 219.54 411.87l-66.44-48.32-35.92-24.48-18.12-22.95-7.81-50.1 32.62-35.92 43.81 2.98 11.19 2.98 44.38 34.15 94.79 73.37 123.79 91.17 18.12 15.06 7.25-5.15.89-3.63-8.13-13.62-67.46-121.69-71.84-123.79-31.97-51.3-8.46-30.76c-2.98-12.64-5.15-23.27-5.15-36.24l37.13-50.42 20.54-6.6 49.53 6.6 20.86 18.12 30.76 70.39 49.85 110.82 77.32 150.68 22.63 44.7 12.08 41.4 4.51 12.64h7.81v-7.25l6.36-84.89 11.76-104.21 11.44-134.09 3.94-37.77 18.68-45.26 37.13-24.48 29.03 13.85 23.84 34.15-3.3 22.07-14.17 92.13-27.79 144.32-18.12 96.64h10.55l12.08-12.08 48.89-64.91 82.15-102.68 36.24-40.75 42.28-45.02 27.14-21.42h51.3l37.77 56.13-16.91 58.0-52.83 67.01-43.81 56.78-62.82 84.56-39.22 67.65 3.62 5.4 9.34-0.89 141.91-30.2 76.67-13.85 91.49-15.71 41.4 19.33 4.51 19.65-16.27 40.19-97.85 24.16-114.77 22.95-170.9 40.43-2.09 1.53 2.42 2.98 76.99 7.25 32.94 1.77 80.62 0 150.12 11.19 39.22 25.93 23.52 31.73-3.95 24.16-60.4 30.76-81.5-19.33-190.23-45.26-65.46-16.27-9.02 0v5.4l54.36 53.15 99.62 89.96 124.75 115.97 6.36 28.67-16.03 22.63-16.91-2.42-109.61-82.47-42.28-37.13-95.76-80.62h-5.66v8.46l22.07 32.3 116.54 175.16 6.04 53.72-8.46 17.48-30.2 10.55-33.18-6.04-68.21-95.76-70.39-107.84-56.78-96.64-6.93 3.95-33.5 360.89-15.71 18.44-36.24 13.85-30.2-22.95-16.03-37.13 16.03-73.37 19.33-95.76 15.7-76.11 14.17-94.55 8.46-31.41-.56-2.09-6.93.89-71.23 97.85-108.4 146.5-85.77 91.81-20.54 8.13-35.6-18.44 3.3-32.94 20.14-29.32 118.72-150.91 71.6-93.58 46.23-53.95-.32-7.81h-2.74L205.29 929.4l-56.13 7.25-24.16-22.63 2.98-37.13 11.44-12.08 94.79-65.24z"/>
        </svg>
        <span class="segment-name">{shortLabel(segment.info.title)}</span>
      {:else if segment.info.provider === "codex"}
        <svg class="brand-icon" viewBox="0 0 24 24" style:fill="var(--text-primary)">
          <path d="M22.418 9.822a5.903 5.903 0 0 0-.52-4.91 6.1 6.1 0 0 0-2.822-2.48 6.007 6.007 0 0 0-3.78-.381A6.053 6.053 0 0 0 10.868.5a6.093 6.093 0 0 0-5.788 4.143 6.033 6.033 0 0 0-4.126 2.896 6.052 6.052 0 0 0 .734 7.139 5.903 5.903 0 0 0 .52 4.911 6.1 6.1 0 0 0 2.822 2.48 6.007 6.007 0 0 0 3.78.38A6.053 6.053 0 0 0 13.132 23.5a6.093 6.093 0 0 0 5.788-4.143 6.033 6.033 0 0 0 4.126-2.896 6.052 6.052 0 0 0-.734-7.139h.106Z"/>
        </svg>
        <span class="segment-name">{shortLabel(segment.info.title)}</span>
      {:else if segment.info.provider === "open_code"}
        <svg class="brand-icon" viewBox="0 0 24 24" style:fill="var(--text-secondary)">
          <path d="M9.4 16.6L4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4Zm5.2 0L19.2 12l-4.6-4.6L16 6l6 6-6 6-1.4-1.4Z"/>
        </svg>
        <span class="segment-name">{shortLabel(segment.info.title)}</span>
      {:else}
        <span class="terminal-icon">{"\uF489"}</span>
        <span class="segment-name">{shortLabel(segment.info.title)}</span>
      {/if}
      <IconButton tone="danger" size="xs" icon={""} description={m.tab_close()} onclick={(e) => handleCloseSegment(e, i)} />
    </div>
  {/each}
  {#if hoverSnapshot}
    <TabTooltip snapshot={hoverSnapshot} x={tooltipX} y={tooltipY} />
  {/if}
</div>

<style>
  .composite-tab {
    display: flex;
    align-items: center;
    height: 28px;
    border-radius: 14px;
    overflow: visible;
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    flex-shrink: 0;
    user-select: none;
    position: relative;
  }

  .segment-first {
    border-radius: 14px 0 0 14px;
  }

  .segment-last {
    border-radius: 0 14px 14px 0;
  }

  /* When there are no dynamic segments, project is also last */
  .segment-first:last-of-type {
    border-radius: 14px;
  }

  .segment {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 10px;
    height: 100%;
    cursor: pointer;
    transition: background 0.15s, opacity 0.15s;
  }

  .segment.active {
    background: color-mix(in srgb, var(--text-primary) 12%, transparent);
  }

  .segment.dimmed {
    opacity: 0.5;
  }

  .segment:not(.active):not(.dimmed) {
    opacity: 0.7;
  }

  .segment:hover {
    opacity: 1;
  }

  .divider {
    width: 1px;
    height: 100%;
    background: var(--accent-primary);
    opacity: 0.3;
    flex-shrink: 0;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .terminal-icon {
    font-family: var(--font-icons);
    font-size: var(--font-size-sm);
    color: var(--accent-purple);
    flex-shrink: 0;
  }

  .worktree-icon {
    font-size: var(--font-size-sm);
    color: var(--accent-cyan, var(--accent-primary));
    flex-shrink: 0;
  }

  .brand-icon {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }

  .segment-name {
    flex: 1;
    min-width: 0;
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
  }


</style>
