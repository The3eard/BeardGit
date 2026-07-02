<!--
  CompareView — compare any ref against any ref (spec 10).

  Layout mirrors the PR-detail structure users already know: a header with two
  ref pickers (branch/tag/SHA autocomplete) + swap + dot-mode toggle, an
  "N ahead / M behind" summary, the ahead commit list (windowed with "Load
  more"), the changed-file list, and a per-file diff via the shared DiffEditor
  (which inherits the binary / too-large placeholder states).

  Read-only: nothing here mutates the repo. Reachable from the graph/branches
  context menus and the command palette — not the sidebar.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { getBranches, listTags } from "$lib/api/tauri";
  import { activeTheme } from "$lib/stores/theme";
  import { formatDateTime } from "$lib/utils/time";
  import { shortOid } from "$lib/utils/git";
  import { IconButton, Button, Skeleton } from "$lib/components/ui";
  import EmptyState from "../common/EmptyState.svelte";
  import FileChangeList from "../common/FileChangeList.svelte";
  import DiffEditor from "../editor/DiffEditor.svelte";
  import ResizableDiffPanel from "../editor/ResizableDiffPanel.svelte";
  import RefPicker, { type RefOption } from "./RefPicker.svelte";
  import {
    compareRefA,
    compareRefB,
    compareMode,
    compareMergeBase,
    compareCommits,
    compareBehindCount,
    compareCommitsCapped,
    compareLoadingMore,
    compareFiles,
    compareLoading,
    compareError,
    compareSelectedFilePath,
    compareOpenDiff,
    compareLoadingDiff,
    compareDiffError,
    setCompareRefA,
    setCompareRefB,
    swapCompareRefs,
    setCompareMode,
    loadMoreCompareCommits,
    openCompareFileDiff,
    closeCompareFileDiff,
    runCompare,
  } from "$lib/stores/compare";

  let refOptions = $state<RefOption[]>([]);

  onMount(async () => {
    // Load branches + tags for the pickers' autocomplete.
    const [branches, tags] = await Promise.all([
      getBranches().catch(() => []),
      listTags().catch(() => []),
    ]);
    refOptions = [
      ...branches.map((b) => ({ name: b.name, kind: "branch" as const })),
      ...tags.map((t) => ({ name: t.name, kind: "tag" as const })),
    ];
    // If entry points pre-filled both refs but the data wasn't loaded yet,
    // ensure a comparison is running.
    if ($compareRefA && $compareRefB && $compareFiles.length === 0 && !$compareLoading) {
      void runCompare();
    }
  });

  let bothRefsSet = $derived(!!$compareRefA && !!$compareRefB);
  let aheadLabel = $derived(`${$compareCommits.length}${$compareCommitsCapped ? "+" : ""}`);
</script>

<div class="compare-view">
  <!-- Header: ref pickers + swap + dot-mode toggle -->
  <div class="compare-header">
    <div class="ref-row">
      <RefPicker
        label={m.compare_base()}
        value={$compareRefA}
        options={refOptions}
        placeholder={m.compare_pick_ref()}
        onSelect={setCompareRefA}
      />
      <IconButton
        icon={"\uF0EC"}
        description={m.compare_swap()}
        tone="default"
        onclick={swapCompareRefs}
      />
      <RefPicker
        label={m.compare_compare()}
        value={$compareRefB}
        options={refOptions}
        placeholder={m.compare_pick_ref()}
        onSelect={setCompareRefB}
      />
    </div>

    <div class="mode-toggle" role="group" aria-label={m.compare_mode()}>
      <button
        class="mode-btn"
        class:active={$compareMode === "three-dot"}
        onclick={() => setCompareMode("three-dot")}
        title={m.compare_mode_three_dot_tooltip()}
      >
        {m.compare_mode_three_dot()}
      </button>
      <button
        class="mode-btn"
        class:active={$compareMode === "two-dot"}
        onclick={() => setCompareMode("two-dot")}
        title={m.compare_mode_two_dot_tooltip()}
      >
        {m.compare_mode_two_dot()}
      </button>
    </div>
  </div>

  {#if !bothRefsSet}
    <EmptyState
      fill
      icon={"\uF126"}
      title={m.compare_empty_title()}
      description={m.compare_empty_description()}
    />
  {:else if $compareLoading}
    <div class="compare-body">
      <Skeleton rows={6} />
    </div>
  {:else if $compareError}
    <EmptyState
      fill
      icon={"\uF071"}
      title={m.compare_error_title()}
      description={$compareError}
    />
  {:else}
    <!-- Summary chips -->
    <div class="summary">
      <span class="chip chip--ahead">{m.compare_ahead({ count: aheadLabel })}</span>
      <span class="chip chip--behind">{m.compare_behind({ count: String($compareBehindCount) })}</span>
      {#if $compareMode === "three-dot" && $compareMergeBase}
        <span class="chip chip--base" title={$compareMergeBase}>
          {m.compare_merge_base({ sha: shortOid($compareMergeBase) })}
        </span>
      {/if}
    </div>

    <div class="compare-body">
      <!-- Ahead commit list (windowed) -->
      <section class="section">
        <h4 class="section-title">{m.compare_commits_title({ count: aheadLabel })}</h4>
        {#if $compareCommits.length === 0}
          <p class="empty-section">{m.compare_no_commits()}</p>
        {:else}
          <ul class="commit-list">
            {#each $compareCommits as commit (commit.oid)}
              <li class="commit-row">
                <span class="commit-sha">{shortOid(commit.oid)}</span>
                <span class="commit-summary">{commit.summary}</span>
                <span class="commit-meta">{commit.author} · {formatDateTime(commit.timestamp)}</span>
              </li>
            {/each}
          </ul>
          {#if $compareCommitsCapped}
            <Button
              variant="neutral"
              size="sm"
              loading={$compareLoadingMore}
              onclick={loadMoreCompareCommits}
            >
              {m.compare_load_more()}
            </Button>
          {/if}
        {/if}
      </section>

      <!-- Changed files -->
      <section class="section">
        <h4 class="section-title">{m.compare_files_title({ count: String($compareFiles.length) })}</h4>
        {#if $compareFiles.length === 0}
          <p class="empty-section">{m.compare_no_files()}</p>
        {:else}
          <FileChangeList files={$compareFiles} onSelect={(p) => openCompareFileDiff(p)} />
        {/if}
      </section>
    </div>

    <!-- Per-file diff panel -->
    {#if $compareOpenDiff || $compareLoadingDiff || $compareDiffError}
      <ResizableDiffPanel loading={$compareLoadingDiff}>
        {#if $compareLoadingDiff}
          <div class="spinner"></div>
        {:else if $compareDiffError}
          <EmptyState fill icon={"\uF071"} title={m.compare_diff_error()} description={$compareDiffError} />
        {:else if $compareOpenDiff}
          <DiffEditor
            oldContent={$compareOpenDiff.oldContent}
            newContent={$compareOpenDiff.newContent}
            filename={$compareOpenDiff.filename}
            placeholder={$compareOpenDiff.placeholder}
            editorTheme={$activeTheme?.editor}
            isDark={$activeTheme?.meta.mode !== "light"}
            onClose={closeCompareFileDiff}
          >
            {#snippet toolbar()}
              <span class="diff-filename">{$compareSelectedFilePath ?? ""}</span>
              <button class="diff-close" aria-label={m.tooltip_close()} onclick={closeCompareFileDiff}>&#xF00D;</button>
            {/snippet}
          </DiffEditor>
        {/if}
      </ResizableDiffPanel>
    {/if}
  {/if}
</div>

<style>
  .compare-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-primary);
    overflow: hidden;
  }

  .compare-header {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .ref-row {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }

  .mode-toggle {
    display: flex;
    align-self: flex-start;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .mode-btn {
    padding: 4px 12px;
    background: var(--bg-secondary);
    border: none;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    cursor: pointer;
  }
  .mode-btn + .mode-btn {
    border-left: 1px solid var(--border);
  }
  .mode-btn:hover {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  }
  .mode-btn.active {
    background: var(--accent-primary);
    color: var(--bg-primary);
    font-weight: 600;
  }

  .summary {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .chip {
    padding: 2px 8px;
    border-radius: 3px;
    font-size: var(--font-size-xs);
    font-weight: 600;
  }
  .chip--ahead {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }
  .chip--behind {
    background: color-mix(in srgb, var(--accent-orange) 15%, transparent);
    color: var(--accent-orange);
  }
  .chip--base {
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }

  .compare-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px 16px;
  }

  .section {
    margin-bottom: 16px;
  }

  .section-title {
    margin: 0 0 8px;
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .empty-section {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-style: italic;
  }

  .commit-list {
    list-style: none;
    margin: 0 0 8px;
    padding: 0;
  }

  .commit-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 3px 0;
    font-size: var(--font-size-sm);
    border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }

  .commit-sha {
    font-family: var(--font-mono);
    color: var(--accent-orange);
    flex-shrink: 0;
  }

  .commit-summary {
    flex: 1;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .commit-meta {
    color: var(--text-secondary);
    font-size: var(--font-size-xs);
    flex-shrink: 0;
  }

  .spinner {
    margin: auto;
    width: 24px;
    height: 24px;
    border: 3px solid var(--border);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: compare-spin 0.8s linear infinite;
  }
  @keyframes compare-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .diff-filename {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--font-icon, monospace);
    font-size: var(--font-size-md);
  }
  .diff-close:hover {
    color: var(--text-primary);
  }
</style>
