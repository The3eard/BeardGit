<!--
  RepoSlot — live git summary for the ACTIVE project: branch name plus
  the starship-style counters (↑ ahead, ↓ behind, + staged, ! modified,
  ? untracked, ⚑ stashes).

  This used to live in the native window title; with the title bar
  merged into the tab strip (`titleBarStyle: Overlay` / frameless) the
  status bar is its new home. Reads `activeRepoStatus`, which is kept
  fresh by the same mutation-driven pipeline that updates the OS title.
  Collapses when a terminal tab or the welcome screen is active.

  Clicking navigates to the Changes view, where the counters live.
-->
<script lang="ts">
  import { activeRepoStatus } from "$lib/stores/projects";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Navigate to a main view (e.g. "changes"). */
    onOpenView: (view: string) => void;
  }

  const { onOpenView }: Props = $props();

  let counters = $derived.by(() => {
    const s = $activeRepoStatus;
    if (!s) return [] as { kind: string; glyph: string; n: number }[];
    const out: { kind: string; glyph: string; n: number }[] = [];
    if (s.ahead > 0) out.push({ kind: "ahead", glyph: "↑", n: s.ahead });
    if (s.behind > 0) out.push({ kind: "behind", glyph: "↓", n: s.behind });
    if (s.staged > 0) out.push({ kind: "staged", glyph: "+", n: s.staged });
    if (s.unstaged > 0) out.push({ kind: "modified", glyph: "!", n: s.unstaged });
    if (s.untracked > 0) out.push({ kind: "untracked", glyph: "?", n: s.untracked });
    if (s.stash_count > 0) out.push({ kind: "stash", glyph: "⚑", n: s.stash_count });
    return out;
  });
</script>

{#if $activeRepoStatus}
  <div class="repo-slot" data-testid="statusbar-repo-slot">
    <button
      class="repo-pill"
      type="button"
      title={m.statusbar_repo_open_changes()}
      onclick={() => onOpenView("changes")}
    >
      <span class="branch-icon nf" aria-hidden="true">{""}</span>
      <span class="branch-name">{$activeRepoStatus.branch}</span>
      {#each counters as c (c.kind)}
        <span class="counter counter-{c.kind}">{c.glyph}{c.n}</span>
      {/each}
    </button>
  </div>
  <!-- Divider lives inside the slot so it collapses with it (a leading
       orphan divider would otherwise show on terminal/welcome tabs). -->
  <span class="slot-divider" aria-hidden="true"></span>
{/if}

<style>
  .repo-slot {
    display: inline-flex;
    align-items: center;
    height: 100%;
    padding: 0 8px;
    min-width: 0;
  }

  .slot-divider {
    width: 1px;
    background: var(--border);
    opacity: 0.4;
    align-self: center;
    height: 60%;
    flex-shrink: 0;
  }

  .repo-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: transparent;
    border: none;
    padding: 0;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-xs);
    cursor: pointer;
    user-select: none;
    min-width: 0;
    transition: color 0.15s;
  }

  .repo-pill:hover {
    color: var(--text-primary);
  }

  .branch-icon {
    font-family: var(--font-icons);
    font-size: var(--font-size-2xs);
    line-height: 1;
  }

  .branch-name {
    font-family: var(--font-mono);
    font-size: var(--font-size-2xs);
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .counter {
    font-size: var(--font-size-2xs);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .counter-ahead {
    color: var(--accent-green);
  }

  .counter-behind,
  .counter-modified {
    color: var(--accent-orange);
  }

  .counter-staged {
    color: var(--accent-primary);
  }

  .counter-untracked {
    color: var(--text-secondary);
  }

  .counter-stash {
    color: var(--accent-purple);
  }
</style>
