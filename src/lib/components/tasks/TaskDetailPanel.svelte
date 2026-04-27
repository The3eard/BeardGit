<!--
  TaskDetailPanel — drill-down view for a single task entry.

  Rendered inside the `TasksPopover` when the user clicks a row to
  inspect details. Streams three data sources through a kind-aware
  lookup so every producer can surface its output without inventing a
  new transport:

  - **Git ops / AI interactive** — numeric `TaskId` entries map 1:1
    into the legacy `taskPanel.ts` `taskOutput` map (populated by
    `task-output` events from the Rust `TaskManager`). Missing output
    is back-filled from `get_task_output` on first open so history
    entries that started before the user opened the popover still
    render their buffer.
  - **AI background** — entries with the `ai-background:<session_id>`
    id read from `aiBackgroundTranscripts`, which is the bridge the AI
    coordinator already maintains for the session viewer.
  - **App update** — has no console stream; we render only the
    metadata header (status + progress live on the row itself in list
    mode). Tasks without captured output deliberately render no
    placeholder block — the empty zone read as broken state, so we
    just let the meta header stand alone.

  Kept as a dedicated component so the popover shell stays thin and
  the detail view can grow independently (scrollback, copy button,
  follow toggle) without bloating the list template.
-->
<script lang="ts">
  import type { TaskEntry } from "$lib/types/tasks";
  import type { TaskId, TaskOutputLine } from "$lib/types";
  import { formatRelativeTimeMs } from "$lib/utils/time";
  import { taskOutput, selectTask } from "$lib/stores/taskPanel";
  import { aiBackgroundRuns, aiBackgroundTranscripts } from "$lib/stores/aiBackground";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    entry: TaskEntry;
  }

  const { entry }: Props = $props();

  /** Stable prefix the aggregator uses for AI-background entry ids. */
  const AI_BACKGROUND_PREFIX = "ai-background:";

  /**
   * Resolve the numeric `TaskId` an entry maps to for `taskOutput`
   * lookups. Returns `null` when the entry's id is not an int-parsable
   * string (so AI-background + auto-update fall through the AI
   * transcript / placeholder branches instead).
   */
  const legacyTaskId = $derived.by<TaskId | null>(() => {
    if (entry.id.startsWith(AI_BACKGROUND_PREFIX)) return null;
    if (entry.id === "auto-update") return null;
    const n = Number.parseInt(entry.id, 10);
    return Number.isFinite(n) ? n : null;
  });

  /**
   * Session id for AI background runs — strips the aggregator prefix
   * so we can key into `aiBackgroundTranscripts` and
   * `aiBackgroundRuns` directly.
   */
  const aiSessionId = $derived(
    entry.id.startsWith(AI_BACKGROUND_PREFIX)
      ? entry.id.slice(AI_BACKGROUND_PREFIX.length)
      : null,
  );

  /**
   * For AI background entries whose `task_id` is known, we can ALSO
   * tap into `taskOutput` (the coordinator forwards every line
   * through both channels). Using it gives us streaming stdout with
   * stream-labelled colouring identical to git ops.
   */
  const aiBackgroundTaskId = $derived.by<TaskId | null>(() => {
    if (!aiSessionId) return null;
    const session = $aiBackgroundRuns.get(aiSessionId);
    return session?.task_id ?? null;
  });

  /**
   * Effective numeric key used for the `taskOutput` map — either the
   * direct legacy id or the resolved AI-background `task_id`.
   */
  const effectiveTaskId = $derived(legacyTaskId ?? aiBackgroundTaskId);

  /**
   * Back-fill `taskOutput` from the backend when the user opens a
   * detail view whose buffer is absent/empty. Safe to call repeatedly;
   * `selectTask` skips the fetch when the local buffer already has
   * lines. We do NOT rely on `selectedTaskId` here — multiple detail
   * views can coexist without fighting over the legacy singleton.
   */
  $effect(() => {
    const id = effectiveTaskId;
    if (id === null) return;
    void selectTask(id);
  });

  /**
   * Combined output lines. Priority order:
   *
   *   1. `taskOutput.get(effectiveTaskId)` when a numeric id resolves.
   *   2. `aiBackgroundTranscripts.get(aiSessionId)` otherwise.
   *   3. `[]` — triggers the "no output" placeholder.
   */
  type RenderedLine = { stream: "stdout" | "stderr"; text: string };

  const outputLines = $derived.by<RenderedLine[]>(() => {
    if (effectiveTaskId !== null) {
      const fromLegacy = $taskOutput.get(effectiveTaskId);
      if (fromLegacy && fromLegacy.length > 0) {
        return fromLegacy.map((l: TaskOutputLine) => ({
          stream: l.stream,
          text: l.text,
        }));
      }
    }
    if (aiSessionId) {
      const transcript = $aiBackgroundTranscripts.get(aiSessionId) ?? [];
      return transcript.map((text: string) => ({
        stream: "stdout" as const,
        text,
      }));
    }
    return [];
  });

  const statusLabel = $derived.by(() => {
    switch (entry.status) {
      case "running":
        return m.tasks_status_running();
      case "success":
        return m.tasks_status_completed();
      case "error":
        return m.tasks_status_failed();
      case "cancelled":
        return m.tasks_status_cancelled();
    }
  });

  const kindLabel = $derived.by(() => {
    switch (entry.kind) {
      case "ai_background":
        return m.tasks_kind_ai_background();
      case "ai_interactive":
        return m.tasks_kind_ai_interactive();
      case "git_fetch":
        return m.tasks_kind_git_fetch();
      case "git_pull":
        return m.tasks_kind_git_pull();
      case "git_push":
        return m.tasks_kind_git_push();
      case "git_clone":
        return m.tasks_kind_git_clone();
      case "app_update":
        return m.tasks_kind_app_update();
    }
  });

  const relativeStarted = $derived(formatRelativeTimeMs(entry.startedAt));

  /**
   * Auto-scroll target. When the user hasn't manually scrolled away
   * from the bottom, new lines appended to the pre should keep the
   * viewport pinned so they see the latest output. Managed purely in
   * CSS via `overflow-anchor: auto`; no JS scroll hackery needed for
   * the first slice.
   */
  let outputEl: HTMLPreElement | undefined = $state();
  let stickToBottom = $state(true);

  function handleScroll() {
    if (!outputEl) return;
    const slack = 16;
    stickToBottom =
      outputEl.scrollTop + outputEl.clientHeight >=
      outputEl.scrollHeight - slack;
  }

  $effect(() => {
    // Re-read so the effect reactively fires when output grows.
    void outputLines.length;
    if (!outputEl || !stickToBottom) return;
    outputEl.scrollTop = outputEl.scrollHeight;
  });
