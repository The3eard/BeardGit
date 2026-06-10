<script lang="ts">
  import { fileStatuses, stageFiles, unstageFiles, commit, amendCommit, refreshStatuses, refreshDiffs } from "../../stores/changes";
  import ChangesList from "./ChangesList.svelte";
  import CleanDialog from "./CleanDialog.svelte";
  import { onMount, onDestroy } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import * as m from "$lib/paraglide/messages";
  import { getHeadMessage, createWorkingTreePatch, savePatchToFile, pushRemote, saveAiReview } from "$lib/api/tauri";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { runMutation } from "$lib/api/runMutation";
  import { hasAiProvider, aiGenerateCommitMessage, aiReviewCode } from "$lib/stores/ai";
  import { addToast } from "$lib/stores/toast";
  import { repoInfo } from "$lib/stores/repo";
  import { taskOutput, selectTask } from "$lib/stores/taskPanel";
  import { setTaskSubtitle } from "$lib/stores/tasks";
  import { openTasksPopover } from "$lib/stores/tasksPopover";
  import { stripAnsi } from "$lib/utils/strip-ansi";
  import { listen } from "@tauri-apps/api/event";
  import { save } from "@tauri-apps/plugin-dialog";
  import type { TaskInfo } from "$lib/types";
  import { Button, IconButton } from "$lib/components/ui";

  let {
    onFileClick,
    onNavigate,
    selectedFile = null,
  }: {
    onFileClick?: (path: string, staged: boolean) => void;
    onNavigate?: (view: string) => void;
    /** File currently shown in the diff panel — highlighted in its list. */
    selectedFile?: { filename: string; isStaged: boolean } | null;
  } = $props();

  let message = $state("");
  let isAmend = $state(false);
  let savedMessage = $state("");
  let showPatchDialog = $state(false);
  let showOverflowMenu = $state(false);
  let patchStagedOnly = $state(true);
  let aiCommitLoading = $state(false);

  // Tracked AI-task listeners. We register Tauri `task-completed` /
  // `task-failed` listeners on demand for each AI run; if the user
  // navigates away before the task fires, the matching event never
  // arrives and the listener leaks. Components that register through
  // `trackListener` get cleaned up automatically in `onDestroy`.
  const pendingUnlistens = new Set<UnlistenFn>();

  function trackListener(fn: UnlistenFn): UnlistenFn {
    pendingUnlistens.add(fn);
    return () => {
      pendingUnlistens.delete(fn);
      fn();
    };
  }

  onDestroy(() => {
    for (const fn of pendingUnlistens) {
      try { fn(); } catch { /* ignore */ }
    }
    pendingUnlistens.clear();
  });

  onMount(() => {
    refreshStatuses();
    refreshDiffs();

    function closeMenus(e: MouseEvent) {
      if (showOverflowMenu && !(e.target as HTMLElement).closest('.toolbar-actions')) {
        showOverflowMenu = false;
      }
    }
    document.addEventListener('click', closeMenus);
    return () => document.removeEventListener('click', closeMenus);
  });

  let staged = $derived($fileStatuses.filter(f => f.is_staged));
  let unstaged = $derived($fileStatuses.filter(f => !f.is_staged));
  let hasUntracked = $derived(unstaged.some(f => f.status === "new"));
  let showCleanDialog = $state(false);

  async function handleAmendToggle() {
    if (isAmend) {
      savedMessage = message;
      try {
        message = await getHeadMessage();
      } catch {
        message = '';
      }
    } else {
      message = savedMessage;
      savedMessage = '';
    }
  }

  async function handleCreatePatch() {
    try {
      const patchText = await createWorkingTreePatch(patchStagedOnly);
      const filePath = await save({
        title: m.patch_save_dialog_title(),
        defaultPath: "changes.patch",
        filters: [{ name: "Patch", extensions: ["patch", "diff"] }],
      });
      if (!filePath) return;
      await savePatchToFile(filePath, patchText);
      showPatchDialog = false;
    } catch (err) {
      alert(m.patch_create_failed({ error: String(err) }));
    }
  }

  async function handleAiCommitMessage() {
    if (staged.length === 0) {
      addToast({ message: m.ai_no_staged_changes(), type: "warning" });
      return;
    }
    aiCommitLoading = true;
    try {
      const taskId = await aiGenerateCommitMessage();
      // Don't auto-open the tasks popover — the user clicked Generate
      // commit message to fill the message box, not to babysit a task.
      // The row still appears in the drawer for after-the-fact viewing
      // (TaskKind::AiHeadless flows through the unified bridge). Same
      // behaviour the Code Review button now has.
      selectTask(taskId);

      const unlistenCompleted = trackListener(await listen<TaskInfo>("task-completed", (event) => {
        if (event.payload.id === taskId) {
          unlistenCompleted();
          unlistenFailed();
          collectAiOutput(taskId);
          aiCommitLoading = false;
        }
      }));
      const unlistenFailed = trackListener(await listen<TaskInfo>("task-failed", (event) => {
        if (event.payload.id === taskId) {
          unlistenCompleted();
          unlistenFailed();
          aiCommitLoading = false;
        }
      }));
    } catch {
      aiCommitLoading = false;
    }
  }

  function collectAiOutput(taskId: number) {
    let output: import("$lib/types").TaskOutputLine[] | undefined;
    const unsubscribe = taskOutput.subscribe((map) => {
      output = map.get(taskId);
    });
    unsubscribe();

    if (output && output.length > 0) {
      const raw = output.map((l) => l.text).join("\n").trim();
      const cleaned = stripAnsi(raw);
      if (cleaned) {
        message = cleaned;
      }
    }
  }

  async function handleCodeReview() {
    // The button itself is disabled when staged.length === 0 (see the
    // template), so this is a defensive guard — keeps the function
    // honest when called programmatically.
    if (staged.length === 0) {
      addToast({ message: m.ai_no_changes_to_review(), type: "warning" });
      return;
    }
    let diff: string;
    try {
      // Staged-only patch: the review reasons about exactly the diff
      // the user is about to commit, not whatever else is in the
      // working tree. Pairs with the disabled-when-no-staged button.
      diff = await createWorkingTreePatch(true);
    } catch (err) {
      const msg = String(err);
      if (msg.includes("No changes to create patch from")) {
        addToast({ message: m.ai_no_changes_to_review(), type: "warning" });
      } else {
        addToast({ type: "error", message: m.ai_review_save_failed({ message: msg }) });
      }
      return;
    }
    if (!diff.trim()) {
      addToast({ message: m.ai_no_changes_to_review(), type: "warning" });
      return;
    }

    let taskId: number;
    try {
      taskId = await aiReviewCode(diff);
    } catch (err) {
      addToast({ type: "error", message: m.ai_review_save_failed({ message: String(err) }) });
      return;
    }

    // Don't auto-open the popover — the user clicked Code Review, not
    // "show me my tasks". The new "AI: review code" row is in the
    // unified drawer (TaskKind::AiHeadless flows through the task event
    // bridge now) so the user can pop the drawer open from the
    // statusbar if they want to follow the stream; otherwise they wait
    // for the success toast.

    const unlistenCompleted = trackListener(await listen<TaskInfo>("task-completed", async (event) => {
      if (event.payload.id !== taskId) return;
      unlistenCompleted();
      unlistenFailed();
      await persistReviewOutput(taskId);
    }));
    const unlistenFailed = trackListener(await listen<TaskInfo>("task-failed", (event) => {
      if (event.payload.id !== taskId) return;
      unlistenCompleted();
      unlistenFailed();
    }));
  }

  /**
   * Pull the cleaned review text out of the task-output store and ask the
   * backend to drop it under `.beardgit/reviews/`. Surfaces a success
   * toast with an "Open" action that launches the saved file in the
   * user's default markdown viewer; falls back to an error toast if the
   * write fails or the task produced no output.
   */
  async function persistReviewOutput(taskId: number) {
    let lines: import("$lib/types").TaskOutputLine[] | undefined;
    const unsubscribe = taskOutput.subscribe((map) => {
      lines = map.get(taskId);
    });
    unsubscribe();

    if (!lines || lines.length === 0) return;
    const cleaned = stripAnsi(lines.map((l) => l.text).join("\n")).trim();
    if (!cleaned) return;

    try {
      const saved = await saveAiReview(cleaned);
      // Mirror the saved file's relative path onto the task entry so
      // the drawer's detail panel shows it under "Context" once the
      // task has finished and the user opens it from history. Without
      // this the drawer just shows the AI's raw output with no link
      // back to the on-disk artefact.
      setTaskSubtitle(String(taskId), saved.relative_path);
      // Belt-and-braces: rewrite `taskOutput` with the cleaned text we
      // just persisted. If any `task-output` events were missed during
      // the run (rAF coalescing, AI providers that batch output until
      // exit, Tauri reload, …) the drawer's detail panel would show an
      // empty pane when the user later clicks the row. The saved file
      // is the source of truth, so we use it to refill the legacy
      // taskOutput map keyed by this task's id. Split on \n so the
      // detail panel renders one <span> per line, matching the live
      // streaming layout.
      taskOutput.update((map) => {
        const reviewLines = cleaned.split(/\r?\n/).map((text) => ({
          stream: "stdout" as const,
          text,
        }));
        map.set(taskId, reviewLines);
        return new Map(map);
      });
      addToast({
        type: "success",
        message: m.ai_review_saved_toast({ path: saved.relative_path }),
        // 10 s is enough to register the path + click Open. We don't
        // make this sticky any more because the row is in the drawer's
        // history, so a missed toast is recoverable.
        duration: 10_000,
        actions: [
          {
            label: m.ai_review_open_action(),
            onclick: () => {
              // Try to open the .md in the user's default markdown
              // viewer first; if `openPath` fails (typically because
              // the OS has no default app registered for `.md`), fall
              // back to revealing the file in Finder/Explorer so the
              // user can still get to it.
              void openPath(saved.path).catch((err) => {
                console.warn("openPath failed, falling back to reveal:", err);
                void revealItemInDir(saved.path);
              });
            },
          },
        ],
      });
    } catch (err) {
      addToast({
        type: "error",
        message: m.ai_review_save_failed({ message: String(err) }),
      });
    }
  }

  let pushInProgress = $state(false);

  async function handlePush() {
    if (pushInProgress || !$repoInfo?.head_branch) return;
    const branch = $repoInfo.head_branch;
    pushInProgress = true;
    try {
      await runMutation({
        kind: "push",
        invoke: () => pushRemote("origin", branch, false),
        successToast: () => `Pushed to origin/${branch}`,
        failureToastPrefix: "Push failed",
        trackAsTask: true,
      });
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      pushInProgress = false;
    }
  }

  async function handleCommit() {
    if (!message.trim()) return;
    if (isAmend) {
      await amendCommit(message);
    } else {
      await commit(message);
    }
    message = "";
    isAmend = false;
  }
