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
  .is-unknown {
    --st: var(--text-muted);
  }
</style>
