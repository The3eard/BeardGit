<script lang="ts">
  /**
   * Detail pane for the AI Sessions view — branches on whichever of the
   * two selection stores is set.
   *
   * 1. `$selectedConversation` non-null → conversation metadata
   *    (provider, cwd, created/last-activity, optional forked-from,
   *    Resume button). No transcript rendering in this slice per Phase-5
   *    scope.
   * 2. `$selectedBackgroundSession` non-null → bg-run detail (status
   *    badge, transcript, cancel/open-terminal/switch/discard action
   *    buttons, token/cost lines). Body is a near-verbatim copy of the
   *    legacy bg-run branch.
   * 3. Otherwise → empty-state placeholder.
   *
   * The selection stores are mutually exclusive (the list rows clear the
   * other whenever one is set), so `$selectedConversation` takes
   * precedence here only as a defensive ordering — in practice at most
   * one is non-null.
   */
  import {
    aiBackgroundTranscripts,
    cancelAiBackgroundRun,
    discardAiBackgroundRunWorktree,
    openTerminalForAiBackgroundSession,
    selectedBackgroundSession,
  } from "$lib/stores/aiBackground";
  import { aiGetBackgroundReport } from "$lib/api/tauri";
  import { renderMarkdown } from "$lib/utils/markdown";
  import {
    selectedConversation,
  } from "$lib/stores/aiConversations";
  import {
    focusTerminal,
    resumeConversation,
  } from "$lib/stores/aiConversationActions";
  import { openProjectTab } from "$lib/stores/projects";
  import { runMutation } from "$lib/api/runMutation";
  import { addToast } from "$lib/stores/toast";
  import { providerName } from "$lib/data/ai-providers";
  import { formatDateTime, formatRelativeTimeMs } from "$lib/utils/time";
  import * as m from "$lib/paraglide/messages";
  import BackgroundRunStatusBadge from "../ai/BackgroundRunStatusBadge.svelte";
  import BackgroundRunTranscript from "../ai/BackgroundRunTranscript.svelte";
  import ProviderIcon from "./ProviderIcon.svelte";
  import {
    selectedActiveTerminal,
    type ActiveTerminal,
  } from "$lib/stores/aiActiveTerminals";

  // ─── Conversation branch data ───
  let conversation = $derived($selectedConversation);

  // ─── Background-run branch data ───
  let bgSession = $derived($selectedBackgroundSession);
  let transcript = $derived.by(() => {
    if (!bgSession) return [] as string[];
    return $aiBackgroundTranscripts.get(bgSession.id) ?? [];
  });
  let status = $derived(bgSession?.background_status ?? null);

  let tokenLine = $derived.by(() => {
    if (!status || status.state !== "completed") return null;
    const tu = status.token_usage;
    if (!tu) return null;
    return m.ai_background_token_usage({
      input: tu.input,
      output: tu.output,
    });
  });

  let costLine = $derived.by(() => {
    if (!status || status.state !== "completed") return null;
    const cost = status.token_usage?.total_cost_usd;
    if (cost == null) return null;
    return m.ai_background_token_cost({ cost: cost.toFixed(4) });
  });

  let isRunning = $derived(
    status?.state === "running" || status?.state === "queued",
  );
  let isTerminal = $derived(
    status?.state === "completed" ||
      status?.state === "failed" ||
      status?.state === "cancelled",
  );

  // ─── Handlers: bg-run branch ───

  /** Focus the bg-run's live PTY. */
  function handleFocusBg() {
    if (!bgSession) return;
    focusTerminal({ kind: "bg", session: bgSession });
  }

  async function handleOpenTerminal() {
    if (!bgSession || !bgSession.worktree_path || isRunning) return;
    const id = bgSession.id;
    try {
      await runMutation({
        kind: "ai_open_terminal",
        invoke: () => openTerminalForAiBackgroundSession(id),
        successToast: () => m.ai_background_open_terminal_success(),
        failureToastPrefix: m.ai_background_open_terminal_error(),
      });
    } catch {
      // runMutation already surfaced a sticky failure toast.
    }
  }

  async function handleDiscard() {
    if (!bgSession || !isTerminal) return;
    // eslint-disable-next-line no-alert
    if (!window.confirm(m.ai_background_discard_confirm())) return;
    const id = bgSession.id;
    try {
      await runMutation({
        kind: "ai_discard_worktree",
        invoke: () => discardAiBackgroundRunWorktree(id),
        failureToastPrefix: m.ai_background_discard_error(),
      });
    } catch {
      // runMutation already surfaced a sticky failure toast.
    }
  }

  async function handleSwitchToWorktree() {
    if (!bgSession?.worktree_path) return;
    try {
      await openProjectTab(bgSession.worktree_path);
    } catch (e) {
      console.error("failed to open worktree tab", e);
    }
  }

  async function handleCancel() {
    if (!bgSession || !isRunning) return;
    const id = bgSession.id;
    try {
      await runMutation({
        kind: "ai_cancel_run",
        invoke: () => cancelAiBackgroundRun(id),
        failureToastPrefix: m.ai_background_cancel_error(),
      });
    } catch {
      // runMutation already surfaced a sticky failure toast.
    }
  }

  // ─── Handlers: conversation branch ───

  /**
   * Prefer the conversation's stored title; fall back to the i18n'd
   * "(no title)" placeholder when the transcript's first user message
   * wasn't a parseable prompt.
   */
  let convTitle = $derived.by(() => {
    if (!conversation) return "";
    return conversation.title && conversation.title.trim().length > 0
      ? conversation.title
      : m.ai_sessions_no_title();
  });

  async function handleResumeConversation() {
    if (!conversation) return;
    try {
      await resumeConversation(conversation);
    } catch (err) {
      addToast({
        message: m.ai_sessions_resume_error({ error: String(err) }),
        type: "error",
      });
    }
  }

  // ─── Active (tab / segment) branch data ───

  /** Current tab/segment selection, or null when not in this branch. */
  let activeTermRaw = $derived($selectedActiveTerminal);

  /**
   * Only surface the active-pane branch for tab/segment selections —
   * bg-kind ActiveTerminals are rendered via the richer bg-run branch
   * keyed on `selectedBackgroundSessionId`.
   */
  type TabOrSegmentTerminal = Extract<ActiveTerminal, { kind: "tab" | "segment" }>;

  let activeTerm = $derived.by<TabOrSegmentTerminal | null>(() => {
    const sel = activeTermRaw;
    if (!sel) return null;
    if (sel.kind === "bg") return null;
    return sel;
  });

  /** Last path segment for compact display. Mirrors ActiveRow's helper. */
  function shortCwd(fullPath: string): string {
    const parts = fullPath.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? fullPath;
  }

  let activeProvider = $derived.by(() => {
    if (!activeTerm) return null;
    return activeTerm.info.provider!;
  });

  let activeTitle = $derived.by(() => {
    if (!activeTerm) return "";
    if (activeTerm.kind === "tab") return `Terminal ${activeTerm.tabIndex + 1}`;
    return `Terminal in ${shortCwd(activeTerm.info.cwd)}`;
  });

  let activeCwd = $derived(activeTerm?.info.cwd ?? null);

  function handleFocusActive() {
    if (!activeTerm) return;
    focusTerminal(activeTerm);
  }

  // ─── Prompt / transcript split ─────────────────────────────────────
  // Vertical split inside the bg-run branch. Defaults to 50/50; the
  // user drags the handle to resize, clamped so neither pane collapses
  // to zero height (keeps the section headers and copy button
  // reachable).
  let promptPct = $state(50);

  function startSplitResize(e: MouseEvent) {
    e.preventDefault();
    const target = (e.currentTarget as HTMLElement).parentElement;
    if (!target) return;
    const rect = target.getBoundingClientRect();
    const total = rect.height;
    const minPct = 15;
    const maxPct = 85;

    function onMouseMove(ev: MouseEvent) {
      const offset = ev.clientY - rect.top;
      const next = (offset / total) * 100;
      promptPct = Math.max(minPct, Math.min(maxPct, next));
    }
    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    }
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  // ─── Report fetch ──────────────────────────────────────────────────
  // Backend asks the AI to write a markdown report at
  // `<repo>/.beardgit/ai-reports/<session_id>.md`. We fetch it when:
  //   - selection changes to a different bg session
  //   - the run reaches a terminal state (where the AI just had a
  //     chance to finish writing)
  //   - the user presses the inline Refresh button below
  //
  // Result is the raw markdown string, or null when the file isn't on
  // disk yet (run still in flight, or AI didn't write one).
  let report = $state<string | null>(null);
  let reportLoading = $state(false);
  /** Bottom pane content selector: rendered report (default) or raw transcript. */
  let bottomView = $state<"report" | "transcript">("report");

  async function loadReport(sessionId: string) {
    reportLoading = true;
    try {
      const next = await aiGetBackgroundReport(sessionId);
      // Last-wins: only keep the result if the selection didn't move.
      if (bgSession?.id === sessionId) {
        report = next;
      }
    } catch {
      // The IPC layer surfaces a friendly toast for hard failures
      // (`get_active_project_path` errors when no project active). For
      // missing-file we already return None on the backend, so a thrown
      // error here means *something else* went wrong — surface as a
      // null report so the UI shows the empty state rather than crashes.
      if (bgSession?.id === sessionId) report = null;
    } finally {
      if (bgSession?.id === sessionId) reportLoading = false;
    }
  }

  // Refetch on bgSession.id change AND on terminal-state transition.
  // Using `untrack` would be more surgical but the cost is one IPC call
  // per status transition — negligible.
  let lastFetchKey = $state("");
  $effect(() => {
    if (!bgSession) {
      report = null;
      lastFetchKey = "";
      return;
    }
    const key = `${bgSession.id}:${bgSession.background_status?.state ?? ""}`;
    if (key === lastFetchKey) return;
    lastFetchKey = key;
    void loadReport(bgSession.id);
  });

  function handleRefreshReport() {
    if (bgSession) void loadReport(bgSession.id);
  }