</script>

<section class="detail" data-testid="task-detail-panel">
  <header class="detail__meta">
    <dl>
      <div class="detail__meta-row">
        <dt>Kind</dt>
        <dd data-testid="task-detail-kind">{kindLabel}</dd>
      </div>
      <div class="detail__meta-row">
        <dt>Status</dt>
        <dd data-testid="task-detail-status" data-status={entry.status}>
          {statusLabel}
        </dd>
      </div>
      <div class="detail__meta-row">
        <dt>Started</dt>
        <dd data-testid="task-detail-started">{relativeStarted}</dd>
      </div>
      {#if entry.subtitle}
        <div class="detail__meta-row">
          <dt>Context</dt>
          <dd data-testid="task-detail-subtitle">{entry.subtitle}</dd>
        </div>
      {/if}
      {#if entry.errorMessage}
        <div class="detail__meta-row">
          <dt>Error</dt>
          <dd class="detail__error" data-testid="task-detail-error">
            {entry.errorMessage}
          </dd>
        </div>
      {/if}
    </dl>
  </header>

  {#if outputLines.length > 0}
    <pre
      class="detail__output"
      data-testid="task-detail-output"
      bind:this={outputEl}
      onscroll={handleScroll}>{#each outputLines as line (`${line.stream}:${line.text}`)}<span
          class="detail__line detail__line--{line.stream}"
          data-stream={line.stream}>{line.text}</span
        >{"\n"}{/each}</pre>
  {/if}
</section>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .detail__meta {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    flex: 0 0 auto;
  }

  .detail__meta dl {
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 10px;
  }

  .detail__meta-row {
    display: contents;
  }

  .detail__meta-row dt {
    color: var(--text-secondary);
    font-size: 11px;
    margin: 0;
  }

  .detail__meta-row dd {
    color: var(--text-primary);
    font-size: 11px;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail__error {
    color: var(--accent-red);
    white-space: pre-wrap !important;
  }

  .detail__output {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--bg-primary);
    padding: 8px 10px;
    margin: 0;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.45;
    color: var(--text-primary);
    white-space: pre;
    overflow-anchor: auto;
  }

  .detail__line {
    display: inline;
  }

  .detail__line--stderr {
    color: var(--accent-red);
  }
</style>
