<script lang="ts">
  import { onMount } from "svelte";
  import type { CliAuthStatus } from "$lib/types";
  import { cliCheckAuthStatus, cliGetAuthCommand, cliGetLogoutCommand, terminalWrite } from "$lib/api/tauri";
  import { openStandaloneTerminal } from "$lib/stores/tabs";
  import * as m from "$lib/paraglide/messages";

  /** Tool display config (static — both tools always shown). */
  const TOOLS: { tool: string; label: () => string; icon: string; color: string }[] = [
    { tool: "gh", label: () => m.cli_auth_gh(), icon: "\uF408", color: "#ffffff" },
    { tool: "glab", label: () => m.cli_auth_glab(), icon: "\uF296", color: "#fc6d26" },
  ];

  let statuses = $state<CliAuthStatus[]>([]);
  let refreshing = $state(false);
  let launching = $state<string | null>(null);

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    refreshing = true;
    try {
      statuses = await cliCheckAuthStatus();
    } catch {
      // silently fail — cards will show empty state
    } finally {
      refreshing = false;
    }
  }

  function getStatus(tool: string): CliAuthStatus | undefined {
    return statuses.find((s) => s.tool === tool);
  }

  async function handleAuth(tool: string) {
    launching = tool;
    try {
      const cmd = await cliGetAuthCommand(tool);
      const { homeDir } = await import("@tauri-apps/api/path");
      const cwd = await homeDir();
      const sessionId = await openStandaloneTerminal(cwd, `${tool} auth login`);
      // Write the auth command followed by Enter into the terminal
      const encoded = btoa(cmd + "\n");
      await terminalWrite(sessionId, encoded);
    } catch {
      // terminal open failed — ignore
    } finally {
      launching = null;
    }
  }

  async function handleLogout(tool: string) {
    launching = tool;
    try {
      const cmd = await cliGetLogoutCommand(tool);
      const { homeDir } = await import("@tauri-apps/api/path");
      const cwd = await homeDir();
      const sessionId = await openStandaloneTerminal(cwd, `${tool} auth logout`);
      const encoded = btoa(cmd + "\n");
      await terminalWrite(sessionId, encoded);
    } catch {
      // terminal open failed — ignore
    } finally {
      launching = null;
    }
  }
</script>

<div class="cli-auth-card">
  <div class="card-header">
    <h2 class="card-title">{m.cli_auth_title()}</h2>
    <button class="refresh-btn" onclick={refresh} disabled={refreshing} title={m.cli_auth_refresh()}>
      <span class="nf" class:spinning={refreshing}>{"\uF021"}</span>
    </button>
  </div>

  <div class="tool-list">
    {#each TOOLS as { tool, label, icon, color }}
      {@const status = getStatus(tool)}
      {@const installed = status?.installed ?? false}
      {@const authenticated = status?.authenticated ?? false}
      {@const username = status?.username ?? null}
      <div class="tool-row" class:authenticated>
        <span class="tool-icon nf" style="color: {installed ? color : 'var(--text-secondary)'}">{icon}</span>
        <div class="tool-info">
          <span class="tool-name">{label()}</span>
          {#if !installed}
            <span class="tool-status not-installed">{m.cli_auth_not_installed()}</span>
          {:else if authenticated && username}
            <span class="tool-status authed">{m.cli_auth_username({ username })}</span>
          {:else if authenticated}
            <span class="tool-status authed">{m.cli_auth_authenticated()}</span>
          {:else}
            <span class="tool-status not-authed">{m.cli_auth_not_authenticated()}</span>
          {/if}
        </div>
        <div class="tool-actions">
          {#if installed && authenticated}
            <span class="auth-badge">{m.cli_auth_authenticated()}</span>
            <button
              class="action-btn logout"
              onclick={() => handleLogout(tool)}
              disabled={launching === tool}
            >
              {m.cli_auth_logout()}
            </button>
          {:else if installed}
            <button
              class="action-btn auth"
              onclick={() => handleAuth(tool)}
              disabled={launching === tool}
            >
              {m.cli_auth_authenticate()}
            </button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .cli-auth-card { max-width: 480px; margin: 48px auto; padding: 32px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; }
  .card-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px; }
  .card-title { font-size: 18px; font-weight: 600; color: var(--text-primary); margin: 0; }

  .refresh-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    width: 28px;
    height: 28px;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, color 0.15s;
  }
  .refresh-btn:hover { background: var(--overlay-hover); color: var(--text-primary); }
  .refresh-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .nf { font-family: var(--font-icons); font-size: 13px; line-height: 1; }

  .spinning { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .tool-list { display: flex; flex-direction: column; gap: 4px; }

  .tool-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-primary);
    transition: border-color 0.15s;
  }

  .tool-row.authenticated {
    border-color: var(--accent-green);
    background: var(--overlay-accent-green);
  }

  .tool-icon { font-size: 18px; flex-shrink: 0; width: 24px; text-align: center; }

  .tool-info { display: flex; flex-direction: column; gap: 1px; flex: 1; min-width: 0; }
  .tool-name { font-size: 13px; font-weight: 500; color: var(--text-primary); }
  .tool-status { font-size: 11px; }
  .tool-status.authed { color: var(--accent-green); }
  .tool-status.not-authed { color: var(--text-secondary); }
  .tool-status.not-installed { color: var(--text-secondary); font-style: italic; }

  .tool-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }

  .auth-badge {
    font-size: 10px;
    font-weight: 600;
    color: var(--accent-green);
    background: var(--overlay-accent-green);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .action-btn {
    font-size: 11px;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  .action-btn:hover { background: var(--overlay-hover); }
  .action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .action-btn.auth {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    color: #fff;
  }
  .action-btn.auth:hover { opacity: 0.9; }

  .action-btn.logout {
    color: var(--text-secondary);
  }
</style>