</script>

{#if conversation}
  <!-- ─── Conversation branch ─── -->
  <div class="detail" data-testid="ai-session-detail-conversation">
    <header class="header">
      <div class="title-row">
        <ProviderIcon provider={conversation.provider} size={20} />
        <span class="provider">{providerName(conversation.provider)}</span>
      </div>
      <div class="title-line" data-testid="ai-session-detail-title">
        {convTitle}
      </div>
      <div class="wt-row">
        <code class="wt-path" data-testid="ai-session-detail-cwd">
          {conversation.cwd}
        </code>
      </div>
    </header>

    <dl class="meta-grid">
      <div class="meta-row">
        <dt>{m.ai_sessions_created_at({ when: formatDateTime(Math.floor(conversation.created_at / 1000)) })}</dt>
      </div>
      <div class="meta-row">
        <dt>{m.ai_sessions_last_activity({ when: formatRelativeTimeMs(conversation.last_activity_at) })}</dt>
      </div>
      {#if conversation.parent_id}
        <div class="meta-row" data-testid="ai-session-detail-forked">
          <dt>{m.ai_sessions_forked_from({ prefix: conversation.parent_id })}</dt>
        </div>
      {/if}
    </dl>

    <div class="actions">
      <button
        class="btn primary"
        onclick={handleResumeConversation}
        title={m.ai_sessions_resume_warning({
          provider: providerName(conversation.provider),
        })}
        data-testid="ai-session-detail-resume"
      >
        {m.ai_sessions_resume_in_new_terminal()}
      </button>
    </div>
  </div>
{:else if bgSession}
  <!-- ─── Background-run branch ─── -->
  <div class="detail" data-testid="ai-session-detail">
    <header class="header">
      <div class="title-row">
        <ProviderIcon provider={bgSession.provider} size={20} />
        <span class="provider">{providerName(bgSession.provider)}</span>
        <BackgroundRunStatusBadge status={bgSession.background_status!} />
        {#if !bgSession.worktree_path}
          <span class="external-badge" data-testid="external-badge">
            {m.ai_sessions_external()}
          </span>
        {/if}
      </div>
      <div class="wt-row">
        <code class="wt-path" data-testid="ai-session-detail-wt-path">
          {bgSession.worktree_path ?? bgSession.cwd}
        </code>
      </div>
    </header>

    {#if tokenLine}
      <div class="meta">
        <span>{tokenLine}</span>
        {#if costLine}<span class="dot">•</span><span>{costLine}</span>{/if}
      </div>
    {/if}

    <div class="actions">
      <button
        class="btn"
        onclick={handleFocusBg}
        data-testid="ai-session-detail-focus"
      >
        {m.ai_sessions_focus()}
      </button>
      {#if isRunning}
        <button class="btn danger" onclick={handleCancel}>
          {m.ai_background_cancel_run()}
        </button>
        {#if bgSession.worktree_path}
          <button
            class="btn"
            disabled
            title={m.ai_background_tooltip_terminal_running()}
            data-testid="ai-session-detail-open-terminal"
          >
            {m.ai_background_open_terminal()}
          </button>
        {/if}
      {:else if bgSession.worktree_path}
        <button
          class="btn"
          onclick={handleOpenTerminal}
          data-testid="ai-session-detail-open-terminal"
        >
          {m.ai_background_open_terminal()}
        </button>
      {/if}
      <button
        class="btn"
        onclick={handleSwitchToWorktree}
        disabled={!bgSession.worktree_path}
      >
        {m.ai_background_switch_to_worktree()}
      </button>
      {#if isTerminal}
        <button class="btn danger" onclick={handleDiscard}>
          {m.ai_background_discard_worktree()}
        </button>
      {/if}
    </div>

    <!-- Prompt + transcript split.
         Two stacked scroll regions instead of a single flex transcript:
         the captured stream-json output is verbose enough to dwarf the
         dialog's original prompt, so we pin the prompt above (≈50% of
         the remaining height by default) and let the user resize via
         the drag handle. -->
    <div class="split" style="--prompt-pct: {promptPct}%">
      <section class="split-pane prompt-pane" data-testid="ai-session-detail-prompt">
        <header class="split-header">
          <span class="split-title">{m.ai_background_section_prompt()}</span>
        </header>
        {#if bgSession.prompt && bgSession.prompt.trim().length > 0}
          <pre class="prompt-text">{bgSession.prompt}</pre>
        {:else}
          <p class="prompt-empty">{m.ai_background_no_prompt_recorded()}</p>
        {/if}
      </section>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="split-handle"
        onmousedown={startSplitResize}
      ></div>
      <section class="split-pane transcript-pane" data-testid="ai-session-detail-bottom">
        <header class="split-header">
          <span class="split-title">
            {bottomView === "report"
              ? m.ai_background_section_report()
              : m.ai_background_section_output()}
          </span>
          <div class="split-actions">
            {#if bottomView === "report"}
              <button
                class="split-link"
                onclick={handleRefreshReport}
                disabled={reportLoading}
                title={m.ai_background_report_refresh_tooltip()}
                data-testid="ai-session-detail-report-refresh"
              >
                {reportLoading
                  ? m.ai_background_report_loading()
                  : m.ai_background_report_refresh()}
              </button>
              <button
                class="split-link"
                onclick={() => (bottomView = "transcript")}
                data-testid="ai-session-detail-show-transcript"
              >
                {m.ai_background_show_transcript({
                  count: transcript.length,
                })}
              </button>
            {:else}
              <button
                class="split-link"
                onclick={() => (bottomView = "report")}
                data-testid="ai-session-detail-show-report"
              >
                {m.ai_background_show_report()}
              </button>
            {/if}
          </div>
        </header>
        {#if bottomView === "report"}
          {#if report && report.trim().length > 0}
            <article
              class="report-body"
              data-testid="ai-session-detail-report"
            >
              {@html renderMarkdown(report)}
            </article>
          {:else if reportLoading}
            <div class="output-empty" data-testid="ai-session-detail-report-loading">
              <p class="output-empty-title">{m.ai_background_report_loading_title()}</p>
            </div>
          {:else if isRunning}
            <div class="output-empty" data-testid="ai-session-detail-report-pending">
              <p class="output-empty-title">{m.ai_background_report_pending_title()}</p>
              <p class="output-empty-hint">{m.ai_background_report_pending_hint()}</p>
            </div>
          {:else}
            <div class="output-empty" data-testid="ai-session-detail-report-missing">
              <p class="output-empty-title">{m.ai_background_report_missing_title()}</p>
              <p class="output-empty-hint">{m.ai_background_report_missing_hint()}</p>
            </div>
          {/if}
        {:else if transcript.length === 0}
          <div class="output-empty" data-testid="ai-session-detail-output-empty">
            <p class="output-empty-title">{m.ai_background_output_empty_title()}</p>
            <p class="output-empty-hint">{m.ai_background_output_empty_hint()}</p>
          </div>
        {:else}
          <BackgroundRunTranscript lines={transcript} />
        {/if}
      </section>
    </div>
  </div>
{:else if activeTerm}
  <!-- ─── Active tab/segment branch ─── -->
  <div class="detail" data-testid="ai-session-detail-active">
    <header class="header">
      <div class="title-row">
        {#if activeProvider}
          <ProviderIcon provider={activeProvider} size={20} />
          <span class="provider">{providerName(activeProvider)}</span>
        {/if}
      </div>
      <div class="title-line" data-testid="ai-session-detail-active-title">
        {activeTitle}
      </div>
      {#if activeCwd}
        <div class="wt-row">
          <code class="wt-path" data-testid="ai-session-detail-active-cwd">
            {activeCwd}
          </code>
        </div>
      {/if}
    </header>

    <div class="actions">
      <button
        class="btn primary"
        onclick={handleFocusActive}
        data-testid="ai-session-detail-focus"
      >
        {m.ai_sessions_focus()}
      </button>
    </div>
  </div>
{:else}
  <div class="empty" data-testid="ai-session-detail-empty">
    {m.ai_sessions_empty()}
  </div>
{/if}

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    flex: 1;
    min-height: 0;
  }

  .header {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .provider {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    text-transform: capitalize;
  }

  .title-line {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.4;
    word-break: break-word;
  }

  .external-badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 1px 6px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--text-secondary) 15%, transparent);
    color: var(--text-secondary);
    border: 1px solid color-mix(in srgb, var(--text-secondary) 30%, transparent);
  }

  .wt-row {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .wt-path {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-blue);
    word-break: break-all;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .meta-grid {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
  }

  .meta-row {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .meta-row dt {
    margin: 0;
    display: inline;
  }

  .dot {
    opacity: 0.4;
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .btn {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    padding: 4px 10px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn:hover:not(:disabled) {
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Tonal at rest, solid on hover. The button is the primary action in
     the panel but it sits next to a "Cancel" / "Discard" pair, so a
     full-saturation accent-blue rest state read as "already
     highlighted" — and the cascading `.btn:hover` rule turned the text
     accent-blue, hiding it against the accent-blue fill. Using a
     translucent tint at rest keeps it clearly the primary CTA without
     screaming, and the hover ramps up to the full accent so the
     interactive feedback is unmistakable. */
  .btn.primary {
    background: color-mix(in srgb, var(--accent-blue) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent-blue) 60%, transparent);
    color: var(--accent-blue);
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    /* Re-state text-primary explicitly so the cascading `.btn:hover`
       (which would set color to accent-blue, matching the new fill
       and hiding the label) cannot win on the `color` property. */
    color: var(--text-primary);
  }

  .btn.danger {
    color: var(--accent-red);
    border-color: color-mix(in srgb, var(--accent-red) 40%, transparent);
  }

  .btn.danger:hover {
    background: color-mix(in srgb, var(--accent-red) 10%, transparent);
  }

  .empty {
    padding: 24px;
    color: var(--text-secondary);
    text-align: center;
    font-size: 12px;
  }

  /* ─── Prompt + transcript split ─────────────────────────────────── */

  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    /* `--prompt-pct` set inline from the script's `promptPct` state.
       The 6px row keeps the drag handle outside both pane percentages
       so the math matches what the user feels when dragging. */
    grid-template-rows: calc(var(--prompt-pct, 50%) - 3px) 6px 1fr;
    border-top: 1px solid var(--border);
  }

  .split-pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .split-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 10px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 8px;
  }

  .split-title {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .split-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Inline link-style buttons for the bottom-pane header (Refresh +
     swap between Report and Transcript). Quiet at rest so they don't
     compete with the Prompt header above; light up on hover. */
  .split-link {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
  }

  .split-link:hover:not(:disabled) {
    color: var(--accent-blue);
  }

  .split-link:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Rendered markdown body of the AI report. Matches the surrounding
     dark theme — backgrounds and links pull from the same CSS tokens
     used elsewhere so the report doesn't look like a foreign element
     pasted into the panel. */
  .report-body {
    flex: 1;
    overflow: auto;
    padding: 12px 14px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 12px;
    line-height: 1.55;
  }

  .report-body :global(h1),
  .report-body :global(h2),
  .report-body :global(h3) {
    margin: 14px 0 8px;
    color: var(--text-primary);
  }
  .report-body :global(h1) { font-size: 15px; }
  .report-body :global(h2) { font-size: 13px; }
  .report-body :global(h3) { font-size: 12px; }
  .report-body :global(p) { margin: 6px 0; }
  .report-body :global(ul),
  .report-body :global(ol) {
    margin: 6px 0;
    padding-left: 22px;
  }
  .report-body :global(li) { margin: 2px 0; }
  .report-body :global(code) {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 1px 4px;
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
    border-radius: 3px;
  }
  .report-body :global(pre) {
    background: color-mix(in srgb, var(--text-primary) 5%, transparent);
    padding: 8px 10px;
    border-radius: 4px;
    overflow: auto;
    font-size: 11px;
  }
  .report-body :global(pre code) {
    background: transparent;
    padding: 0;
  }
  .report-body :global(a) {
    color: var(--accent-blue);
    text-decoration: none;
  }
  .report-body :global(a:hover) {
    text-decoration: underline;
  }
  .report-body :global(blockquote) {
    margin: 6px 0;
    padding-left: 10px;
    border-left: 2px solid var(--border);
    color: var(--text-secondary);
  }

  .output-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 16px;
    background: var(--bg-primary);
    text-align: center;
  }

  .output-empty-title {
    margin: 0;
    font-size: 12px;
    color: var(--text-primary);
    font-weight: 500;
  }

  .output-empty-hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
    font-style: italic;
    max-width: 360px;
    line-height: 1.5;
  }

  .prompt-text {
    margin: 0;
    flex: 1;
    overflow: auto;
    padding: 8px 10px;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--bg-primary);
  }

  .prompt-empty {
    margin: 0;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 8px 10px;
    font-size: 11px;
    color: var(--text-secondary);
    font-style: italic;
    background: var(--bg-primary);
  }

  /* Slim drag affordance between the two panes. The visible 6px row
     ships an underline-style hover state — no centred grip dots since
     the surrounding header bars already telegraph "two stacked
     panels". */
  .split-handle {
    cursor: row-resize;
    background: var(--border);
    transition: background 0.15s;
  }

  .split-handle:hover {
    background: var(--accent-blue);
  }

  .transcript-pane {
    /* `BackgroundRunTranscript` already provides its own bordered box;
       the pane just needs to give it a flex slot to fill. */
  }
</style>
