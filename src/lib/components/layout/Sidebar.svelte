<script lang="ts">
  import { repoInfo } from "../../stores/repo";
  import { fileStatuses } from "../../stores/changes";
  import { hasActiveProvider, activeProvider } from "../../stores/provider";
  import * as m from "$lib/paraglide/messages";

  let { onNavigate }: { onNavigate?: (view: string) => void } = $props();

  type NavItem = { label: string; icon: string; id: string };

  const navItems: NavItem[] = [
    { label: m.sidebar_graph(), icon: "\uE728", id: "graph" },
    { label: m.sidebar_changes(), icon: "\uF440", id: "changes" },
    { label: m.sidebar_branches(), icon: "\uE725", id: "branches" },
    { label: m.sidebar_tags(), icon: "\uF02B", id: "tags" },
    { label: m.sidebar_stashes(), icon: "\uF187", id: "stashes" },
  ];

  const providerItems: NavItem[] = [
    { label: m.sidebar_pipelines(), icon: "\uF144", id: "pipelines" },
    { label: m.sidebar_merge_requests(), icon: "\uF407", id: "merge-requests" },
  ];

  let activeNav = $state("graph");

  function handleNav(id: string) {
    activeNav = id;
    onNavigate?.(id);
  }

  let changeCount = $derived($fileStatuses.length);
</script>

<aside class="sidebar">
  <nav class="nav-section">
    <div class="section-label">{m.sidebar_navigation()}</div>
    {#each navItems as item}
      <button
        class="nav-item"
        class:active={activeNav === item.id}
        onclick={() => handleNav(item.id)}
      >
        <span class="nav-icon">{item.icon}</span>
        <span class="nav-label">{item.label}</span>
        {#if item.id === "changes" && changeCount > 0}
          <span class="nav-badge">{changeCount}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="nav-section">
    <div class="section-label">{m.sidebar_worktrees()}</div>
    <div class="placeholder-text wip">
      {m.worktrees_coming_soon()}
    </div>
  </div>

  <nav class="nav-section">
    <div class="section-label">
      <span class="provider-status-dot" class:connected={$hasActiveProvider}></span>
      {$activeProvider?.kind === 'github' ? m.provider_github() : m.provider_gitlab()}
    </div>
    {#each providerItems as item}
      <button
        class="nav-item"
        class:active={activeNav === item.id}
        onclick={() => handleNav(item.id)}
      >
        <span class="nav-icon">{item.icon}</span>
        <span class="nav-label">{item.label}</span>
      </button>
    {/each}
  </nav>

  <div class="spacer"></div>

  <div class="nav-section">
    <button
      class="nav-item"
      class:active={activeNav === "settings"}
      onclick={() => handleNav("settings")}
    >
      <span class="nav-icon">{"\uF013"}</span>
      <span class="nav-label">{m.sidebar_settings()}</span>
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: clamp(180px, 15vw, 240px);
    min-width: 0;
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    user-select: none;
  }

  .nav-section {
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .nav-section:last-child {
    border-bottom: none;
  }

  .section-label {
    padding: 4px 16px 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 16px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s;
  }

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .nav-item.active {
    background: rgba(88, 166, 255, 0.1);
    color: var(--accent-blue);
  }

  .nav-item.active .nav-icon {
    color: var(--accent-blue);
  }

  .nav-icon {
    width: 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 14px;
    font-family: var(--font-icons);
  }

  .nav-label {
    flex: 1;
  }

  .nav-badge {
    font-size: 10px;
    background: var(--accent-blue);
    color: #ffffff;
    border-radius: 8px;
    padding: 0 5px;
    min-width: 16px;
    text-align: center;
    line-height: 16px;
  }

  .placeholder-text {
    padding: 6px 16px;
    font-size: 12px;
    color: var(--text-secondary);
    font-style: italic;
  }

  .placeholder-text.wip {
    font-style: italic;
    opacity: 0.5;
  }

  .provider-status-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #666666;
    margin-right: 4px;
    vertical-align: middle;
  }

  .provider-status-dot.connected {
    background: #3fb950;
  }

  .spacer {
    flex: 1;
  }
</style>
