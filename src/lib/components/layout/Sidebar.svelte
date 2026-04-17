<script lang="ts">
  import { repoInfo } from "../../stores/repo";
  import { fileStatuses } from "../../stores/changes";
  import { hasActiveProvider, activeProvider } from "../../stores/provider";
  import * as m from "$lib/paraglide/messages";

  let {
    onNavigate,
    activeView = "graph",
    collapsed = false,
    onToggleCollapse,
  }: {
    onNavigate?: (view: string) => void;
    activeView?: string;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
  } = $props();

  type NavItem = { label: string; icon: string; id: string };

  const navItems: NavItem[] = [
    { label: m.sidebar_graph(), icon: "\uE728", id: "graph" },
    { label: m.sidebar_changes(), icon: "\uF440", id: "changes" },
    { label: m.sidebar_branches(), icon: "\uE725", id: "branches" },
    { label: m.sidebar_tags(), icon: "\uF02B", id: "tags" },
    { label: m.sidebar_stashes(), icon: "\uF187", id: "stashes" },
    { label: m.sidebar_worktrees(), icon: "\uE728", id: "worktrees" },
    { label: m.sidebar_reflog(), icon: "\uF1DA", id: "reflog" },
    { label: m.sidebar_bisect(), icon: "\uF002", id: "bisect" },
    { label: m.sidebar_submodules(), icon: "\uF1E6", id: "submodules" },
    { label: m.sidebar_ai_config(), icon: "\uF085", id: "ai-config" },
    { label: m.sidebar_ai_sessions(), icon: "\uF489", id: "ai-sessions" },
  ];

  // MR/PR label depends on the active forge — GitHub says "Pull requests",
  // everyone else (GitLab, and future forges that inherit the term) says
  // "Merge requests". Keeping the id stable so activeView routing doesn't
  // care which terminology we render.
  let providerItems = $derived<NavItem[]>([
    { label: m.sidebar_pipelines(), icon: "\uF144", id: "pipelines" },
    { label: m.sidebar_issues(), icon: "\uF188", id: "issues" },
    {
      label:
        $activeProvider?.kind === "github"
          ? m.sidebar_pull_requests()
          : m.sidebar_merge_requests(),
      icon: "\uF407",
      id: "merge-requests",
    },
    { label: m.sidebar_releases(), icon: "\uF135", id: "releases" },
  ]);

  function handleNav(id: string) {
    onNavigate?.(id);
  }

  let changeCount = $derived($fileStatuses.length);
</script>

<aside class="sidebar" class:collapsed>
  <nav class="nav-section">
    {#if !collapsed}
      <div class="section-label">{m.sidebar_navigation()}</div>
    {/if}
    {#each navItems as item}
      <button
        class="nav-item"
        class:active={activeView === item.id}
        onclick={() => handleNav(item.id)}
        title={collapsed ? item.label : undefined}
        data-testid="nav-{item.id}"
      >
        <span class="nav-icon">{item.icon}</span>
        {#if !collapsed}
          <span class="nav-label">{item.label}</span>
          {#if item.id === "changes" && changeCount > 0}
            <span class="nav-badge">{changeCount}</span>
          {/if}
        {/if}
      </button>
    {/each}
  </nav>

  <nav class="nav-section">
    {#if !collapsed}
      <div class="section-label">
        <span class="provider-status-dot" class:connected={$hasActiveProvider}></span>
        {$activeProvider?.kind === 'github' ? m.provider_github() : m.provider_gitlab()}
      </div>
    {/if}
    {#each providerItems as item}
      <button
        class="nav-item"
        class:active={activeView === item.id}
        onclick={() => handleNav(item.id)}
        title={collapsed ? item.label : undefined}
        data-testid="nav-{item.id}"
      >
        <span class="nav-icon">{item.icon}</span>
        {#if !collapsed}
          <span class="nav-label">{item.label}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="spacer"></div>

  <div class="nav-section bottom-section">
    <button
      class="nav-item"
      class:active={activeView === "settings"}
      onclick={() => handleNav("settings")}
      title={collapsed ? m.sidebar_settings() : undefined}
      data-testid="nav-settings"
    >
      <span class="nav-icon">{"\uF013"}</span>
      {#if !collapsed}
        <span class="nav-label">{m.sidebar_settings()}</span>
      {/if}
    </button>
    <button
      class="nav-item collapse-btn"
      onclick={onToggleCollapse}
      title={collapsed ? m.sidebar_expand() : m.sidebar_collapse()}
    >
      <span class="nav-icon">{collapsed ? "\uF054" : "\uF053"}</span>
      {#if !collapsed}
        <span class="nav-label">{m.sidebar_collapse()}</span>
      {/if}
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
    transition: width 150ms ease;
  }

  .sidebar.collapsed {
    width: 44px;
  }

  .nav-section {
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .nav-section:last-child {
    border-bottom: none;
  }

  .bottom-section {
    border-top: 1px solid var(--border);
    border-bottom: none;
  }

  .section-label {
    padding: 4px 16px 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    overflow: hidden;
    white-space: nowrap;
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
    overflow: hidden;
    white-space: nowrap;
  }

  .sidebar.collapsed .nav-item {
    justify-content: center;
    padding: 6px 0;
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
    flex-shrink: 0;
  }

  .nav-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .collapse-btn .nav-icon {
    font-size: 12px;
  }
</style>
