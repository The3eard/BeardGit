<script lang="ts">
  import { onMount } from "svelte";
  import { aiProviders, preferredAiProvider, detectAiProviders, setPreferredProvider, loadPreferredProvider } from "$lib/stores/ai";
  import type { AiProviderKind } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

  /** All known provider kinds (for display even when not installed). */
  const ALL_KINDS: { kind: AiProviderKind; label: () => string; icon: string; color: string }[] = [
    { kind: "claude_code", label: () => m.ai_settings_provider_claude(), icon: "\uF135", color: "#d97757" },
    { kind: "codex", label: () => m.ai_settings_provider_codex(), icon: "\uF121", color: "#10a37f" },
    { kind: "open_code", label: () => m.ai_settings_provider_opencode(), icon: "\uF489", color: "#8b8b8b" },
  ];

  let refreshing = $state(false);

  onMount(async () => {
    await detectAiProviders();
    await loadPreferredProvider();
  });

  async function handleRefresh() {
    refreshing = true;
    try {
      await detectAiProviders();
    } finally {
      refreshing = false;
    }
  }

  async function handleSelect(kind: AiProviderKind) {
    const available = $aiProviders.some((p) => p.kind === kind);
    if (!available) return;
    await setPreferredProvider($preferredAiProvider === kind ? null : kind);
  }

  function isDetected(kind: AiProviderKind): boolean {
    return $aiProviders.some((p) => p.kind === kind);
  }

  function getVersion(kind: AiProviderKind): string | null {
    return $aiProviders.find((p) => p.kind === kind)?.version ?? null;
  }

  function isPreferred(kind: AiProviderKind): boolean {
    if ($preferredAiProvider) return $preferredAiProvider === kind;
    return $aiProviders.length > 0 && $aiProviders[0].kind === kind;
  }
</script>

<div class="ai-card">
  <div class="card-header">
    <h2 class="card-title">{m.ai_settings_title()}</h2>
    <button class="refresh-btn" onclick={handleRefresh} disabled={refreshing} title="Refresh">
      <span class="nf" class:spinning={refreshing}>{"\uF021"}</span>
    </button>
  </div>

  <div class="provider-list">
    {#each ALL_KINDS as { kind, label, icon, color }}
      {@const detected = isDetected(kind)}
      {@const preferred = isPreferred(kind)}
      {@const version = getVersion(kind)}
      <button
        class="provider-row"
        class:detected
        class:preferred
        disabled={!detected}
        onclick={() => handleSelect(kind)}
      >
        <span class="provider-icon nf" style="color: {detected ? color : 'var(--text-secondary)'}">{icon}</span>
        <div class="provider-info">
          <span class="provider-name">{label()}</span>
          {#if detected && version}
            <span class="provider-version">{m.ai_settings_version({ version })}</span>
          {:else if !detected}
            <span class="provider-status not-found">{m.ai_settings_not_found()}</span>
          {/if}
        </div>
        {#if preferred && detected}
          <span class="preferred-badge">{m.ai_settings_default_provider()}</span>
        {:else if detected}
          <span class="detected-badge">{m.ai_settings_detected()}</span>
        {/if}
      </button>
    {/each}
  </div>

  {#if $aiProviders.length === 0}
    <div class="empty-state">{m.ai_settings_no_providers()}</div>
  {/if}
</div>

<style>
  .ai-card { max-width: 480px; margin: 48px auto; padding: 32px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; }
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

  .provider-list { display: flex; flex-direction: column; gap: 4px; }

  .provider-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-primary);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    text-align: left;
    width: 100%;
  }

  .provider-row:hover:not(:disabled) { background: var(--overlay-hover); }
  .provider-row:disabled { opacity: 0.45; cursor: not-allowed; }

  .provider-row.preferred {
    border-color: var(--accent-blue);
    background: var(--overlay-accent-blue);
  }

  .provider-icon { font-size: 18px; flex-shrink: 0; width: 24px; text-align: center; }

  .provider-info { display: flex; flex-direction: column; gap: 1px; flex: 1; min-width: 0; }
  .provider-name { font-size: 13px; font-weight: 500; color: var(--text-primary); }
  .provider-version { font-size: 11px; color: var(--text-secondary); }
  .provider-status.not-found { font-size: 11px; color: var(--text-secondary); font-style: italic; }

  .preferred-badge {
    font-size: 10px;
    font-weight: 600;
    color: var(--accent-blue);
    background: var(--overlay-accent-blue);
    padding: 2px 8px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .detected-badge {
    font-size: 10px;
    color: var(--accent-green);
    background: var(--overlay-accent-green);
    padding: 2px 8px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .empty-state {
    text-align: center;
    color: var(--text-secondary);
    font-size: 12px;
    font-style: italic;
    margin-top: 16px;
  }
</style>
