<!--
  AiSettings.svelte — AI provider picker + background-run config.

  Phase 4.5 refactor only: same behaviour as the pre-MT-5 version
  (detect providers, pick a preferred one, tune the background
  worktree / concurrency / auto-accept defaults) — now rendered on
  top of the shared `Card` / `SettingSection` / `FormRow` / `Field`
  / `Button` primitives so the inline card CSS disappears.

  Provider rows stay as a bespoke grid because the
  icon-label-status-badge layout is unique to this category and
  doesn't map cleanly to `FormRow`.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  export const settingsIndex: SettingDescriptor[] = [
    {
      id: "ai.provider",
      label: "Preferred AI provider",
      description:
        "Pick which installed AI tool (Claude Code / Codex / OpenCode) BeardGit uses by default.",
      category: "ai",
      anchor: "provider",
    },
    {
      id: "ai.worktree-root",
      label: "AI worktree root",
      description:
        "Directory where AI background worktrees are created. Relative paths resolve to each repo.",
      category: "ai",
      anchor: "worktree-root",
    },
    {
      id: "ai.concurrency",
      label: "Concurrent background runs",
      description:
        "Maximum number of AI background runs that may execute at once. Extra runs are queued.",
      category: "ai",
      anchor: "concurrency",
    },
    {
      id: "ai.auto-accept",
      label: "Auto-accept AI permission prompts",
      description:
        "Allow AI agents to edit files in the worktree without confirmation — use with care.",
      category: "ai",
      anchor: "auto-accept",
    },
  ];
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import {
    aiProviders,
    aiProvidersDetecting,
    preferredAiProvider,
    detectAiProviders,
    setPreferredProvider,
    loadPreferredProvider,
  } from "$lib/stores/ai";
  import type { AiBackgroundSettings, AiProviderKind } from "$lib/types";
  import {
    aiBackgroundGetSettings,
    aiBackgroundSetSettings,
  } from "$lib/api/tauri";
  import * as m from "$lib/paraglide/messages";
  import {
    Card,
    SettingSection,
    FormRow,
    Field,
    Switch,
  } from "$lib/components/ui";
  // ProviderIcon is shared with AiSessions + Spec 4's generic-icon fix.
  import ProviderIcon from "$lib/components/ai-sessions/ProviderIcon.svelte";

  const ALL_KINDS: { kind: AiProviderKind; label: () => string }[] = [
    { kind: "claude_code", label: () => m.ai_settings_provider_claude() },
    { kind: "codex", label: () => m.ai_settings_provider_codex() },
    { kind: "open_code", label: () => m.ai_settings_provider_opencode() },
  ];

  let bgSettings = $state<AiBackgroundSettings>({
    worktree_root: null,
    concurrency_cap: 3,
    auto_accept_permissions: false,
  });
  let bgSaving = $state(false);
  let bgError = $state<string | null>(null);

  // Fire-and-forget on mount: the Settings shell paints immediately while
  // the three async operations populate their respective stores in the
  // background. `detectAiProviders` runs PATH probes + `--version` calls
  // that can take ~1 s on a cold cache — we render the provider list as
  // "detecting..." during that window, not "Not found".
  onMount(() => {
    void detectAiProviders();
    void loadPreferredProvider();
    void (async () => {
      try {
        bgSettings = await aiBackgroundGetSettings();
      } catch (e) {
        bgError = String(e);
      }
    })();
  });

  async function saveBgSettings() {
    bgSaving = true;
    bgError = null;
    try {
      await aiBackgroundSetSettings({
        worktree_root:
          bgSettings.worktree_root && bgSettings.worktree_root.trim().length > 0
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

<Card
  title={m.settings_ai_providers_section_title()}
  description={m.settings_ai_providers_section_description()}
>
  <SettingSection title={m.ai_settings_title()}>
    <div class="provider-list" data-setting-anchor="provider">
      {#each ALL_KINDS as { kind, label } (kind)}
        {@const detected = isDetected(kind)}
        {@const preferred = isPreferred(kind)}
        {@const version = getVersion(kind)}
        {@const detecting = $aiProvidersDetecting && !detected}
        <button
          type="button"
          class="provider-row"
          class:detected
          class:preferred
          disabled={!detected}
          onclick={() => handleSelect(kind)}
        >
          <ProviderIcon provider={kind} size={20} />
          <div class="provider-info">
            <span class="provider-name">{label()}</span>
            {#if detected && version}
              <span class="provider-version"
                >{m.ai_settings_version({ version })}</span
              >
            {:else if detecting}
              <span class="provider-status detecting">
                <span class="detecting-spinner" aria-hidden="true"></span>
                {m.ai_settings_detecting()}
              </span>
            {:else if !detected}
              <span class="provider-status not-found"
                >{m.ai_settings_not_found()}</span
              >
            {/if}
          </div>
          {#if preferred && detected}
            <span class="preferred-badge"
              >{m.ai_settings_default_provider()}</span
            >
          {:else if detected}
            <span class="detected-badge">{m.ai_settings_detected()}</span>
          {/if}
        </button>
      {/each}
    </div>

    {#if !$aiProvidersDetecting && $aiProviders.length === 0}
      <div class="empty-state">{m.ai_settings_no_providers()}</div>
    {/if}
  </SettingSection>
</Card>

<Card
  title={m.settings_ai_background_section_title()}
  description={m.settings_ai_background_section_description()}
>
  <SettingSection title={m.settings_ai_background_heading()}>
    <div data-setting-anchor="worktree-root">
      <Field
        label={m.settings_ai_worktree_root_label()}
        description={m.settings_ai_worktree_root_hint()}
        for="bg-root"
      >
        <input
          id="bg-root"
          class="field-input"
          type="text"
          placeholder=".beardgit/ai-worktrees"
          value={bgSettings.worktree_root ?? ""}
          oninput={(e) => {
            bgSettings.worktree_root = e.currentTarget.value;
          }}
          onblur={saveBgSettings}
        />
      </Field>
    </div>

    <div data-setting-anchor="concurrency">
      <FormRow
        label={m.settings_ai_concurrency_label()}
        for="bg-cap"
        helperText={m.settings_ai_concurrency_hint()}
      >
        <input
          id="bg-cap"
          class="field-input short"
          type="number"
          min="1"
          max="32"
          bind:value={bgSettings.concurrency_cap}
          onblur={saveBgSettings}
        />
      </FormRow>
    </div>

    <div data-setting-anchor="auto-accept">
      <FormRow
        label={m.settings_ai_auto_accept_label()}
        for="bg-auto-accept"
        helperText={m.settings_ai_auto_accept_hint()}
      >
        <Switch
          id="bg-auto-accept"
          checked={bgSettings.auto_accept_permissions}
          onchange={(e) => {
            bgSettings.auto_accept_permissions = (e.target as HTMLInputElement).checked;
            saveBgSettings();
          }}
        />
      </FormRow>
    </div>

    {#if bgError}
      <p class="error-text">{bgError}</p>
    {:else if bgSaving}
      <p class="saving-text">…</p>
    {/if}
  </SettingSection>
</Card>

<style>
  .provider-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .provider-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-primary);
    cursor: pointer;
    transition:
      background 0.15s,
      border-color 0.15s;
    text-align: left;
    width: 100%;
    font-family: inherit;
  }

  .provider-row:hover:not(:disabled) {
    background: var(--overlay-hover);
  }
  .provider-row:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .provider-row.preferred {
    border-color: var(--accent-primary);
    background: var(--overlay-accent-blue);
  }

  .provider-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    flex: 1;
    min-width: 0;
  }
  .provider-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }
  .provider-version {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .provider-status.not-found {
    font-size: 11px;
    color: var(--text-secondary);
    font-style: italic;
  }
  .provider-status.detecting {
    font-size: 11px;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .detecting-spinner {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 1.5px solid var(--text-secondary);
    border-top-color: transparent;
    border-radius: 50%;
    animation: ai-settings-spin 0.6s linear infinite;
    opacity: 0.7;
  }
  @keyframes ai-settings-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .preferred-badge {
    font-size: 10px;
    font-weight: 600;
    color: var(--accent-primary);
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
    color: var(--text-secondary);
    font-size: 12px;
    font-style: italic;
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
    width: 100%;
  }

  .field-input.short {
    max-width: 100px;
    width: 100px;
  }

  .field-input:focus {
    border-color: var(--accent-primary);
  }

  .error-text {
    color: var(--accent-red);
    font-size: 12px;
  }

  .saving-text {
    color: var(--text-secondary);
    font-size: 12px;
  }
</style>