</script>

<div class="staging-area" data-testid="staging-area">
  <div class="file-lists">
    <ChangesList
      files={staged}
      title={m.staging_staged()}
      isStaged={true}
      selectedPath={selectedFile?.isStaged ? selectedFile.filename : null}
      onUnstage={(paths) => unstageFiles(paths)}
      onFileClick={(path) => onFileClick?.(path, true)}
      onNavigate={onNavigate}
    />

    <ChangesList
      files={unstaged}
      title={m.staging_unstaged()}
      isStaged={false}
      selectedPath={selectedFile && !selectedFile.isStaged ? selectedFile.filename : null}
      onStage={(paths) => stageFiles(paths)}
      onFileClick={(path) => onFileClick?.(path, false)}
      onNavigate={onNavigate}
    />
  </div>

  <div class="commit-box">
    <!-- Toolbar row: Amend + icon buttons + overflow -->
    <div class="commit-toolbar">
      <label class="amend-toggle">
        <input type="checkbox" bind:checked={isAmend} onchange={handleAmendToggle} data-testid="amend-toggle" />
        <span>{m.staging_amend_toggle()}</span>
      </label>
      <div class="toolbar-actions">
        {#if $hasAiProvider}
          <IconButton
            tone="default"
            icon={"\uF0EB"}
            description={staged.length === 0
              ? m.ai_commit_message_disabled_tooltip()
              : m.ai_commit_message()}
            loading={aiCommitLoading}
            disabled={aiCommitLoading || staged.length === 0}
            onclick={handleAiCommitMessage}
          />
          <IconButton
            tone="default"
            icon={"\uF002"}
            description={staged.length === 0 ? m.ai_review_disabled_tooltip() : m.ai_code_review()}
            disabled={staged.length === 0}
            onclick={handleCodeReview}
          />
        {/if}
        <IconButton
          tone="default"
          icon={"\uF141"}
          description={m.changes_overflow_more()}
          onclick={() => { showOverflowMenu = !showOverflowMenu; }}
        />

        {#if showOverflowMenu}
          <div class="overflow-menu">
            <button class="overflow-menu-item" onclick={() => { showOverflowMenu = false; showPatchDialog = true; }}>
              <span class="nf">{"\uF1C9"}</span> {m.patch_create_changes()}
            </button>
            {#if hasUntracked}
              <button class="overflow-menu-item" onclick={() => { showOverflowMenu = false; showCleanDialog = true; }}>
                <span class="nf">{"\uE20E"}</span> {m.changes_overflow_clean()}
              </button>
            {/if}
            <div class="overflow-separator"></div>
            <button class="overflow-menu-item" onclick={() => { showOverflowMenu = false; onNavigate?.('reflog'); }}>
              <span class="nf">{"\uF1DA"}</span> {m.changes_overflow_history()}
            </button>
            <button class="overflow-menu-item" onclick={() => { showOverflowMenu = false; handlePush(); }}>
              <span class="nf">{"\uF062"}</span> {m.changes_overflow_push()}
            </button>
          </div>
        {/if}
      </div>
    </div>

    <!-- Commit message textarea -->
    <textarea
      class="commit-input"
      placeholder={m.staging_commit_placeholder()}
      bind:value={message}
      onkeydown={(e) => { if (e.key === 'Enter' && e.metaKey) handleCommit(); }}
      data-testid="commit-message"
    ></textarea>

    <!-- Single commit button -->
    <Button
      variant="primary"
      disabled={!message.trim() || (!isAmend && staged.length === 0)}
      onclick={handleCommit}
      testid="commit-btn"
    >
      {isAmend
        ? m.staging_amend_button()
        : staged.length === 1
          ? m.staging_commit_button_one({ count: String(staged.length) })
          : m.staging_commit_button({ count: String(staged.length) })}
    </Button>

    {#if showPatchDialog}
      <div class="patch-source-dialog">
        <label class="radio-label">
          <input type="radio" bind:group={patchStagedOnly} value={true} />
          {m.patch_staged_only()}
        </label>
        <label class="radio-label">
          <input type="radio" bind:group={patchStagedOnly} value={false} />
          {m.patch_all_changes()}
        </label>
        <div class="patch-dialog-actions">
          <Button variant="neutral" size="sm" onclick={handleCreatePatch}>
            {m.patch_create_changes()}
          </Button>
          <Button variant="neutral" size="sm" onclick={() => { showPatchDialog = false; }}>
            {m.patch_cancel()}
          </Button>
        </div>
      </div>
    {/if}
  </div>

  {#if showCleanDialog}
    <CleanDialog onClose={() => showCleanDialog = false} />
  {/if}
</div>

<style>
  .staging-area {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .file-lists {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .commit-box {
    padding: 10px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex-shrink: 0;
  }

  .commit-input {
    width: 100%;
    min-height: 68px;
    resize: vertical;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 10px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: inherit;
    outline: none;
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }

  .commit-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.5;
  }

  .commit-input:focus {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px var(--overlay-accent-blue);
  }

  .nf {
    font-family: var(--font-icons);
    font-size: 13px;
    line-height: 1;
  }

  /* ── Commit toolbar ─────────────────────────────────── */

  .commit-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .toolbar-actions {
    display: flex;
    gap: 2px;
    margin-left: auto;
    position: relative;
  }

  /* ── Overflow menu ───────────────────────────────────── */

  .overflow-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    min-width: 180px;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px;
    z-index: 10;
    box-shadow: 0 4px 12px var(--overlay-shadow);
  }

  .overflow-menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    background: none;
    border: none;
    border-radius: 5px;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    transition: background 0.1s ease;
  }

  .overflow-menu-item:hover {
    background: var(--overlay-hover);
  }

  .overflow-menu-item .nf {
    font-size: 13px;
    color: var(--text-secondary);
    width: 16px;
    text-align: center;
  }

  .overflow-separator {
    height: 1px;
    background: var(--border);
    margin: 4px 8px;
  }


  .amend-toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: color 0.15s ease;
  }

  .amend-toggle:hover {
    color: var(--text-primary);
  }

  .amend-toggle input[type="checkbox"] {
    margin: 0;
    accent-color: var(--accent-primary);
  }

  .patch-source-dialog {
    padding: 10px 12px;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-top: 4px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .radio-label input[type="radio"] {
    accent-color: var(--accent-primary);
  }

  .patch-dialog-actions {
    display: flex;
    gap: 6px;
    margin-top: 8px;
    justify-content: flex-end;
  }
</style>
