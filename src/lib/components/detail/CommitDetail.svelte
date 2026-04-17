<script lang="ts">
  import type { CommitInfo, CommitFileChange } from "../../types";
  import * as m from "$lib/paraglide/messages";
  import FileChangeList from "../common/FileChangeList.svelte";
  import ContextMenu from "../common/ContextMenu.svelte";
  import type { MenuItem } from "../common/ContextMenu.svelte";
  import Xrefs from "../common/Xrefs.svelte";
  import { hashString as _hashString } from "$lib/utils/ref-colors";
  import { openBlame, blameActiveTab } from "$lib/stores/blame";
  import { formatDateTime } from "../../utils/time";

  let {
    commit,
    files = [],
    showNavigateToGraph = false,
    onNavigateToGraph,
    onClose,
    onFileClick,
    onNavigate,
  }: {
    commit: CommitInfo;
    files?: CommitFileChange[];
    showNavigateToGraph?: boolean;
    onNavigateToGraph?: (oid: string) => void;
    onClose?: () => void;
    onFileClick?: (path: string) => void;
    onNavigate?: (view: string) => void;
  } = $props();

  let ctxVisible = $state(false);
  let ctxX = $state(0);
  let ctxY = $state(0);
  let ctxFile = $state<string | null>(null);

  function openFileContextMenu(e: MouseEvent, path: string) {
    e.preventDefault();
    ctxFile = path;
    ctxX = e.clientX;
    ctxY = e.clientY;
    ctxVisible = true;
  }

  function buildFileContextItems(path: string): MenuItem[] {
    return [
      {
        label: m.context_blame(),
        action: () => {
          openBlame(path, commit.oid);
          onNavigate?.('blame');
        },
      },
      {
        label: m.context_file_history(),
        action: () => {
          openBlame(path, commit.oid);
          blameActiveTab.set('history');
          onNavigate?.('blame');
        },
      },
    ];
  }

  function handleFileSelect(path: string) {
    onFileClick?.(path);
  }

  function formatRef(ref: string): string {
    if (ref.startsWith("refs/heads/")) return ref.replace("refs/heads/", "");
    if (ref.startsWith("refs/remotes/")) return ref.replace("refs/remotes/", "");
    if (ref.startsWith("refs/tags/")) return ref.replace("refs/tags/", "");
    if (ref === "HEAD") return "HEAD";
    return ref;
  }

  const REF_COLORS = [
    { color: 'var(--accent-blue)', rgb: '88, 166, 255' },
    { color: 'var(--accent-green)', rgb: '63, 185, 80' },
    { color: 'var(--accent-orange)', rgb: '240, 136, 62' },
    { color: 'var(--accent-purple)', rgb: '188, 140, 255' },
    { color: 'var(--accent-red)', rgb: '248, 81, 73' },
  ];

  /** Delegates to the shared ref-colors utility for consistent hashing. */
  const hashString = _hashString;

  function refColorIndex(ref: string): number {
    if (ref === "HEAD") return -1;
    return hashString(formatRef(ref)) % REF_COLORS.length;
  }

  function refStyle(ref: string): string {
    if (ref === "HEAD") {
      return 'color: #f778ba; background: rgba(247, 120, 186, 0.12); border: 1px solid rgba(247, 120, 186, 0.3)';
    }
    const idx = refColorIndex(ref);
    const { color, rgb } = REF_COLORS[idx];
    return `color: ${color}; background: rgba(${rgb}, 0.12); border: 1px solid rgba(${rgb}, 0.3)`;
  }

</script>

