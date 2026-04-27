<!--
  TabStatusStrip — compact at-a-glance status pills shown on non-active
  project tabs. Mirrors the title bar's status grammar (↑ ahead, ↓ behind,
  ! modified, + staged, ? untracked, ⚑ stashes) so the user can monitor
  multiple repos without switching tabs. Reads from the in-memory
  `projectSnapshots` store, which is rewritten by the watcher pipeline on
  every external mutation (commit, push, stash, etc.).

  Active tab does NOT render this — its title bar already carries the
  same information.
-->
<script lang="ts">
  import type { ProjectSnapshot } from "$lib/types";

  interface Props {
    snapshot: ProjectSnapshot | null;
  }

  let { snapshot }: Props = $props();

  let segments = $derived.by(() => {
    if (!snapshot) return [] as { kind: string; glyph: string; n: number }[];
    const out: { kind: string; glyph: string; n: number }[] = [];
    if (snapshot.ahead > 0) out.push({ kind: "ahead", glyph: "↑", n: snapshot.ahead });
    if (snapshot.behind > 0) out.push({ kind: "behind", glyph: "↓", n: snapshot.behind });
    if (snapshot.unstaged > 0) out.push({ kind: "modified", glyph: "!", n: snapshot.unstaged });
    if (snapshot.staged > 0) out.push({ kind: "staged", glyph: "+", n: snapshot.staged });
    if (snapshot.untracked > 0) out.push({ kind: "untracked", glyph: "?", n: snapshot.untracked });
    if (snapshot.stash_count > 0) out.push({ kind: "stash", glyph: "⚑", n: snapshot.stash_count });
    return out;
  });
</script>

{#if segments.length > 0}
  <span class="tab-status-strip">
    {#each segments as seg (seg.kind)}
      <span class="seg seg-{seg.kind}">{seg.glyph}{seg.n}</span>
    {/each}
  </span>
{/if}

<style>
  .tab-status-strip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 500;
    line-height: 1;
  }

  .seg {
    padding: 1px 4px;
    border-radius: 6px;
    background: var(--overlay-accent-muted);
    color: var(--text-secondary);
  }

  .seg-ahead {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }

  .seg-behind,
  .seg-modified {
    background: color-mix(in srgb, var(--accent-orange) 15%, transparent);
    color: var(--accent-orange);
  }

  .seg-staged {
    background: color-mix(in srgb, var(--accent-blue) 15%, transparent);
    color: var(--accent-blue);
  }

  .seg-untracked {
    background: var(--overlay-accent-muted);
    color: var(--text-secondary);
  }

  .seg-stash {
    background: color-mix(in srgb, var(--accent-purple) 15%, transparent);
    color: var(--accent-purple);
  }
</style>
