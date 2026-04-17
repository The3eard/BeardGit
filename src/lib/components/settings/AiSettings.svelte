<script lang="ts">
  import { onMount } from "svelte";
  import { aiProviders, preferredAiProvider, detectAiProviders, setPreferredProvider, loadPreferredProvider } from "$lib/stores/ai";
  import type { AiBackgroundSettings, AiProviderKind } from "$lib/types";
  import { aiBackgroundGetSettings, aiBackgroundSetSettings } from "$lib/api/tauri";
  import * as m from "$lib/paraglide/messages";

  /** All known provider kinds (for display even when not installed). */
  const ALL_KINDS: { kind: AiProviderKind; label: () => string; icon: string; color: string }[] = [
    { kind: "claude_code", label: () => m.ai_settings_provider_claude(), icon: "\uF135", color: "#d97757" },
    { kind: "codex", label: () => m.ai_settings_provider_codex(), icon: "\uF121", color: "#ffffff" },
    { kind: "open_code", label: () => m.ai_settings_provider_opencode(), icon: "\uF489", color: "#8b8b8b" },
  ];

  let refreshing = $state(false);
  let bgSettings = $state<AiBackgroundSettings>({
    worktree_root: null,
    concurrency_cap: 3,
    auto_accept_permissions: false,
  });
  let bgSaving = $state(false);
  let bgError = $state<string | null>(null);

  onMount(async () => {
    await detectAiProviders();
    await loadPreferredProvider();
    try {
      bgSettings = await aiBackgroundGetSettings();
    } catch (e) {
      bgError = String(e);
    }
  });

  async function saveBgSettings() {
    bgSaving = true;
    bgError = null;
    try {
      await aiBackgroundSetSettings({
        worktree_root: bgSettings.worktree_root && bgSettings.worktree_root.trim().length > 0
          ? bgSettings.worktree_root.trim()
          : null,
        concurrency_cap: Math.max(1, Math.floor(bgSettings.concurrency_cap)),
        auto_accept_permissions: bgSettings.auto_accept_permissions,
      });
    } catch (e) {
      bgError = String(e);
    } finally {
      bgSaving = false;
    }
  }

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

<div class="ai-card">
  <h2 class="card-title">{m.settings_ai_background_heading()}</h2>

  <div class="bg-field">
    <label class="field-label" for="bg-root">{m.settings_ai_worktree_root_label()}</label>
    <input
      id="bg-root"
      class="field-input"
      type="text"
      placeholder=".beardgit/ai-worktrees"
      value={bgSettings.worktree_root ?? ""}
      oninput={(e) => { bgSettings.worktree_root = e.currentTarget.value; }}
      onblur={saveBgSettings}
    />
    <p class="field-hint">{m.settings_ai_worktree_root_hint()}</p>
  </div>

  <div class="bg-field">
    <label class="field-label" for="bg-cap">{m.settings_ai_concurrency_label()}</label>
    <input
      id="bg-cap"
      class="field-input short"
      type="number"
      min="1"
      max="32"
      bind:value={bgSettings.concurrency_cap}
      onblur={saveBgSettings}
    />
    <p class="field-hint">{m.settings_ai_concurrency_hint()}</p>
  </div>

  <div class="bg-field">
    <label class="checkbox-label">
      <input
        type="checkbox"
        bind:checked={bgSettings.auto_accept_permissions}
        onchange={saveBgSettings}
      />
      <span>{m.settings_ai_auto_accept_label()}</span>
    </label>
    <p class="field-hint">{m.settings_ai_auto_accept_hint()}</p>
  </div>

  {#if bgError}
    <p class="error-text">{bgError}</p>
  {:else if bgSaving}
    <p class="saving-text">…</p>
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

  .bg-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 16px;
  }

  .field-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .field-input {
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
    outline: none;
    box-sizing: border-box;
  }

  .field-input.short {
    max-width: 100px;
  }

  .field-input:focus {
    border-color: var(--accent-blue);
  }

  .field-hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .error-text {
    color: #f85149;
    font-size: 12px;
  }

  .saving-text {
    color: var(--text-secondary);
    font-size: 12px;
  }
</style>
