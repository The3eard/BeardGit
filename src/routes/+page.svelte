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
  import ResizableDiffPanel from "$lib/components/editor/ResizableDiffPanel.svelte";
  import LazyComponent from "$lib/components/common/LazyComponent.svelte";
  import EmptyState from "$lib/components/common/EmptyState.svelte";
  import { repoInfo, isLoading, error } from "$lib/stores/repo";
  import { selectedCommit, selectedOid, selectedCommitFiles, openFileDiff, navigateToCommit, graphNavigateDown, graphNavigateUp, graphNavigateFirst, graphNavigateLast, fetchDiffSides, fileDiffPanel, loadingFileDiff, closeFileDiff } from "$lib/stores/graph";
  import type { RawDiffContent } from "$lib/stores/graph";
  import * as m from "$lib/paraglide/messages";
  import TasksPopover from "$lib/components/tasks/TasksPopover.svelte";
  import {
    tasksPopoverOpen,
    toggleTasksPopover,
    openTasksPopover,
    closeTasksPopover,
  } from "$lib/stores/tasksPopover";
  import { initProjects, openFolderAsProject, openProjectTab, activeProject, switchToNextTab, switchToPrevTab, closeActiveTab, switchToTab, onProjectSwitch } from "$lib/stores/projects";
  import { openCloneDialog } from "$lib/stores/cloneDialog";
  import StashView from "$lib/components/stash/StashView.svelte";
  import ConflictToolbar from "$lib/components/conflict/ConflictToolbar.svelte";
  import TagView from "$lib/components/tags/TagView.svelte";
  import BranchView from "$lib/components/branches/BranchView.svelte";
  import WorktreeList from "$lib/components/worktrees/WorktreeList.svelte";
  import SubmoduleList from "$lib/components/submodules/SubmoduleList.svelte";
  import MrPrView from "$lib/components/mr-pr/MrPrView.svelte";
  import IssueView from "$lib/components/issues/IssueView.svelte";
  import { activeViewStore, installProviderDisconnectReroute } from "$lib/stores/navigation";
  import { branchFileDiff, branchSelectedCommit, branchSelectedFiles, closeBranchCommitDetail } from "$lib/stores/branches";
  import { blamePreviousView } from "$lib/stores/blame";
  import { initTerminalEvents } from "$lib/stores/terminal";
  import { activeTab, activeTabIndex, findLastProjectTabIndex, openTerminalTab, switchSegment, openTabs, getActiveTerminalSegment, getCompositeTerminals } from "$lib/stores/tabs";
  import { getSidebarCollapsed, setSidebarCollapsed, resolveStartupTheme } from "$lib/api/tauri";
  import { loadSidebarLayout } from "$lib/stores/sidebarLayout";
  import { loadEditorPrefs } from "$lib/stores/editorPrefs";
  import ReflogView from "$lib/components/reflog/ReflogView.svelte";
  import ContextMenu from "$lib/components/common/ContextMenu.svelte";
  import type { MenuItem } from "$lib/components/common/ContextMenu.svelte";
  import {
    loadReflog,
    clearReflogSelection,
    reflogFileDiff,
    selectedReflogEntry as selectedReflogEntryStore,
  } from "$lib/stores/reflog";
  import type { ReflogEntry } from "$lib/types";
  import { shortOid } from "$lib/utils/git";
  import * as api from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import { openStagingFile, openStagingDiff, loadStagingDiff, closeStagingDiff } from "$lib/stores/changes";
  import { activeTheme, applyTheme, listenThemeChanges, initTheme } from "$lib/stores/theme";
  import { registerShortcuts, unregisterShortcuts, toggleCheatSheet } from "$lib/stores/shortcuts";
  import { openCommandPalette } from "$lib/stores/commandPalette";
  import CommandPalette from "$lib/components/common/CommandPalette.svelte";
  import { Button } from "$lib/components/ui";
  import { addToast } from "$lib/stores/toast";
  import { get } from "svelte/store";
  import ShortcutOverlay from "$lib/components/common/ShortcutOverlay.svelte";
  import { detectAiProviders, loadPreferredProvider } from "$lib/stores/ai";
  import CreateBackgroundRunDialog from "$lib/components/ai/CreateBackgroundRunDialog.svelte";
  import RepoConfigPage from "$lib/components/repo-config/RepoConfigPage.svelte";
  import {
    persistTabsForProject as persistEditorTabs,
    startFileEditorListeners,
  } from "$lib/stores/fileEditor";
  import { initRepoConfigRouteSync } from "$lib/stores/repoConfigRoute";
  import { startAiBackgroundListeners, refreshAiBackgroundRuns, openCreateBackgroundRunDialogRequest } from "$lib/stores/aiBackground";
  import { startConversationListeners, stopConversationListeners } from "$lib/stores/aiConversations";
  import { createBranchDialog, openCreateBranchDialog, closeCreateBranchDialog } from "$lib/stores/createBranchDialog";
  import CreateBranchDialog from "$lib/components/branches/CreateBranchDialog.svelte";
  import InitRepoDialog from "$lib/components/init-repo/InitRepoDialog.svelte";
  import CloneRepoDialog from "$lib/components/clone/CloneRepoDialog.svelte";
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
    replyToReviewComment,
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
  let teardownDragDropListener: (() => void) | null = null;
  let teardownFileEditor: (() => void) | null = null;
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

  /**
   * The staging file open in the diff pane, mapped from the changes store
   * for the ChangesList highlight. The store is the source of truth so the
   * mutation dispatcher's `refreshDiffs()` can re-fetch the open file's
   * hunks without this component wiring up its own listener.
   */
  let selectedStagingFile = $derived(
    $openStagingFile
      ? { filename: $openStagingFile.path, isStaged: $openStagingFile.isStaged }
      : null,
  );
  let registeredShortcutIds: string[] = [];
  const CHANGES_SIDEBAR_DEFAULT_WIDTH = 320;
  let changesSidebarWidth = $state(CHANGES_SIDEBAR_DEFAULT_WIDTH);
  let isDraggingChanges = $state(false);
  let sidebarCollapsed = $state(false);
  let recentRepos = $state<{ path: string; name: string }[]>([]);
  /** True while the OS is hovering a draggable file/folder over the
   * welcome screen. Drives the dashed-border highlight. */
  let isDraggingOverWelcome = $state(false);

  // Lazy-load recent repos for the welcome screen. The list is small and
  // only matters when no project is active, so we re-fetch on every
  // welcome render rather than caching globally.
  $effect(() => {
    if ($activeProject) return;
    let cancelled = false;
    api.getRecentRepos()
      .then((r) => { if (!cancelled) recentRepos = r.slice(0, 5); })
      .catch(() => {});
    return () => { cancelled = true; };
  });


  /** Max width for the changes sidebar: 80% of the changes layout,
   *  measured from the resize handle's container so the diff pane
   *  always keeps ~20%. Falls back to the window when unmounted. */
  function changesSidebarMaxWidth(handle: HTMLElement | null): number {
    const containerWidth =
      handle?.parentElement?.clientWidth ?? window.innerWidth;
    return containerWidth * 0.8;
  }

  function startChangesSidebarResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = changesSidebarWidth;
    const maxW = changesSidebarMaxWidth(e.currentTarget as HTMLElement);
    isDraggingChanges = true;

    function onMouseMove(e: MouseEvent) {
      const delta = e.clientX - startX;
      changesSidebarWidth = Math.max(240, Math.min(maxW, startWidth + delta));
    }

    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      isDraggingChanges = false;
    }

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  function resetChangesSidebarWidth() {
    changesSidebarWidth = CHANGES_SIDEBAR_DEFAULT_WIDTH;
  }

  function handleChangesResizeKeys(e: KeyboardEvent) {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      changesSidebarWidth = Math.max(240, changesSidebarWidth - 20);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      const maxW = changesSidebarMaxWidth(e.currentTarget as HTMLElement);
      changesSidebarWidth = Math.min(maxW, changesSidebarWidth + 20);
    } else if (e.key === "Home") {
      e.preventDefault();
      resetChangesSidebarWidth();
    }
  }

  onMount(async () => {
    // Initialize theme
    try {
      const theme = await resolveStartupTheme();
      activeTheme.set(theme);
      applyTheme(theme);
    } catch (e) {
      try {
        await initTheme("beardgit-dark");
      } catch {
        addToast({ message: m.theme_load_failed(), type: "error" });
      }
    }

    // Welcome-screen drag-drop: open a folder by dropping it on the
    // window when no project is active. We listen globally and gate
    // on `$activeProject` rather than scoping to a DOM node because
    // Tauri's drag-drop event bypasses HTML5 dispatch when
    // `dragDropEnabled` is on.
    try {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      teardownDragDropListener = await getCurrentWebview().onDragDropEvent((event) => {
        const payload = event.payload as
          | { type: "enter" | "over"; paths?: string[] }
          | { type: "drop"; paths: string[] }
          | { type: "leave" };
        if (payload.type === "enter" || payload.type === "over") {
          if (!$activeProject) isDraggingOverWelcome = true;
        } else if (payload.type === "leave") {
          isDraggingOverWelcome = false;
        } else if (payload.type === "drop") {
          isDraggingOverWelcome = false;
          if ($activeProject) return;
          const path = payload.paths?.[0];
          if (path) void openProjectTab(path);
        }
      });
    } catch {
      // Optional feature — older Tauri / non-desktop envs simply
      // skip the drag-drop listener.
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
    // Hydrate the editor preferences store so the (PR3) editor panel
    // can read the persisted toggles synchronously when it first
    // mounts. Failure is non-fatal — the store stays `null` and
    // consumers render their loading state until a later success.
    await loadEditorPrefs();

    // Start the file-editor's project-mutated listener (flags external
    // changes on open buffers) and remember the teardown for onDestroy.
    teardownFileEditor = startFileEditorListeners();

    // Reset view to graph on project tab switch for instant responsiveness
    onProjectSwitch(() => {
      // Persist the just-leaving project's open editor tabs so reopening
      // the project (this session or after a restart) restores the same
      // set. Persist BEFORE the new project starts loading so we capture
      // the right paths.
      const prev = get(activeProject);
      if (prev?.path) persistEditorTabs(prev.path);

      tryChangeView("graph");
      selectedDiff = null;
      closeStagingDiff();
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
        keys: { mod: true, shift: true, key: "K" },
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
        id: "util.commandPalette",
        keys: { mod: true, shift: true, key: "P" },
        label: m.command_palette_title(),
        category: "General",
        action: () => openCommandPalette(),
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
    teardownDragDropListener?.();
    teardownDragDropListener = null;
    teardownFileEditor?.();
    teardownFileEditor = null;
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
    const apply = () => {
      activeView = nextView;
      // Leaving the Changes view resets the open file/diff so re-entering
      // lands on the empty diff panel. The checkbox selection persists via
      // the changesSelection store.
      if (nextView !== "changes") closeStagingDiff();
    };
    if (activeView === "repo-config" && repoConfigPageRef) {
      repoConfigPageRef.requestGuardedNavigation(apply);
    } else {
      apply();
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
        closeStagingDiff();
      }
      return;
    }

    if (tab?.kind === "composite" && tab.activeSegmentIndex >= 0) {
      // Switch to project segment of the same composite tab
      const idx = get(activeTabIndex);
      switchSegment(idx, -1);
      tryChangeView(view);
      selectedDiff = null;
      closeStagingDiff();
      return;
    }

    if (view === "blame") {
      blamePreviousView.set(activeView);
    }
    tryChangeView(view);
    selectedDiff = null;
    closeStagingDiff();
  }

  function handleToggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    setSidebarCollapsed(sidebarCollapsed);
  }

  async function handleFileClick(path: string, staged: boolean) {
    // Fetch the clicked file's full hunks/lines on demand — the Changes
    // list renders from lightweight stats, so hunks are pulled only for
    // the file the user actually opens.
    await loadStagingDiff(path, staged);
  }

  async function handleBranchFileClick(path: string) {
    const commit = $branchSelectedCommit;
    if (!commit) return;
    const parentOid = commit.parents?.[0] ?? null;
    try {
      branchFileDiff.set(await fetchDiffSides(commit.oid, parentOid, path));
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
      reflogFileDiff.set(await fetchDiffSides(entry.oid, parentOid, path));
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
      onReply: async (threadId, body) => {
        // `threadId` is the forge-specific identifier the parser stored on
        // the inline comment's `discussion_id`:
        //   - GitHub → root review-comment id; POST /pulls/{n}/comments/{id}/replies
        //   - GitLab → discussion id;          POST /merge_requests/{n}/discussions/{id}/notes
        // Both endpoints land the new note inside the existing thread.
        await replyToReviewComment(detail.summary.number, threadId, body);
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
              <LazyComponent
                loader={() => import("$lib/components/terminal/TerminalView.svelte")}
                props={{ terminal: tab.terminal }}
              />
            </div>
          {:else if tab.kind === "composite"}
            {#each tab.segments as segment, si (segment.type === "terminal" ? `c${i}-t-${segment.info.sessionId}` : `c${i}-skip-${si}`)}
              {#if segment.type === "terminal"}
                <div class="terminal-persist" class:visible={i === $activeTabIndex && tab.activeSegmentIndex === si} style:background={$activeTheme?.colors.background}>
                  <LazyComponent
                    loader={() => import("$lib/components/terminal/TerminalView.svelte")}
                    props={{ terminal: segment.info }}
                  />
                </div>
              {/if}
            {/each}
          {/if}
        {/each}
        {#if $activeTab?.kind === "terminal" || ($activeTab?.kind === "composite" && $activeTab.activeSegmentIndex >= 0 && $activeTab.segments[$activeTab.activeSegmentIndex]?.type === "terminal")}
          <!-- Terminal is showing via persistent layer above -->
        {:else if activeView === "settings"}
        <LazyComponent loader={() => import("$lib/components/settings/SettingsPage.svelte")} />
      {:else if activeView === "pipelines"}
        <LazyComponent loader={() => import("$lib/components/pipeline/PipelineView.svelte")} />
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
            <ResizableDiffPanel>
              <DiffEditor
                oldContent={$branchFileDiff.oldContent}
                newContent={$branchFileDiff.newContent}
                filename={$branchFileDiff.filename}
                placeholder={$branchFileDiff.placeholder}
                editorTheme={$activeTheme?.editor}
                isDark={$activeTheme?.meta.mode !== 'light'}
                onClose={() => branchFileDiff.set(null)}
              />
            </ResizableDiffPanel>
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
            <ResizableDiffPanel>
              <DiffEditor
                oldContent={$reflogFileDiff.oldContent}
                newContent={$reflogFileDiff.newContent}
                filename={$reflogFileDiff.filename}
                placeholder={$reflogFileDiff.placeholder}
                editorTheme={$activeTheme?.editor}
                isDark={$activeTheme?.meta.mode !== 'light'}
                onClose={() => reflogFileDiff.set(null)}
              />
            </ResizableDiffPanel>
          {/if}
        </div>
      {:else if activeView === "submodules"}
        <SubmoduleList />
      {:else if activeView === "bisect"}
        <LazyComponent loader={() => import("$lib/components/bisect/BisectWorkflow.svelte")} />
      {:else if activeView === "ai-config"}
        <LazyComponent loader={() => import("$lib/components/ai-config/AiConfigEditor.svelte")} />
      {:else if activeView === "ai-sessions"}
        <LazyComponent loader={() => import("$lib/components/ai-sessions/AiSessionsView.svelte")} />
      {:else if activeView === "blame"}
        <LazyComponent
          loader={() => import("$lib/components/blame/BlameView.svelte")}
          props={{ onNavigateBack: (view: string) => tryChangeView(view) }}
        />
      {:else if activeView === "merge-requests"}
        <div class="branch-layout mr-pr-layout">
          <div class="branch-main">
            <MrPrView onFileClick={handlePrFileClick} />
          </div>
          {#if $prFileDiff || $loadingPrFileDiff || $prFileDiffError}
            <ResizableDiffPanel loading={$loadingPrFileDiff}>
              {#if $loadingPrFileDiff}
                <div class="spinner"></div>
              {:else if $prFileDiffError}
                <div class="diff-error-state" role="alert">
                  <EmptyState
                    fill
                    icon={"\uF071"}
                    title={m.pr_diff_error_title()}
                    description={$prFileDiffError}
                  />
                </div>
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
            </ResizableDiffPanel>
          {/if}
        </div>
      {:else if activeView === "issues"}
        <IssueView />
      {:else if activeView === "releases"}
        <LazyComponent loader={() => import("$lib/components/releases/ReleaseView.svelte")} />
      {:else if activeView === "repo-config"}
        <RepoConfigPage bind:this={repoConfigPageRef} />
      {:else if activeView === "requests"}
        <LazyComponent loader={() => import("$lib/components/requests/RequestsPanel.svelte")} />
      {:else if activeView === "editor"}
        <LazyComponent loader={() => import("$lib/components/file-editor/FileEditorPanel.svelte")} />
      {:else if activeView === "compare"}
        <LazyComponent loader={() => import("$lib/components/compare/CompareView.svelte")} />
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
              <StagingArea onFileClick={handleFileClick} onNavigate={handleNavigate} selectedFile={selectedStagingFile} />
            </div>
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <div class="changes-resize-handle" class:is-dragging={isDraggingChanges} role="separator" aria-orientation="vertical" aria-label={m.resize_changes_sidebar()} tabindex="0" onmousedown={startChangesSidebarResize} ondblclick={resetChangesSidebarWidth} onkeydown={handleChangesResizeKeys}></div>
            <div class="changes-diff">
              {#if $openStagingDiff && $openStagingFile}
                <StagingDiffEditor
                  diff={$openStagingDiff}
                  isStaged={$openStagingFile.isStaged}
                  filename={$openStagingFile.path}
                  onClose={closeStagingDiff}
                />
              {:else}
                <EmptyState title={m.diff_empty()} />

              {/if}
            </div>
          </div>
        {:else}
          {#if $fileDiffPanel}
            <div class="graph-with-diff">
              <div class="graph-area">
                <GitGraph />
              </div>
              <ResizableDiffPanel>
                <DiffEditor
                  oldContent={$fileDiffPanel.oldContent}
                  newContent={$fileDiffPanel.newContent}
                  filename={$fileDiffPanel.filename}
                  placeholder={$fileDiffPanel.placeholder}
                  editorTheme={$activeTheme?.editor}
                  isDark={$activeTheme?.meta.mode !== 'light'}
                  onClose={closeFileDiff}
                />
              </ResizableDiffPanel>
            </div>
          {:else if $loadingFileDiff}
            <div class="graph-with-diff">
              <div class="graph-area">
                <GitGraph />
              </div>
              <ResizableDiffPanel loading>
                <div class="spinner"></div>
              </ResizableDiffPanel>
            </div>
          {:else}
            <GitGraph />
          {/if}
        {/if}
      {:else}
        <div class="welcome-screen" class:welcome-screen--drop-target={isDraggingOverWelcome}>
          <img class="welcome-logo" src="/logo.svg" alt="BeardGit" />
          <h2 class="welcome-title">{m.app_title()}</h2>
          <p class="welcome-tagline">{m.welcome_tagline()}</p>
          <p class="welcome-subtitle">{m.app_welcome_subtitle()}</p>

          <div class="welcome-chips">
            <span class="welcome-chip">{m.welcome_chip_graph()}</span>
            <span class="welcome-chip">{m.welcome_chip_reviews()}</span>
            <span class="welcome-chip">{m.welcome_chip_pipelines()}</span>
            <span class="welcome-chip">{m.welcome_chip_terminals()}</span>
            <span class="welcome-chip">{m.welcome_chip_ai()}</span>
            <span class="welcome-chip">{m.welcome_chip_http()}</span>
          </div>

          <div class="welcome-actions">
            <Button variant="primary" size="lg" icon={"\uF07C"} onclick={openFolderAsProject}>
              {m.welcome_open()}
            </Button>
            <Button variant="neutral" size="lg" icon={"\uF019"} onclick={openCloneDialog}>
              {m.welcome_clone()}
            </Button>
          </div>

          <div class="welcome-links">
            <button class="welcome-link" onclick={openFolderAsProject}>{m.welcome_init_action()} →</button>
            <button class="welcome-link" onclick={openCommandPalette}>{m.welcome_palette_hint()}</button>
          </div>

          <div class="welcome-recent">
            <h3 class="welcome-recent-title">{m.welcome_recent_title()}</h3>
            {#if recentRepos.length === 0}
              <p class="welcome-recent-empty">{m.welcome_no_recent()}</p>
            {:else}
              <ul class="welcome-recent-list">
                {#each recentRepos as repo}
                  <li>
                    <button class="welcome-recent-row" onclick={() => openProjectTab(repo.path)}>
                      <span class="welcome-recent-name">{repo.name}</span>
                      <span class="welcome-recent-path">{repo.path}</span>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>

          <div class="welcome-dropzone" aria-hidden="true">
            <span class="welcome-dropzone-icon nf">{"\uF07B"}</span>
            <span class="welcome-dropzone-main">{m.welcome_dropzone()}</span>
            <span class="welcome-dropzone-hint">{m.welcome_dropzone_hint()}</span>
          </div>
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
          showOpenInEditor={true}
          onNavigateToGraph={(oid) => { navigateToCommit(oid); handleNavigate("graph"); }}
          onClose={() => closeBranchCommitDetail()}
          onFileClick={handleBranchFileClick}
          onNavigate={handleNavigate}
        />
      </div>
    {/if}
  </div>

  <ShortcutOverlay />
  <CommandPalette />
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

  <InitRepoDialog />

  <CloneRepoDialog />

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
    border: 2px dashed transparent;
    border-radius: 8px;
    margin: 24px;
    transition: border-color 0.15s, background 0.15s;
  }

  .welcome-screen--drop-target {
    border-color: var(--accent-primary);
    background: var(--overlay-accent-blue);
  }

  .welcome-logo {
    width: 112px;
    height: 112px;
    margin-bottom: 4px;
    opacity: 0.92;
    user-select: none;
    -webkit-user-drag: none;
  }

  .welcome-title {
    font-size: 20px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .welcome-tagline {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-primary);
    margin-top: 2px;
  }

  .welcome-subtitle {
    font-size: var(--font-size-md);
    color: var(--text-secondary);
  }

  /* Capability chips — sell what BeardGit does at a glance. */
  .welcome-chips {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 6px;
    max-width: min(440px, 80%);
    margin-top: 4px;
  }

  .welcome-chip {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 10px;
  }

  .welcome-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  /* Tertiary text links: init flow + command-palette affordance. */
  .welcome-links {
    display: flex;
    gap: 16px;
    margin-top: 10px;
  }

  .welcome-link {
    background: none;
    border: none;
    color: var(--accent-primary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
  }

  .welcome-link:hover {
    text-decoration: underline;
  }

  .welcome-recent {
    margin-top: 24px;
    width: min(420px, 80%);
    text-align: center;
  }

  .welcome-recent-title {
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .welcome-recent-empty {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    opacity: 0.7;
  }

  .welcome-recent-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
  }

  .welcome-recent-row {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 6px 12px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-primary);
    text-align: left;
    transition: background 0.15s, border-color 0.15s;
  }

  .welcome-recent-row:hover {
    background: var(--overlay-hover);
    border-color: var(--border);
  }

  .welcome-recent-name {
    font-size: var(--font-size-md);
    font-weight: 500;
  }

  .welcome-recent-path {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Persistent drop target — always visible (not just mid-drag) so the
     drag-to-open affordance is discoverable. Brightens when the whole
     screen enters --drop-target state. */
  .welcome-dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    margin-top: 28px;
    padding: 16px 28px;
    width: min(420px, 80%);
    border: 1.5px dashed var(--border);
    border-radius: 8px;
    color: var(--text-muted);
    transition: border-color 0.15s, color 0.15s;
  }

  .welcome-screen--drop-target .welcome-dropzone {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .welcome-dropzone-icon {
    font-family: var(--font-icons);
    font-size: var(--font-size-lg);
    margin-bottom: 2px;
  }

  .welcome-dropzone-main {
    font-size: var(--font-size-sm);
  }

  .welcome-dropzone-hint {
    font-size: var(--font-size-2xs);
    opacity: 0.8;
  }

  .loading-text {
    font-size: var(--font-size-lg);
    color: var(--text-secondary);
  }

  .error-text {
    font-size: var(--font-size-md);
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
    background: var(--overlay-accent-blue);
  }

  .changes-resize-handle.is-dragging {
    background: var(--accent-primary);
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

  .nav-btn {
    background: none; border: none; color: var(--text-secondary);
    font-size: var(--font-size-sm);
    padding: 2px 6px; cursor: pointer;
  }
  .nav-btn:hover { color: var(--text-primary); }
  .diff-position {
    color: var(--text-secondary); font-size: var(--font-size-xs); margin-left: auto;
    padding-right: 8px;
  }
  .diff-error-state {
    display: flex;
    height: 100%;
    overflow-y: auto;
  }

</style>
