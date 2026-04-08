<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import TabBar from "$lib/components/layout/TabBar.svelte";
  import Sidebar from "$lib/components/layout/Sidebar.svelte";
  import StatusBar from "$lib/components/layout/StatusBar.svelte";
  import GitGraph from "$lib/components/graph/GitGraph.svelte";
  import CommitDetail from "$lib/components/detail/CommitDetail.svelte";
  import StagingArea from "$lib/components/changes/StagingArea.svelte";
  import DiffEditor from "$lib/components/editor/DiffEditor.svelte";
  import StagingDiffEditor from "$lib/components/editor/StagingDiffEditor.svelte";
  import SettingsPage from "$lib/components/settings/SettingsPage.svelte";
  import PipelineList from "$lib/components/pipeline/PipelineList.svelte";
  import PipelineDetail from "$lib/components/pipeline/PipelineDetail.svelte";
  import JobLog from "$lib/components/pipeline/JobLog.svelte";
  import { repoInfo, isLoading, error } from "$lib/stores/repo";
  import { selectedCommit, selectedOid, selectedCommitFiles, openFileDiff, navigateToCommit, graphNavigateDown, graphNavigateUp, graphNavigateFirst, graphNavigateLast } from "$lib/stores/graph";
  import type { RawDiffContent } from "$lib/stores/graph";
  import { selectedCiRun, jobLog } from "$lib/stores/provider";
  import type { ThemeData } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import TaskPopover from "$lib/components/tasks/TaskPopover.svelte";
  import TaskPanel from "$lib/components/tasks/TaskPanel.svelte";
  import { panelMode } from "$lib/stores/tasks";
  import { initProjects, openFolderAsProject, activeProject, switchToNextTab, switchToPrevTab, closeActiveTab } from "$lib/stores/projects";
  import StashView from "$lib/components/stash/StashView.svelte";
  import ConflictToolbar from "$lib/components/conflict/ConflictToolbar.svelte";
  import TagView from "$lib/components/tags/TagView.svelte";
  import BranchView from "$lib/components/branches/BranchView.svelte";
  import WorktreeList from "$lib/components/worktrees/WorktreeList.svelte";
  import BlameView from "$lib/components/blame/BlameView.svelte";
  import { branchFileDiff, branchSelectedCommit, branchSelectedFiles, closeBranchCommitDetail } from "$lib/stores/branches";
  import { blamePreviousView } from "$lib/stores/blame";
  import { getFileAtCommit, getFileIndex, getFileWorkdir } from "$lib/api/tauri";
  import { unstagedDiffs, stagedDiffs } from "$lib/stores/changes";
  import type { FileDiff } from "$lib/types";
  import { fileDiffPanel, loadingFileDiff, closeFileDiff } from "$lib/stores/graph";
  import { activeTheme, applyTheme, listenThemeChanges, initTheme } from "$lib/stores/theme";
  import { registerShortcuts, unregisterShortcuts, toggleCheatSheet } from "$lib/stores/shortcuts";
  import { expandPanel as expandTaskPanel } from "$lib/stores/tasks";
  import { refreshStatuses } from "$lib/stores/changes";
  import * as api from "$lib/api/tauri";
  import { get } from "svelte/store";
  import ShortcutOverlay from "$lib/components/common/ShortcutOverlay.svelte";

  let activeView = $state("graph");
  let selectedDiff = $state<RawDiffContent | null>(null);
  let selectedStagingFile = $state<{ filename: string; isStaged: boolean } | null>(null);

  /** Look up the FileDiff for the currently selected staging file from stores. */
  let selectedStagingDiff = $derived.by<FileDiff | null>(() => {
    if (!selectedStagingFile) return null;
    const diffs = selectedStagingFile.isStaged ? $stagedDiffs : $unstagedDiffs;
    return diffs.find(d => d.path === selectedStagingFile!.filename) ?? null;
  });
  let showJobLog = $state(false);
  let registeredShortcutIds: string[] = [];
  let diffPanelHeight = $state(250);
  let pipelineSidebarWidth = $state(360);
  let taskPanelHeight = $state(200);

  function startResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = pipelineSidebarWidth;

    function onMouseMove(e: MouseEvent) {
      const delta = e.clientX - startX;
      const minW = Math.max(280, window.innerWidth * 0.2);
      const maxW = Math.min(600, window.innerWidth * 0.5);
      pipelineSidebarWidth = Math.max(minW, Math.min(maxW, startWidth + delta));
    }

    function onMouseUp() {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  function startDiffResize(e: MouseEvent) {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = diffPanelHeight;

    function onMouseMove(e: MouseEvent) {
      const delta = startY - e.clientY; // dragging up increases height
      const container = document.querySelector('.graph-with-diff') as HTMLElement;
      const maxH = container ? container.clientHeight * 0.6 : 500;
      diffPanelHeight = Math.max(150, Math.min(maxH, startHeight + delta));
    }

    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    }

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  function startTaskPanelResize(e: MouseEvent) {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = taskPanelHeight;

    function onMouseMove(e: MouseEvent) {
      // Dragging up (negative delta) increases height; dragging down decreases it.
      const delta = startY - e.clientY;
      const minH = Math.max(100, window.innerHeight * 0.1);
      const maxH = Math.min(400, window.innerHeight * 0.4);
      taskPanelHeight = Math.max(minH, Math.min(maxH, startHeight + delta));
    }

    function onMouseUp() {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  onMount(async () => {
    // Initialize theme
    try {
      const theme = await invoke<ThemeData>("resolve_startup_theme");
      activeTheme.set(theme);
      applyTheme(theme);
    } catch (e) {
      console.warn("resolve_startup_theme failed, trying fallback:", e);
      try {
        await initTheme("github-dark");
      } catch (e2) {
        console.error("Theme fallback also failed:", e2);
      }
    }
    await listenThemeChanges();

    initProjects();

    // --- Keyboard shortcuts ---
    const viewMap = ["graph", "changes", "branches", "tags", "stashes", "worktrees"];
    const navShortcuts = viewMap.map((view, i) => ({
      id: `nav.${view}`,
      keys: { mod: true, key: String(i + 1) },
      label: `Go to ${view.charAt(0).toUpperCase() + view.slice(1)}`,
      category: "Navigation",
      action: () => handleNavigate(view),
    }));
    navShortcuts.push({
      id: "nav.settings",
      keys: { mod: true, key: "," },
      label: m.sidebar_settings(),
      category: "Navigation",
      action: () => handleNavigate("settings"),
    });

    const tabShortcuts = [
      {
        id: "tab.next",
        keys: { mod: true, key: "Tab" },
        label: "Next tab",
        category: "Tabs",
        action: () => { switchToNextTab(); },
      },
      {
        id: "tab.prev",
        keys: { mod: true, shift: true, key: "Tab" },
        label: "Previous tab",
        category: "Tabs",
        action: () => { switchToPrevTab(); },
      },
      {
        id: "tab.close",
        keys: { mod: true, key: "w" },
        label: m.tab_close(),
        category: "Tabs",
        action: () => { closeActiveTab(); },
      },
    ];

    const gitShortcuts = [
      {
        id: "git.fetch",
        keys: { mod: true, shift: true, key: "F" },
        label: m.toolbar_fetch(),
        category: "Git",
        action: async () => {
          try { await api.fetchRemote("origin"); } catch { /* toolbar shows error */ }
        },
      },
      {
        id: "git.pull",
        keys: { mod: true, shift: true, key: "L" },
        label: m.toolbar_pull(),
        category: "Git",
        action: async () => {
          const info = get(repoInfo);
          if (!info?.head_branch) return;
          try { await api.pullRemote("origin", info.head_branch); } catch { /* handled */ }
        },
      },
      {
        id: "git.push",
        keys: { mod: true, shift: true, key: "P" },
        label: m.toolbar_push(),
        category: "Git",
        action: async () => {
          const info = get(repoInfo);
          if (!info?.head_branch) return;
          try { await api.pushRemote("origin", info.head_branch); } catch { /* handled */ }
        },
      },
      {
        id: "git.stageAll",
        keys: { mod: true, shift: true, key: "S" },
        label: m.changes_stage_all(),
        category: "Git",
        action: async () => {
          try { await api.stageAll(); await refreshStatuses(); } catch { /* handled */ }
        },
      },
      {
        id: "git.unstageAll",
        keys: { mod: true, shift: true, key: "U" },
        label: m.changes_unstage_all(),
        category: "Git",
        action: async () => {
          try { await api.unstageAll(); await refreshStatuses(); } catch { /* handled */ }
        },
      },
    ];

    const graphShortcuts = [
      {
        id: "graph.down",
        keys: { key: "j" },
        label: "Next commit",
        category: "Graph",
        action: () => graphNavigateDown(),
      },
      {
        id: "graph.up",
        keys: { key: "k" },
        label: "Previous commit",
        category: "Graph",
        action: () => graphNavigateUp(),
      },
      {
        id: "graph.first",
        keys: { key: "Home" },
        label: "First commit",
        category: "Graph",
        action: () => graphNavigateFirst(),
      },
      {
        id: "graph.last",
        keys: { key: "End" },
        label: "Last commit",
        category: "Graph",
        action: () => graphNavigateLast(),
      },
      {
        id: "graph.search",
        keys: { key: "/" },
        label: "Search commits",
        category: "Graph",
        action: () => {
          document.querySelector<HTMLInputElement>('.search-bar input')?.focus();
        },
      },
      {
        id: "graph.searchMod",
        keys: { mod: true, key: "f" },
        label: "Search commits",
        category: "Graph",
        action: () => {
          document.querySelector<HTMLInputElement>('.search-bar input')?.focus();
        },
      },
    ];

    const utilShortcuts = [
      {
        id: "util.cheatSheet",
        keys: { key: "?" },
        label: "Show keyboard shortcuts",
        category: "General",
        action: () => toggleCheatSheet(),
      },
      {
        id: "util.tasks",
        keys: { mod: true, shift: true, key: "T" },
        label: m.tasks_title(),
        category: "General",
        action: () => expandTaskPanel(),
      },
    ];

    const allShortcutIds = [
      ...navShortcuts,
      ...tabShortcuts,
      ...gitShortcuts,
      ...graphShortcuts,
      ...utilShortcuts,
    ];
    registerShortcuts(allShortcutIds);
    registeredShortcutIds = allShortcutIds.map((s) => s.id);
  });

  onDestroy(() => {
    if (registeredShortcutIds.length > 0) {
      unregisterShortcuts(registeredShortcutIds);
    }
  });

  function handleNavigate(view: string) {
    // Store the previous view before switching so blame can navigate back.
    if (view === "blame") {
      blamePreviousView.set(activeView);
    }
    activeView = view;
    selectedDiff = null;
    selectedStagingFile = null;
    showJobLog = false;
  }

  function handleJobSelect(_jobId: number) {
    showJobLog = true;
  }

  function handleFileClick(path: string, staged: boolean) {
    selectedStagingFile = { filename: path, isStaged: staged };
  }

  async function handleBranchFileClick(path: string) {
    const commit = $branchSelectedCommit;
    if (!commit) return;
    const parentOid = commit.parents?.[0] ?? null;
    try {
      const [oldContent, newContent] = await Promise.all([
        parentOid ? getFileAtCommit(parentOid, path).catch(() => "") : Promise.resolve(""),
        getFileAtCommit(commit.oid, path).catch(() => ""),
      ]);
      branchFileDiff.set({ oldContent, newContent, filename: path });
    } catch {
      branchFileDiff.set(null);
    }
  }
</script>

<div class="app-shell">
  <TabBar />

  <div class="main-area">
    <Sidebar onNavigate={handleNavigate} />

    <div class="center-panel">
      <ConflictToolbar />
      {#if activeView === "settings"}
        <SettingsPage />
      {:else if activeView === "pipelines"}
        <div class="pipelines-layout">
          <div class="pipelines-sidebar" style="width: {pipelineSidebarWidth}px">
            <PipelineList />
          </div>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="resize-handle" onmousedown={startResize}></div>
          <div class="pipelines-main">
            {#if showJobLog && $jobLog}
              <div class="pipelines-detail">
                <PipelineDetail onSelectJob={handleJobSelect} />
              </div>
              <div class="pipelines-log">
                <JobLog />
              </div>
            {:else}
              <PipelineDetail onSelectJob={handleJobSelect} />
            {/if}
          </div>
        </div>
      {:else if activeView === "stashes"}
        <StashView />
      {:else if activeView === "tags"}
        <TagView />
      {:else if activeView === "branches"}
        <div class="branch-layout">
          <div class="branch-main">
            <BranchView />
          </div>
          {#if $branchFileDiff}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="diff-resize-handle" onmousedown={startDiffResize}></div>
            <div class="diff-panel" style="height: {diffPanelHeight}px">
              <DiffEditor
                oldContent={$branchFileDiff.oldContent}
                newContent={$branchFileDiff.newContent}
                filename={$branchFileDiff.filename}
                editorTheme={$activeTheme?.editor}
                isDark={$activeTheme?.meta.mode !== 'light'}
                onClose={() => branchFileDiff.set(null)}
              />
            </div>
          {/if}
        </div>
      {:else if activeView === "worktrees"}
        <WorktreeList />
      {:else if activeView === "blame"}
        <BlameView onNavigateBack={(view) => { activeView = view; }} />
      {:else if activeView === "merge-requests"}
        <div class="wip-placeholder">
          <div class="wip-icon">🚧</div>
          <h3>{m.app_wip_title()}</h3>
          <p>{m.app_wip_message({ view: activeView })}</p>
        </div>
      {:else if $isLoading}
        <div class="welcome-screen">
          <div class="spinner spinner--large"></div>
          <div class="loading-text">{m.app_opening_repo()}</div>
        </div>
      {:else if $error}
        <div class="welcome-screen">
          <div class="error-text">Error: {$error}</div>
          <button class="open-btn" onclick={openFolderAsProject}>{m.app_try_again()}</button>
        </div>
      {:else if $repoInfo}
        {#if activeView === "changes"}
          <div class="changes-layout">
            <div class="changes-sidebar">
              <StagingArea onFileClick={handleFileClick} onNavigate={handleNavigate} />
            </div>
            <div class="changes-diff">
              {#if selectedStagingDiff && selectedStagingFile}
                <StagingDiffEditor
                  diff={selectedStagingDiff}
                  isStaged={selectedStagingFile.isStaged}
                  filename={selectedStagingFile.filename}
                  onClose={() => { selectedStagingFile = null; }}
                />
              {:else}
                <div class="no-diff">
                  <p>{m.diff_empty()}</p>
                </div>
              {/if}
            </div>
          </div>
        {:else}
          {#if $fileDiffPanel}
            <div class="graph-with-diff">
              <div class="graph-area">
                <GitGraph />
              </div>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="diff-resize-handle" onmousedown={startDiffResize}></div>
              <div class="diff-panel" style="height: {diffPanelHeight}px">
                <DiffEditor
                  oldContent={$fileDiffPanel.oldContent}
                  newContent={$fileDiffPanel.newContent}
                  filename={$fileDiffPanel.filename}
                  editorTheme={$activeTheme?.editor}
                  isDark={$activeTheme?.meta.mode !== 'light'}
                  onClose={closeFileDiff}
                />
              </div>
            </div>
          {:else if $loadingFileDiff}
            <div class="graph-with-diff">
              <div class="graph-area">
                <GitGraph />
              </div>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="diff-resize-handle" onmousedown={startDiffResize}></div>
              <div class="diff-panel diff-panel-loading" style="height: {diffPanelHeight}px">
                <div class="spinner"></div>
              </div>
            </div>
          {:else}
            <GitGraph />
          {/if}
        {/if}
      {:else}
        <div class="welcome-screen">
          <div class="welcome-icon">{"\uE702"}</div>
          <h2 class="welcome-title">{m.app_title()}</h2>
          <p class="welcome-subtitle">{m.app_welcome_subtitle()}</p>
          <button class="open-btn" onclick={openFolderAsProject}>{m.app_open_repo()}</button>
        </div>
      {/if}
      {#if $panelMode === "panel"}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="task-panel-resize-handle" onmousedown={startTaskPanelResize}></div>
        <div class="task-panel-container" style="height: {taskPanelHeight}px;">
          <TaskPanel />
        </div>
      {/if}
    </div>

    {#if activeView === "graph" && $selectedCommit}
      <div class="graph-detail-sidebar">
        <CommitDetail
          commit={$selectedCommit}
          files={$selectedCommitFiles}
          onClose={() => {
            selectedOid.set(null);
            selectedCommit.set(null);
            selectedCommitFiles.set([]);
          }}
          onFileClick={(path) => openFileDiff($selectedCommit!.oid, path)}
          onNavigateToGraph={(oid) => navigateToCommit(oid)}
          onNavigate={handleNavigate}
        />
      </div>
    {/if}

    {#if activeView === "branches" && $branchSelectedCommit}
      <div class="graph-detail-sidebar">
        <CommitDetail
          commit={$branchSelectedCommit}
          files={$branchSelectedFiles}
          showNavigateToGraph={true}
          onNavigateToGraph={(oid) => navigateToCommit(oid)}
          onClose={() => closeBranchCommitDetail()}
          onFileClick={handleBranchFileClick}
          onNavigate={handleNavigate}
        />
      </div>
    {/if}
  </div>

  {#if $panelMode === "popover"}
    <TaskPopover />
  {/if}

  <ShortcutOverlay />
  <StatusBar />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .main-area {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .center-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .welcome-screen {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }

  .welcome-icon {
    font-size: 48px;
    font-family: var(--font-icons);
    color: var(--accent-blue);
    opacity: 0.5;
  }

  .welcome-title {
    font-size: 20px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .welcome-subtitle {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .open-btn {
    margin-top: 8px;
    padding: 8px 24px;
    background: var(--accent-blue);
    color: #ffffff;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .open-btn:hover {
    opacity: 0.9;
  }

  .loading-text {
    font-size: 14px;
    color: var(--text-secondary);
  }

  .error-text {
    font-size: 13px;
    color: var(--accent-orange);
    max-width: 400px;
    text-align: center;
    word-break: break-word;
  }

  .changes-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .changes-sidebar {
    width: clamp(240px, 22vw, 360px);
    min-width: 0;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .changes-diff {
    flex: 1;
    overflow: hidden;
  }

  .pipelines-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .pipelines-sidebar {
    min-width: min(280px, 30vw);
    max-width: min(600px, 50vw);
    border-right: 1px solid var(--border);
    overflow: hidden;
    flex-shrink: 0;
  }

  .resize-handle {
    width: 4px;
    cursor: col-resize;
    background: transparent;
    transition: background 0.15s;
    flex-shrink: 0;
  }

  .resize-handle:hover {
    background: var(--accent-blue);
  }

  .pipelines-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .pipelines-detail {
    flex: 0 0 auto;
    max-height: 45%;
    overflow: auto;
    border-bottom: 1px solid var(--border);
  }

  .pipelines-log {
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }

  .wip-placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-secondary);
  }

  .wip-icon {
    font-size: 48px;
    opacity: 0.5;
  }

  .wip-placeholder h3 {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .wip-placeholder p {
    font-size: 13px;
    margin: 0;
  }

  .task-panel-resize-handle {
    height: 4px;
    background: var(--border);
    cursor: ns-resize;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .task-panel-resize-handle:hover {
    background: var(--accent-blue);
  }

  .task-panel-container {
    flex-shrink: 0;
    overflow: hidden;
    border-top: 1px solid var(--border);
  }

  .branch-layout {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .branch-main {
    flex: 1;
    overflow: hidden;
    display: flex;
  }

  .graph-detail-sidebar {
    width: clamp(260px, 22vw, 360px);
    flex-shrink: 0;
    display: flex;
  }

  .graph-with-diff {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .graph-area {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .diff-resize-handle {
    height: 4px;
    cursor: row-resize;
    background: transparent;
    transition: background 0.15s;
    flex-shrink: 0;
    border-top: 1px solid var(--border);
  }

  .diff-resize-handle:hover {
    background: var(--accent-blue);
  }

  .diff-panel {
    flex-shrink: 0;
    overflow: hidden;
  }

  .diff-panel-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    border-top: 1px solid var(--border);
  }

  .no-diff {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
