<!--
  TaskEntryRow — renders a single `TaskEntry` inside the TasksPopover.

  Dispatches three concerns by `TaskEntry.kind`:

  1. **Icon.** Nerd-font glyphs for git ops and app updates; the shared
     `ProviderIcon` for AI kinds when the entry subtitle starts with a
     provider hint (`claude_code:` / `codex:` / `open_code:`), otherwise a
     neutral AI brain glyph.
  2. **Progress.** Determinate bar with % when `progress.percent` is
     known; indeterminate animated stripes otherwise.
  3. **Actions.** Whatever the aggregator attached to `entry.actions`.
     The row is declarative — it does NOT compute its own actions so the
     store stays the single source of truth.

  `onAction` lets the parent drawer route each click back to the router
  (`cancelTaskById` for `cancel`, `clearFinished` for `dismiss`, etc.).
-->
<script lang="ts">
  import type { TaskAction, TaskEntry, TaskKind } from "$lib/types/tasks";
  import type { AiProviderKind } from "$lib/types";
  import { formatRelativeTimeMs } from "$lib/utils/time";
  import ProviderIcon from "$lib/components/ai-sessions/ProviderIcon.svelte";
  import { Button } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    entry: TaskEntry;
    onAction: (id: TaskAction["id"]) => void;
  }

  const { entry, onAction }: Props = $props();

  /**
   * Nerd-font / FontAwesome glyphs per kind.
   *
   * Git kinds reuse `\uE725` (the git branch icon used throughout the
   * rest of the app); app updates use the FontAwesome download arrow
   * `\uF019`. AI kinds fall back to a brain glyph when no provider hint
   * is attached to the subtitle — the brand-icon branch in the template
   * takes priority when a hint IS present.
   */
  const kindGlyph = $derived.by(() => {
    switch (entry.kind) {
      case "git_fetch":
      case "git_pull":
      case "git_push":
      case "git_clone":
        return "\uE725"; // nf-dev-git_branch
      case "app_update":
        return "\uF019"; // fa-download
      case "ai_background":
      case "ai_interactive":
      case "ai_headless":
      default:
        // fa-lightbulb-o — same glyph the Changes toolbar uses for
        // the AI commit-message button so the drawer rows feel of-a-piece.
        return "\uF0EB";
    }
  });

  /**
   * Extract a provider hint from `entry.subtitle` when the producer
   * prefixes it with e.g. `"claude_code: fix/refactor-api"`. Returns
   * `null` when no known provider prefix is found.
   */
  const providerHint = $derived.by<AiProviderKind | null>(() => {
    if (!isAiKind(entry.kind)) return null;
    const title = `${entry.title} ${entry.subtitle ?? ""}`.toLowerCase();
    if (title.includes("claude")) return "claude_code";
    if (title.includes("codex")) return "codex";
    if (title.includes("opencode") || title.includes("open_code"))
      return "open_code";
    return null;
  });

  function isAiKind(kind: TaskKind): boolean {
    return (
      kind === "ai_background" ||
      kind === "ai_interactive" ||
      kind === "ai_headless"
    );
  }

  /** Short one-line subtitle, always suffixed with the relative time. */
  const subtitleLine = $derived.by(() => {
    const rel = formatRelativeTimeMs(entry.startedAt);
    if (entry.subtitle && rel) return `${entry.subtitle} • ${rel}`;
    return entry.subtitle ?? rel;
  });

  /** Localized kind label for a11y + small caption. */
  const kindLabel = $derived.by(() => {
    switch (entry.kind) {
      case "ai_background":
        return m.tasks_kind_ai_background();
      case "ai_interactive":
        return m.tasks_kind_ai_interactive();
      case "ai_headless":
        return m.tasks_kind_ai_headless();
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
</script>

<article
  class="task-row"
  data-testid="task-row"
  data-task-id={entry.id}
  data-task-kind={entry.kind}
  data-task-status={entry.status}
>
  <div class="task-row__header">
    <span
      class="task-row__icon"
      aria-label={kindLabel}
      data-testid="task-row-icon"
      data-kind={entry.kind}
    >
      {#if providerHint}
        <ProviderIcon provider={providerHint} size={14} />
      {:else}
        <span class="task-row__icon-glyph" aria-hidden="true"
          >{kindGlyph}</span
        >
      {/if}
    </span>
    <div class="task-row__head-text">
      <h4 class="task-row__title">{entry.title}</h4>
      {#if subtitleLine}
        <p class="task-row__subtitle">{subtitleLine}</p>
      {/if}
    </div>
  </div>

  {#if entry.progress}
    <div
      class="task-row__progress"
      data-testid="task-row-progress"
      data-determinate={entry.progress.determinate ? "true" : "false"}
    >
      {#if entry.progress.determinate && typeof entry.progress.percent === "number"}
        <div
          class="task-row__progress-bar"
          style="--progress: {entry.progress.percent}%"
          data-testid="task-row-progress-bar"
        ></div>
        <span class="task-row__progress-label">
          {entry.progress.percent}%
        </span>
      {:else}
        <div
          class="task-row__progress-bar task-row__progress-bar--indet"
          data-testid="task-row-progress-indet"
        ></div>
      {/if}
    </div>
  {/if}

  {#if entry.errorMessage}
    <p class="task-row__error" data-testid="task-row-error">
      {entry.errorMessage}
    </p>
  {/if}

  {#if entry.actions.length > 0}
    <!--
      Stop the click from bubbling up to the parent `<li>` in
      TasksPopover, which has an `onclick={openDetail}` handler. Without
      this, clicking Dismiss / Cancel / Retry on a row simultaneously
      fires the action AND opens the detail view, which is jarring (the
      user has to back out twice). Action buttons are self-contained and
      don't want the row's row-level interaction.
    -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="task-row__actions"
      onclick={(e) => e.stopPropagation()}
    >
      {#each entry.actions as action (action.id)}
        <Button
          variant={action.variant ?? "neutral"}
          size="sm"
          testid={`task-row-action-${action.id}`}
          onclick={() => onAction(action.id)}
        >
          {action.label}
        </Button>
      {/each}
    </div>
  {/if}
</article>

<style>
  .task-row {
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-primary);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .task-row__header {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .task-row__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .task-row__icon-glyph {
    font-family: var(--font-icons);
    font-size: var(--font-size-md);
    color: var(--text-secondary);
    line-height: 1;
  }

  .task-row__head-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .task-row__title {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .task-row__subtitle {
    margin: 0;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .task-row__progress {
    position: relative;
    height: 6px;
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
    border-radius: 3px;
    overflow: hidden;
  }

  .task-row__progress-bar {
    height: 100%;
    background: var(--accent-primary);
    width: var(--progress, 100%);
    transition: width 0.15s ease;
  }

  .task-row__progress-bar--indet {
    width: 30%;
    background: linear-gradient(
      90deg,
      transparent 0%,
      var(--accent-primary) 50%,
      transparent 100%
    );
    animation: task-row-indet 1.4s linear infinite;
  }

  @keyframes task-row-indet {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(333%);
    }
  }

  .task-row__progress-label {
    position: absolute;
    right: 6px;
    top: -14px;
    font-size: var(--font-size-2xs);
    color: var(--text-secondary);
  }

  .task-row__error {
    margin: 0;
    font-size: var(--font-size-xs);
    color: var(--accent-red);
  }

  .task-row__actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  /* Bump the hover delta on row action buttons (Dismiss, Cancel, Retry,
     …). The shared `.bg-btn--neutral:hover` overlay is rgba(255,255,255,
     0.06) which is barely visible against the row's `--bg-primary`
     backdrop — easy to miss on a light theme and easy to interpret as
     "this button isn't interactive". This rule lifts the hover to a
     ~12% mix of the foreground color so the affordance reads cleanly
     in both themes without touching the global Button styles. */
  :global(.task-row__actions .bg-btn--neutral:hover:not(:disabled)) {
    background: color-mix(in srgb, var(--text-primary) 12%, var(--bg-secondary));
  }
</style>