<aside class="commit-detail">
  <div class="detail-header">
    <h3 class="detail-title">{m.commit_detail_title()}</h3>
    <div class="detail-header-actions">
      {#if showNavigateToGraph && onNavigateToGraph}
        <button class="header-btn navigate-btn" onclick={() => onNavigateToGraph!(commit.oid)} title="Show in Graph">
          ↗ Graph
        </button>
      {/if}
      {#if onClose}
        <button class="close-btn" onclick={() => onClose!()}>
          {"\uF00D"}
        </button>
      {/if}
    </div>
  </div>

  <div class="detail-body">
    <div class="detail-section">
      <div class="commit-summary">{commit.summary}</div>
      {#if commit.body}
        <div class="commit-body"><Xrefs text={commit.body} /></div>
      {/if}
    </div>

    <div class="detail-section">
      <div class="detail-row">
        <span class="detail-label">{m.commit_detail_author()}</span>
        <span class="detail-value">{commit.author}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">{m.commit_detail_email()}</span>
        <span class="detail-value email">{commit.email}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">{m.commit_detail_date()}</span>
        <span class="detail-value">{formatDateTime(commit.timestamp)}</span>
      </div>
    </div>

    <div class="detail-section">
      <div class="detail-row">
        <span class="detail-label">{m.commit_detail_sha()}</span>
        <span class="detail-value sha">{commit.oid}</span>
      </div>
    </div>

    {#if commit.parents.length > 0}
      <div class="detail-section">
        <div class="detail-label">{m.commit_detail_parents()}</div>
        {#each commit.parents as parent}
          {#if onNavigateToGraph}
            <button class="parent-oid clickable" onclick={() => onNavigateToGraph!(parent)}>
              {parent.substring(0, 12)}
            </button>
          {:else}
            <span class="parent-oid">{parent.substring(0, 12)}</span>
          {/if}
        {/each}
      </div>
    {/if}

    {#if commit.refs.length > 0}
      <div class="detail-section">
        <div class="detail-label">{m.commit_detail_refs()}</div>
        <div class="ref-list">
          {#each commit.refs as ref}
            <span class="ref-badge" style={refStyle(ref)}>{formatRef(ref)}</span>
          {/each}
        </div>
      </div>
    {/if}

    {#if files.length > 0}
      <div class="detail-section">
        <div class="detail-label">{m.commit_detail_files({ count: String(files.length) })}</div>
        <FileChangeList files={files} onSelect={handleFileSelect} onContextMenu={openFileContextMenu} />
      </div>
    {/if}
  </div>
</aside>

<ContextMenu
  items={ctxFile ? buildFileContextItems(ctxFile) : []}
  x={ctxX}
  y={ctxY}
  visible={ctxVisible}
  onClose={() => (ctxVisible = false)}
/>

<style>
  .commit-detail {
    min-width: 0;
    flex: 1;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .detail-header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .detail-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    font-family: var(--font-icons);
    padding: 2px 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .header-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    font-family: var(--font-icons);
    padding: 2px 4px;
    border-radius: 3px;
  }

  .header-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }

  .navigate-btn {
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    font-size: 11px;
    letter-spacing: 0.3px;
  }

  .detail-body {
    padding: 0;
  }

  .detail-section {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .detail-section:last-child {
    border-bottom: none;
  }

  .commit-summary {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.4;
    word-break: break-word;
  }

  .commit-body {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .detail-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 4px;
  }

  .detail-row:last-child {
    margin-bottom: 0;
  }

  .detail-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    min-width: 48px;
    flex-shrink: 0;
  }

  .detail-value {
    font-size: 12px;
    color: var(--text-primary);
    word-break: break-all;
  }

  .detail-value.email {
    color: var(--accent-blue);
  }

  .detail-value.sha {
    font-family: "SF Mono", "Fira Code", "Consolas", monospace;
    font-size: 11px;
    color: var(--accent-orange);
    word-break: break-all;
  }

  .parent-oid {
    font-family: "SF Mono", "Fira Code", "Consolas", monospace;
    font-size: 11px;
    color: var(--accent-blue);
    margin-top: 4px;
    cursor: default;
  }

  .parent-oid.clickable {
    background: none;
    border: none;
    padding: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent-blue);
    cursor: pointer;
  }

  .parent-oid.clickable:hover {
    text-decoration: underline;
  }

  .ref-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }

  .ref-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 500;
    background: none;
    cursor: default;
  }


</style>
