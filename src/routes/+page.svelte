<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import TabBar from "$lib/components/layout/TabBar.svelte";
  import Sidebar from "$lib/components/layout/Sidebar.svelte";
  import StatusBar from "$lib/components/layout/StatusBar.svelte";
  import GitGraph from "$lib/components/graph/GitGraph.svelte";
  import CommitDetail from "$lib/components/detail/CommitDetail.svelte";
  import StagingArea from "$lib/components/changes/StagingArea.svelte";
  import DiffEditor from "$lib/components/editor/DiffEditor.svelte";
  import StagingDiffEditor from "$lib/components/editor/StagingDiffEditor.svelte";
  import SettingsPage from "$lib/components/settings/SettingsPage.svelte";
  import PipelineView from "$lib/components/pipeline/PipelineView.svelte";
  import { repoInfo, isLoading, error } from "$lib/stores/repo";
  import { selectedCommit, selectedOid, selectedCommitFiles, openFileDiff, navigateToCommit, graphNavigateDown, graphNavigateUp, graphNavigateFirst, graphNavigateLast } from "$lib/stores/graph";
  import type { RawDiffContent } from "$lib/stores/graph";
  import * as m from "$lib/paraglide/messages";
  import TasksPopover from "$lib/components/tasks/TasksPopover.svelte";
  import {
    tasksPopoverOpen,
    toggleTasksPopover,
    openTasksPopover,
    closeTasksPopover,
  } from "$lib/stores/tasksPopover";
  import { initProjects, openFolderAsProject, activeProject, switchToNextTab, switchToPrevTab, closeActiveTab, switchToTab, onProjectSwitch } from "$lib/stores/projects";
  import StashView from "$lib/components/stash/StashView.svelte";
  import ConflictToolbar from "$lib/components/conflict/ConflictToolbar.svelte";
  import TagView from "$lib/components/tags/TagView.svelte";
  import BranchView from "$lib/components/branches/BranchView.svelte";
  import WorktreeList from "$lib/components/worktrees/WorktreeList.svelte";
  import SubmoduleList from "$lib/components/submodules/SubmoduleList.svelte";
  import BlameView from "$lib/components/blame/BlameView.svelte";
  import MrPrView from "$lib/components/mr-pr/MrPrView.svelte";
  import IssueView from "$lib/components/issues/IssueView.svelte";
  import ReleaseView from "$lib/components/releases/ReleaseView.svelte";
  import { activeViewStore, installProviderDisconnectReroute } from "$lib/stores/navigation";
  import { branchFileDiff, branchSelectedCommit, branchSelectedFiles, closeBranchCommitDetail } from "$lib/stores/branches";
  import { blamePreviousView } from "$lib/stores/blame";
  import TerminalView from "$lib/components/terminal/TerminalView.svelte";
  import { initTerminalEvents } from "$lib/stores/terminal";
  import { activeTab, activeTabIndex, findLastProjectTabIndex, openTerminalTab, switchSegment, openTabs, getActiveTerminalSegment, getCompositeTerminals } from "$lib/stores/tabs";
  import { getSidebarCollapsed, setSidebarCollapsed, resolveStartupTheme } from "$lib/api/tauri";
  import { loadSidebarLayout } from "$lib/stores/sidebarLayout";
  import ReflogView from "$lib/components/reflog/ReflogView.svelte";
  import AiConfigEditor from "$lib/components/ai-config/AiConfigEditor.svelte";
  import AiSessionsView from "$lib/components/ai-sessions/AiSessionsView.svelte";
  import BisectWorkflow from "$lib/components/bisect/BisectWorkflow.svelte";
  import ContextMenu from "$lib/components/common/ContextMenu.svelte";
  import type { MenuItem } from "$lib/components/common/ContextMenu.svelte";
  import {
    loadReflog,
    clearReflogSelection,
    reflogFileDiff,
    selectedReflogEntry as selectedReflogEntryStore,
  } from "$lib/stores/reflog";
  import type { ReflogEntry } from "$lib/types";
  import { getFileAtCommitText as getFileAtCommit, getFileIndex, getFileWorkdir } from "$lib/api/tauri";
  import { shortOid } from "$lib/utils/git";
  import * as api from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import { unstagedDiffs, stagedDiffs } from "$lib/stores/changes";
  import type { FileDiff } from "$lib/types";
  import { fileDiffPanel, loadingFileDiff, closeFileDiff } from "$lib/stores/graph";
  import { activeTheme, applyTheme, listenThemeChanges, initTheme } from "$lib/stores/theme";
  import { registerShortcuts, unregisterShortcuts, toggleCheatSheet } from "$lib/stores/shortcuts";
  import { addToast } from "$lib/stores/toast";
  import { refreshStatuses, refreshDiffs } from "$lib/stores/changes";
  import { get } from "svelte/store";
  import ShortcutOverlay from "$lib/components/common/ShortcutOverlay.svelte";
  import { detectAiProviders, loadPreferredProvider } from "$lib/stores/ai";
  import CreateBackgroundRunDialog from "$lib/components/ai/CreateBackgroundRunDialog.svelte";
  import RepoConfigPage from "$lib/components/repo-config/RepoConfigPage.svelte";
  import { initRepoConfigRouteSync } from "$lib/stores/repoConfigRoute";
  import { startAiBackgroundListeners, refreshAiBackgroundRuns, openCreateBackgroundRunDialogRequest } from "$lib/stores/aiBackground";
  import { startConversationListeners, stopConversationListeners } from "$lib/stores/aiConversations";
  import { createBranchDialog, openCreateBranchDialog, closeCreateBranchDialog } from "$lib/stores/createBranchDialog";
  import CreateBranchDialog from "$lib/components/branches/CreateBranchDialog.svelte";
  import {
    prFileDiff,
    loadingPrFileDiff,
    prFileDiffError,
    loadPrFileDiff,
    closePrFileDiff,
    mrPrDetail,
    mrPrDiffFiles,
    selectedPrFilePath,
    postReviewComment,
    resolveDiscussion,
    unresolveDiscussion,
    registerPrDiffShortcuts,
    unregisterPrDiffShortcuts,
  } from "$lib/stores/mr-pr";
  import type { DiffCommentsLayerProps } from "$lib/components/editor/diff-comments-layer";

  let activeView = $state("graph");
  let repoConfigPageRef = $state<RepoConfigPage | undefined>(undefined);
  let teardownRepoConfigRoute: (() => void) | null = null;
  let teardownProviderReroute: (() => void) | null = null;
  let showAiBackgroundDialog = $state(false);

  // Open the dialog whenever any entry point pings the shared signal store.
  let lastDialogRequest = 0;
  $effect(() => {
    const next = $openCreateBackgroundRunDialogRequest;
    if (next !== lastDialogRequest) {
      lastDialogRequest = next;
      if (next > 0) showAiBackgroundDialog = true;
    }
  });
  let selectedDiff = $state<RawDiffContent | null>(null);
  let selectedStagingFile = $state<{ filename: string; isStaged: boolean } | null>(null);

  /** Look up the FileDiff for the currently selected staging file from stores. */
  let selectedStagingDiff = $derived.by<FileDiff | null>(() => {
    if (!selectedStagingFile) return null;
    const diffs = selectedStagingFile.isStaged ? $stagedDiffs : $unstagedDiffs;
    return diffs.find(d => d.path === selectedStagingFile!.filename) ?? null;
  });
  let registeredShortcutIds: string[] = [];
  let diffPanelHeight = $state(250);
  let changesSidebarWidth = $state(320);
  let sidebarCollapsed = $state(false);

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

  function startChangesSidebarResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = changesSidebarWidth;

    function onMouseMove(e: MouseEvent) {
      const delta = e.clientX - startX;
      changesSidebarWidth = Math.max(240, Math.min(600, startWidth + delta));
    }

    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    }

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  onMount(async () => {
    // Initialize theme
    try {
      const theme = await resolveStartupTheme();
      activeTheme.set(theme);
      applyTheme(theme);
    } catch (e) {
      try {
        await initTheme("github-dark");
      } catch {
        addToast({ message: m.theme_load_failed(), type: "error" });
      }
    }
    await listenThemeChanges();

    teardownProviderReroute = installProviderDisconnectReroute();
    initProjects();
    initTerminalEvents();
    detectAiProviders();
    loadPreferredProvider();
    startAiBackgroundListeners();
    refreshAiBackgroundRuns().catch(() => {});
    // AI session auto-refresh listeners are per-project-path — register
    // once here with the initial active project (if any), and re-register
    // from the `onProjectSwitch` callback below. Putting this at the
    // app-shell level keeps `AiSessionsView` / `AiSessionList` free of
    // `onMount` work so the view swap into AI Sessions paints the same
    // frame as the rest of the sections (pipelines / tags / branches).
    refreshAiSessionListeners();

    try {
      sidebarCollapsed = await getSidebarCollapsed();
    } catch {
      // Default to expanded
    }
    // Hydrate the customised Navigation layout in parallel with the
    // other settings so the first Sidebar render uses the persisted
    // order + hidden set instead of flashing the default order first.
    await loadSidebarLayout();

    // Reset view to graph on project tab switch for instant responsiveness
    onProjectSwitch(() => {
      tryChangeView("graph");
      selectedDiff = null;
      selectedStagingFile = null;
      // Point the AI session listeners at the freshly active project.
      refreshAiSessionListeners();
    });

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
      {
        id: "ui.toggleSidebar",
        keys: { mod: true, key: "b" },
        label: m.sidebar_collapse(),
        category: "UI",
        action: () => handleToggleSidebar(),
      },
      {
        id: "tab.newTerminal",
        keys: { mod: true, key: "t" },
        label: m.tab_terminal_here(),
        category: "Tabs",
        action: async () => {
          const proj = get(activeProject);
          if (proj) {
            const name = proj.path.split("/").pop() ?? "Terminal";
            await openTerminalTab(proj.path, `Terminal · ${name}`);
          } else {
            try {
              const { homeDir } = await import("@tauri-apps/api/path");
              const home = await homeDir();
              await openTerminalTab(home, "Terminal");
            } catch {
              await openTerminalTab("/", "Terminal");
            }
          }
        },
      },
    ];

    const gitShortcuts = [
      {
        id: "git.fetch",
        keys: { mod: true, shift: true, key: "F" },
        label: m.toolbar_fetch(),
        category: "Git",
        action: async () => {
          try {
            await runMutation({
              kind: "fetch",
              invoke: () => api.fetchRemote("origin"),
              successToast: (n) =>
                `Fetched origin — ${n} ref${n === 1 ? "" : "s"}`,
              failureToastPrefix: "Fetch failed",
              trackAsTask: true,
            });
          } catch { /* runMutation surfaced the toast */ }
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
          const branch = info.head_branch;
          try {
            await runMutation({
              kind: "pull",
              invoke: () => api.pullRemote("origin", branch),
              successToast: (n) =>
                `Pulled origin/${branch} — ${n} commit${n === 1 ? "" : "s"}`,
              failureToastPrefix: "Pull failed",
              trackAsTask: true,
            });
          } catch { /* runMutation surfaced the toast */ }
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
          const branch = info.head_branch;
          try {
            await runMutation({
              kind: "push",
              invoke: () => api.pushRemote("origin", branch, false),
              successToast: () => `Pushed to origin/${branch}`,
              failureToastPrefix: "Push failed",
              trackAsTask: true,
            });
          } catch { /* runMutation surfaced the toast */ }
        },
      },
      {
        id: "branch.newBranch",
        keys: { mod: true, shift: true, key: "B" },
        label: "New branch",
        category: "Git",
        action: () => openCreateBranchDialog({ kind: "head" }),
      },
      {
        id: "git.stageAll",
        keys: { mod: true, shift: true, key: "S" },
        label: m.changes_stage_all(),
        category: "Git",
        action: async () => {
          try {
            await runMutation({
              kind: "stage",
              invoke: () => api.stageAll(),
              failureToastPrefix: "Stage failed",
            });
          } catch { /* runMutation surfaced the toast */ }
        },
      },
      {
        id: "git.unstageAll",
        keys: { mod: true, shift: true, key: "U" },
        label: m.changes_unstage_all(),
        category: "Git",
        action: async () => {
          try {
            await runMutation({
              kind: "unstage",
              invoke: () => api.unstageAll(),
              failureToastPrefix: "Unstage failed",
            });
          } catch { /* runMutation surfaced the toast */ }
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
        global: true,
      },
      {
        id: "util.tasks",
        keys: { mod: true, shift: true, key: "T" },
        label: m.tasks_title(),
        category: "General",
        // Retained as a secondary binding for muscle memory — opens
        // the same popover as Cmd+J. The dedicated expanded-panel mode
        // was retired alongside the cluster-0.3 drawer.
        action: () => openTasksPopover(),
      },
      {
        id: "ai.newBackgroundRun",
        keys: { mod: true, shift: true, key: "A" },
        label: m.ai_background_new_run_button(),
        category: "AI",
        action: () => { showAiBackgroundDialog = true; },
        global: true,
      },
      {
        id: "util.tasksPopover",
        keys: { mod: true, key: "j" },
        label: m.tasks_title(),
        category: "General",
        // Global so the popover opens even while an input is focused —
        // users hit Cmd+J mid-commit-message all the time.
        action: () => toggleTasksPopover(),
        global: true,
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

    teardownRepoConfigRoute = initRepoConfigRouteSync();
  });

  onDestroy(() => {
    if (registeredShortcutIds.length > 0) {
      unregisterShortcuts(registeredShortcutIds);
    }
    teardownRepoConfigRoute?.();
    teardownRepoConfigRoute = null;
    teardownProviderReroute?.();
    teardownProviderReroute = null;
  });

  /**
   * (Re-)wire the AI session auto-refresh listeners to the currently
   * active project. Called once from `onMount` with the initial project
   * and again from `onProjectSwitch` whenever the user picks a
   * different tab. Kept outside `AiSessionList.svelte` / `AiSessionsView`
   * so the AI Sessions view swap stays a zero-work shell-paint — same
   * async-first pattern as `TagView` / `BranchView` / `PipelineView`.
   */
  function refreshAiSessionListeners(): void {
    stopConversationListeners();
    const proj = get(activeProject);
    if (proj?.path) {
      void startConversationListeners(proj.path);
    }
  }

  /**
   * Route any proposed `activeView` change through `RepoConfigPage`'s
   * navigation guard when the user is currently on the repo-config view
   * and has unsaved edits in the active section. Outside that view the
   * assignment runs straight through.
   */
  function tryChangeView(nextView: string): void {
    if (activeView === "repo-config" && repoConfigPageRef) {
      repoConfigPageRef.requestGuardedNavigation(() => {
        activeView = nextView;
      });
    } else {
      activeView = nextView;
    }
  }

  async function handleNavigate(view: string) {
    // If a terminal tab is active, switch to the last project tab first
    const tab = get(activeTab);
    if (tab?.kind === "terminal") {
      const projIdx = findLastProjectTabIndex();
      if (projIdx >= 0) {
        await switchToTab(projIdx);
        tryChangeView(view);
        selectedDiff = null;
        selectedStagingFile = null;
      }
      return;
    }

    if (tab?.kind === "composite" && tab.activeSegmentIndex >= 0) {
      // Switch to project segment of the same composite tab
      const idx = get(activeTabIndex);
      switchSegment(idx, -1);
      tryChangeView(view);
      selectedDiff = null;
      selectedStagingFile = null;
      return;
    }

    if (view === "blame") {
      blamePreviousView.set(activeView);
    }
    tryChangeView(view);
    selectedDiff = null;
    selectedStagingFile = null;
  }

  function handleToggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    setSidebarCollapsed(sidebarCollapsed);
  }

  async function handleFileClick(path: string, staged: boolean) {
    selectedStagingFile = { filename: path, isStaged: staged };
    // Ensure diffs are loaded — they may be empty after a project switch
    const diffs = staged ? get(stagedDiffs) : get(unstagedDiffs);
    if (diffs.length === 0) {
      await refreshDiffs();
    }
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

  async function handleReflogFileClick(path: string) {
    const entry = get(selectedReflogEntryStore);
    if (!entry) return;
    try {
      const detail = await api.getCommitDetail(entry.oid);
      const parentOid = detail.parents?.[0] ?? null;
      const [oldContent, newContent] = await Promise.all([
        parentOid ? getFileAtCommit(parentOid, path).catch(() => "") : Promise.resolve(""),
        getFileAtCommit(entry.oid, path).catch(() => ""),
      ]);
      reflogFileDiff.set({ oldContent, newContent, filename: path });
    } catch {
      reflogFileDiff.set(null);
    }
  }

  // ── PR diff panel ──────────────────────────────────────────────────
  /** Open the PR per-file diff panel for `path`. */
  async function handlePrFileClick(path: string) {
    const detail = get(mrPrDetail);
    if (!detail) return;
    await loadPrFileDiff(detail, path);
  }

  /** Cycle to the prev/next file in the PR file list (bracket shortcuts). */
  function handlePrFileNav(delta: -1 | 1) {
    const files = get(mrPrDiffFiles);
    if (files.length === 0) return;
    const cur = get(selectedPrFilePath);
    const idx = cur ? files.findIndex((f) => f.path === cur) : -1;
    const next = (idx + delta + files.length) % files.length;
    void handlePrFileClick(files[next].path);
  }

  // Register bracket-key shortcuts while the merge-requests view is active.
  $effect(() => {
    if (activeView === "merge-requests") {
      registerPrDiffShortcuts({
        onPrev: () => handlePrFileNav(-1),
        onNext: () => handlePrFileNav(1),
      });
      return () => unregisterPrDiffShortcuts();
    }
  });

  /** Build the per-file comments layer from the current PR detail. */
  function commentsLayerFor(path: string): DiffCommentsLayerProps | undefined {
    const detail = get(mrPrDetail);
    if (!detail) return undefined;
    const comments = detail.comments.filter((c) => c.path === path);
    return {
      comments,
      onPost: async (line, body) => {
        await postReviewComment(detail.summary.number, path, line, body);
      },
      onReply: async (_threadId, body) => {
        // GitHub: no dedicated reply endpoint via CLI — post a new
        // inline comment on the same line; the thread groups on the
        // GitHub side. GitLab: same endpoint + the discussion id
        // is implicit because the note lands in the same discussion
        // when position matches.
        const line = comments[0]?.line ?? 1;
        await postReviewComment(detail.summary.number, path, line, body);
      },
      onToggleResolve: async (discussionId, resolved) => {
        if (resolved) await unresolveDiscussion(detail.summary.number, discussionId);
        else await resolveDiscussion(detail.summary.number, discussionId);
      },
    };
  }

  // ── Reflog context menu ────────────────────────────────────────────
  let reflogCtxVisible = $state(false);
  let reflogCtxX = $state(0);
  let reflogCtxY = $state(0);
  let reflogCtxEntry = $state<ReflogEntry | null>(null);

  function handleReflogContextMenu(e: MouseEvent, entry: ReflogEntry, _index: number) {
    reflogCtxEntry = entry;
    reflogCtxX = e.clientX;
    reflogCtxY = e.clientY;
    reflogCtxVisible = true;
  }

  function buildReflogContextItems(entry: ReflogEntry): MenuItem[] {
    return [
      {
        label: m.reflog_show_in_graph(),
        action: () => {
          navigateToCommit(entry.oid);
          handleNavigate("graph");
        },
      },
      {
        label: m.graph_checkout({ sha: shortOid(entry.oid) }),
        action: async () => {
          try {
            await runMutation({
              kind: "checkout_detached",
              invoke: () => api.checkoutDetached(entry.oid),
              successToast: () => `Checked out ${shortOid(entry.oid)} (detached)`,
              failureToastPrefix: "Checkout failed",
            });
            await loadReflog();
          } catch { /* runMutation surfaced the toast */ }
        },
      },
      {
        label: m.graph_create_branch({ sha: shortOid(entry.oid) }),
        action: async () => {
          const name = prompt(m.graph_branch_name_prompt());
          if (!name) return;
          try {
            await runMutation({
              kind: "branch_create",
              invoke: () => api.createBranchAt(name, entry.oid),
              successToast: () => `Created branch ${name}`,
              failureToastPrefix: "Branch create failed",
            });
            await loadReflog();
          } catch { /* runMutation surfaced the toast */ }
        },
      },
      { separator: true },
      {
        label: m.graph_reset_soft(),
        action: async () => {
          if (confirm(m.graph_reset_confirm_soft({ sha: shortOid(entry.oid) }))) {
            try {
              await runMutation({
                kind: "reset_soft",
                invoke: () => api.resetToCommit(entry.oid, "soft"),
                successToast: () => `Reset (soft) to ${shortOid(entry.oid)}`,
                failureToastPrefix: "Reset failed",
              });
              await loadReflog();
            } catch { /* runMutation surfaced the toast */ }
          }
        },
      },
      {
        label: m.graph_reset_mixed(),
        action: async () => {
          if (confirm(m.graph_reset_confirm_mixed({ sha: shortOid(entry.oid) }))) {
            try {
              await runMutation({
                kind: "reset_mixed",
                invoke: () => api.resetToCommit(entry.oid, "mixed"),
                successToast: () => `Reset (mixed) to ${shortOid(entry.oid)}`,
                failureToastPrefix: "Reset failed",
              });
              await loadReflog();
            } catch { /* runMutation surfaced the toast */ }
          }
        },
      },
      {
        label: m.graph_reset_hard(),
        action: async () => {
          if (confirm(m.graph_reset_confirm_hard({ sha: shortOid(entry.oid) }))) {
            try {
              await runMutation({
                kind: "reset_hard",
                invoke: () => api.resetToCommit(entry.oid, "hard"),
                successToast: () => `Reset (hard) to ${shortOid(entry.oid)}`,
                failureToastPrefix: "Reset failed",
              });
              await loadReflog();
            } catch { /* runMutation surfaced the toast */ }
          }
        },
      },
      { separator: true },
      {
        label: m.graph_copy_sha({ sha: shortOid(entry.oid) }),
        action: () => navigator.clipboard.writeText(entry.oid),
      },
    ];
  }

  // ── Reflog lifecycle ───────────────────────────────────────────────
  // SplitView handles initial load and repo-changed listener.
  // We only need to clear selection when navigating away to prevent stale state.
  $effect(() => {
    if (activeView !== "reflog") {
      clearReflogSelection();
    }
  });

  // ── Navigation store bridge ────────────────────────────────────────
  // Mirror the local `activeView` into `activeViewStore` so cross-cutting
  // components (e.g. <Xrefs>) can programmatically switch views.
  $effect(() => {
    activeViewStore.set(activeView);
  });
  $effect(() => {
    const v = $activeViewStore;
    if (v !== activeView) tryChangeView(v);
  });
</script>

<div class="app-shell">
  <TabBar />

  <div class="main-area">
    <Sidebar
      onNavigate={handleNavigate}
      {activeView}
      collapsed={sidebarCollapsed}
      onToggleCollapse={handleToggleSidebar}
    />

    <div class="center-panel" style:background={($activeTab?.kind === "terminal" || ($activeTab?.kind === "composite" && $activeTab.activeSegmentIndex >= 0 && $activeTab.segments[$activeTab.activeSegmentIndex]?.type === "terminal")) ? $activeTheme?.colors.background : undefined}>
      <ConflictToolbar />
      <div class="content-wrapper">
        <!-- Persistent terminal layer: always mounted, shown/hidden via visibility.
             Uses absolute positioning so terminals keep full dimensions when hidden.
             Keyed by sessionId so Svelte never destroys/recreates on tab reorder. -->
        {#each $openTabs as tab, i}
          {#if tab.kind === "terminal"}
            <div class="terminal-persist" class:visible={i === $activeTabIndex} style:background={$activeTheme?.colors.background}>
              <TerminalView terminal={tab.terminal} />
            </div>
          {:else if tab.kind === "composite"}
            {#each tab.segments as segment, si (segment.type === "terminal" ? `c${i}-t-${segment.info.sessionId}` : `c${i}-skip-${si}`)}
              {#if segment.type === "terminal"}
                <div class="terminal-persist" class:visible={i === $activeTabIndex && tab.activeSegmentIndex === si} style:background={$activeTheme?.colors.background}>
                  <TerminalView terminal={segment.info} />
                </div>
              {/if}
            {/each}
          {/if}
        {/each}
        {#if $activeTab?.kind === "terminal" || ($activeTab?.kind === "composite" && $activeTab.activeSegmentIndex >= 0 && $activeTab.segments[$activeTab.activeSegmentIndex]?.type === "terminal")}
          <!-- Terminal is showing via persistent layer above -->
        {:else if activeView === "settings"}
        <SettingsPage />
      {:else if activeView === "pipelines"}
        <PipelineView />
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
        <WorktreeList
          onNavigateToGraph={(oid) => { navigateToCommit(oid); handleNavigate("graph"); }}
        />
      {:else if activeView === "reflog"}
        <div class="branch-layout">
          <div class="branch-main">
            <ReflogView
              onContextMenu={handleReflogContextMenu}
              onNavigateToGraph={(oid) => { navigateToCommit(oid); handleNavigate("graph"); }}
              onNavigate={handleNavigate}
              onFileClick={handleReflogFileClick}
            />
          </div>
          {#if $reflogFileDiff}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="diff-resize-handle" onmousedown={startDiffResize}></div>
            <div class="diff-panel" style="height: {diffPanelHeight}px">
              <DiffEditor
                oldContent={$reflogFileDiff.oldContent}
                newContent={$reflogFileDiff.newContent}
                filename={$reflogFileDiff.filename}
                editorTheme={$activeTheme?.editor}
                isDark={$activeTheme?.meta.mode !== 'light'}
                onClose={() => reflogFileDiff.set(null)}
              />
            </div>
          {/if}
        </div>
      {:else if activeView === "submodules"}
        <SubmoduleList />
      {:else if activeView === "bisect"}
        <BisectWorkflow />
      {:else if activeView === "ai-config"}
        <AiConfigEditor />
      {:else if activeView === "ai-sessions"}
        <AiSessionsView />
      {:else if activeView === "blame"}
        <BlameView onNavigateBack={(view) => tryChangeView(view)} />
      {:else if activeView === "merge-requests"}
        <div class="branch-layout mr-pr-layout">
          <div class="branch-main">
            <MrPrView onFileClick={handlePrFileClick} />
          </div>
          {#if $prFileDiff || $loadingPrFileDiff || $prFileDiffError}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="diff-resize-handle" onmousedown={startDiffResize}></div>
            <div class="diff-panel" style="height: {diffPanelHeight}px">
              {#if $loadingPrFileDiff}
                <div class="diff-panel-loading"><div class="spinner"></div></div>
              {:else if $prFileDiffError}
                <div class="diff-error" role="alert">{$prFileDiffError}</div>
              {:else if $prFileDiff}
                <DiffEditor
                  oldContent={$prFileDiff.oldContent}
                  newContent={$prFileDiff.newContent}
                  filename={$prFileDiff.filename}
                  editorTheme={$activeTheme?.editor}
                  isDark={$activeTheme?.meta.mode !== 'light'}
                  placeholder={$prFileDiff.binary ? m.diff_binary_file() : undefined}
                  commentsLayer={commentsLayerFor($prFileDiff.filename)}
                >
                  {#snippet toolbar()}
                    {@const files = $mrPrDiffFiles}
                    {@const idx = files.findIndex((f) => f.path === $prFileDiff?.filename)}
                    <button class="nav-btn" aria-label="Previous file" onclick={() => handlePrFileNav(-1)}>&#x276E;</button>
                    <button class="nav-btn" aria-label="Next file" onclick={() => handlePrFileNav(1)}>&#x276F;</button>
                    <span class="diff-filename">{$prFileDiff?.filename ?? ""}</span>
                    <span class="diff-position">{idx + 1} / {files.length}</span>
                    <button class="diff-close" onclick={closePrFileDiff}>&#xF00D;</button>
                  {/snippet}
                </DiffEditor>
              {/if}
            </div>
          {/if}
        </div>
      {:else if activeView === "issues"}
        <IssueView />
      {:else if activeView === "releases"}
        <ReleaseView />
      {:else if activeView === "repo-config"}
        <RepoConfigPage bind:this={repoConfigPageRef} />
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
            <div class="changes-sidebar" style="width: {changesSidebarWidth}px">
              <StagingArea onFileClick={handleFileClick} onNavigate={handleNavigate} />
            </div>
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="changes-resize-handle" onmousedown={startChangesSidebarResize}></div>
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
      </div><!-- /content-wrapper -->
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
          onNavigateToGraph={(oid) => { navigateToCommit(oid); handleNavigate("graph"); }}
          onClose={() => closeBranchCommitDetail()}
          onFileClick={handleBranchFileClick}
          onNavigate={handleNavigate}
        />
      </div>
    {/if}
  </div>

  <ShortcutOverlay />
  <StatusBar />

  <TasksPopover open={$tasksPopoverOpen} onClose={closeTasksPopover} />

  {#if showAiBackgroundDialog}
    <CreateBackgroundRunDialog onClose={() => (showAiBackgroundDialog = false)} />
  {/if}

  <CreateBranchDialog
    open={$createBranchDialog.open}
    initialSource={$createBranchDialog.source}
    onClose={closeCreateBranchDialog}
  />

  {#if reflogCtxVisible && reflogCtxEntry}
    <ContextMenu
      items={buildReflogContextItems(reflogCtxEntry)}
      x={reflogCtxX}
      y={reflogCtxY}
      visible={reflogCtxVisible}
      onClose={() => { reflogCtxVisible = false; }}
    />
  {/if}
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

  .content-wrapper {
    flex: 1;
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .terminal-persist {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    visibility: hidden;
    z-index: 0;
  }

  .terminal-persist.visible {
    visibility: visible;
    z-index: 10;
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
    color: var(--text-primary);
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
    flex-shrink: 0;
    overflow: hidden;
  }

  .changes-resize-handle {
    width: 4px;
    cursor: col-resize;
    background: transparent;
    transition: background 0.15s;
    flex-shrink: 0;
    border-left: 1px solid var(--border);
  }

  .changes-resize-handle:hover {
    background: var(--accent-blue);
  }

  .changes-diff {
    flex: 1;
    overflow: hidden;
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
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    font-size: 13px;
    font-style: italic;
    opacity: 0.5;
    gap: 8px;
  }

  .nav-btn {
    background: none; border: none; color: var(--text-secondary);
    font-size: 12px;
    padding: 2px 6px; cursor: pointer;
  }
  .nav-btn:hover { color: var(--text-primary); }
  .diff-position {
    color: var(--text-secondary); font-size: 11px; margin-left: auto;
    padding-right: 8px;
  }
  .diff-error {
    padding: 12px 16px;
    color: var(--text-error, #f85149);
    font-size: 13px;
  }

</style>
