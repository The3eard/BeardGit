<!--
  FileStatusBadge — the single, shared file-status indicator.

  A small colour-coded square holding the status letter (A/M/D/R/C/U/!/?).
  Replaces the three hand-rolled status renderers (Changes, FileChangeList,
  MR/PR diff) so the same concept reads identically everywhere. Status
  strings from either backend vocabulary are normalised via
  `normalizeFileStatus`. Colours are theme tokens; modified stays orange
  (copper is reserved for the active view + primary actions).
-->
<script lang="ts">
  import { normalizeFileStatus, type FileStatusKind } from "$lib/utils/fileStatus";
  import * as m from "$lib/paraglide/messages";

  let { status }: { status: string } = $props();

  const info = $derived(normalizeFileStatus(status));

  const LABELS: Record<FileStatusKind, () => string> = {
    added: m.file_status_added,
    modified: m.file_status_modified,
    deleted: m.file_status_deleted,
    renamed: m.file_status_renamed,
    copied: m.file_status_copied,
    untracked: m.file_status_untracked,
    conflicted: m.file_status_conflicted,
    unknown: m.file_status_unknown,
  };
  const label = $derived(LABELS[info.kind]());
</script>

<span class="file-status-badge is-{info.kind}" title={label} aria-label={label}>
  {info.letter}
</span>

<style>
  .file-status-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-2xs);
    font-weight: 700;
    line-height: 1;
    background: color-mix(in srgb, var(--st) 18%, transparent);
    color: var(--st);
  }

  .is-added {
    --st: var(--accent-green);
  }
  .is-modified {
    --st: var(--accent-orange);
  }
  .is-deleted {
    --st: var(--accent-red);
  }
  .is-renamed {
    --st: var(--accent-purple);
  }
  .is-copied {
    --st: var(--accent-blue);
  }
  .is-untracked {
    --st: var(--accent-blue);
  }
  .is-conflicted {
    --st: var(--accent-red);
  }
  /* The only kind whose colour is an audited text token, and so the only
     one that cannot afford the tinted fill. `background` is 18 % of `--st`
     over whatever is behind it, which lightens the surface under the
     letter in dark mode and darkens it in light — either way it eats
     contrast. Measured across all 31 themes, `--text-muted` on its own
     18 % tint bottoms out at 4.04:1 on the page and 3.37:1 on a panel,
     under the 4.5:1 the theme audit enforces. `--text-secondary` fares no
     better (4.65 / 3.82), so this is not fixable by picking a brighter
     token: nothing dimmer than `--text-primary` survives being drawn on a
     tint of itself. Unfilled, the letter sits on the plain surface, which
     is exactly the pair `audit_surfaces` measures.

     The seven other kinds keep the fill: accents carry no contrast floor
     and are far louder than the surface to begin with. */
  .is-unknown {
    --st: var(--text-muted);
    background: none;
  }
</style>
