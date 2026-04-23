<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { aiProviders, defaultAiProvider } from "$lib/stores/ai";
  import { branches } from "$lib/stores/repo";
  import { configFiles, loadConfigFiles } from "$lib/stores/aiConfig";
  import { aiBackgroundRuns, startAiBackgroundRun } from "$lib/stores/aiBackground";
  import type { AiProviderKind, StartBackgroundRunRequest } from "$lib/types";

  interface Props {
    onClose: () => void;
    /** Optional pre-selected base branch (defaults to current HEAD). */
    initialBaseBranch?: string;
  }

  let { onClose, initialBaseBranch }: Props = $props();

  type Tab = "text" | "saved" | "skill";

  let provider = $state<AiProviderKind | null>(null);
  let baseBranch = $state<string>("");
  let slug = $state<string>("");
  let slugEdited = $state(false);
  let activeTab = $state<Tab>("text");
  let freeText = $state("");
  let savedPromptPath = $state<string | null>(null);
  let skillName = $state<string | null>(null);
  let submitting = $state(false);
  let error = $state<string | null>(null);

  // Derived provider list — only installed ones show up.
  let availableProviders = $derived($aiProviders);
  let branchList = $derived($branches.map((b) => b.name));

  // Saved prompts (kind === "prompt") and skills (kind === "skill").
  let savedPrompts = $derived(
    $configFiles.filter((f) => f.kind === "instructions" && f.path.includes("/prompts/")),
  );
  let skills = $derived($configFiles.filter((f) => f.kind === "skill"));

  // Auto-slug from the first tokens of whichever prompt source is active.
  let derivedSlug = $derived.by(() => {
    const source =
      activeTab === "skill" && skillName
        ? skillName
        : activeTab === "saved" && savedPromptPath
          ? savedPromptPath.split("/").pop()?.replace(/\.md$/, "") ?? ""
          : freeText;
    return source
      .split(/\s+/)
      .slice(0, 4)
      .map((tok) =>
        tok
          .toLowerCase()
          .replace(/[^a-z0-9-_]/g, ""),
      )
      .filter(Boolean)
      .join("-") || "ai-run";
  });

  let effectiveSlug = $derived(slugEdited ? slug : derivedSlug);

  // Validation — at least one prompt source must be set, and the slug must
  // not collide with an active/queued run.
  let slugCollides = $derived.by(() => {
    const target = effectiveSlug;
    return Array.from($aiBackgroundRuns.values()).some((s) => {
      if (!s.worktree_path) return false;
      return s.worktree_path.endsWith("/" + target);
    });
  });

  let hasPromptContent = $derived.by(() => {
    if (activeTab === "text") return freeText.trim().length > 0;
    if (activeTab === "saved")
      return freeText.trim().length > 0 || !!savedPromptPath;
    if (activeTab === "skill") return !!skillName;
    return false;
  });

  let validationError = $derived.by(() => {
    if (!provider) return m.ai_background_validation_no_provider();
    if (!hasPromptContent) return m.ai_background_validation_no_prompt();
    if (slugCollides) return m.ai_background_validation_slug_exists();
    return null;
  });

  async function handleSubmit() {
    if (validationError || !provider || submitting) return;
    submitting = true;
    error = null;
    try {
      const request: StartBackgroundRunRequest = {
        provider,
        base_branch: baseBranch,
        prompt: freeText,
        skill: activeTab === "skill" ? skillName : null,
        saved_prompt_path: activeTab === "saved" ? savedPromptPath : null,
        resume_session_id: null,
        worktree_slug_override: slugEdited ? slug : null,
      };
      await startAiBackgroundRun(request);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    } else if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      handleSubmit();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    // Preload config files so the saved-prompt and skill pickers are
    // populated by the time the user flips to their tabs.
    loadConfigFiles().catch(() => {});
    provider = $defaultAiProvider;
    baseBranch = initialBaseBranch ?? ($branches.find((b) => b.is_head)?.name ?? "");
    return () => window.removeEventListener("keydown", handleKeydown);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="backdrop" onclick={onClose} onkeydown={(e) => { if (e.key === "Escape") onClose(); }} role="button" tabindex="-1"></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={m.ai_background_dialog_title()}>
  <h3 class="dialog-title">{m.ai_background_dialog_title()}</h3>

  <div class="row">
    <div class="form-field">
      <label class="field-label" for="aibg-provider">{m.ai_background_provider_label()}</label>
      <select
        id="aibg-provider"
        class="field-input"
        bind:value={provider}
        disabled={submitting}
      >
        {#each availableProviders as p (p.kind)}
          <option value={p.kind}>{p.kind.replace("_", " ")}</option>
        {/each}
      </select>
    </div>

    <div class="form-field">
      <label class="field-label" for="aibg-base">{m.ai_background_base_branch_label()}</label>
      <input
        id="aibg-base"
        class="field-input"
        list="aibg-branches"
        placeholder={m.ai_background_base_branch_placeholder()}
        bind:value={baseBranch}
        disabled={submitting}
      />
      <datalist id="aibg-branches">
        {#each branchList as b (b)}
          <option value={b}></option>
        {/each}
      </datalist>
    </div>
  </div>

  <div class="form-field">
    <label class="field-label" for="aibg-slug">{m.ai_background_worktree_slug_label()}</label>
    <input
      id="aibg-slug"
      class="field-input"
      value={effectiveSlug}
      oninput={(e) => {
        slug = e.currentTarget.value;
        slugEdited = true;
      }}
      disabled={submitting}
    />
    <p class="field-hint">{m.ai_background_worktree_slug_hint()}</p>
  </div>

  <div class="tabs">
    <button
      class="tab"
      class:active={activeTab === "text"}
      onclick={() => (activeTab = "text")}
      type="button">{m.ai_background_prompt_tab_text()}</button
    >
    <button
      class="tab"
      class:active={activeTab === "saved"}
      onclick={() => (activeTab = "saved")}
      type="button">{m.ai_background_prompt_tab_saved()}</button
    >
    <button
      class="tab"
      class:active={activeTab === "skill"}
      onclick={() => (activeTab = "skill")}
      type="button">{m.ai_background_prompt_tab_skill()}</button
    >
  </div>

  <div class="tab-panel">
    {#if activeTab === "text"}
      <label class="field-label" for="aibg-prompt">{m.ai_background_prompt_label()}</label>
      <textarea
        id="aibg-prompt"
        class="prompt-area"
        placeholder={m.ai_background_prompt_placeholder()}
        bind:value={freeText}
        disabled={submitting}
      ></textarea>
    {:else if activeTab === "saved"}
      {#if savedPrompts.length === 0}
        <p class="empty">{m.ai_background_saved_prompt_empty()}</p>
      {:else}
        <label class="field-label" for="aibg-saved">{m.ai_background_saved_prompt_label()}</label>
        <select
          id="aibg-saved"
          class="field-input"
          bind:value={savedPromptPath}
          disabled={submitting}
        >
          <option value={null}>—</option>
          {#each savedPrompts as prompt (prompt.path)}
            <option value={prompt.path}>
              {prompt.path.split("/").slice(-2).join("/")}
            </option>
          {/each}
        </select>
      {/if}
      <label class="field-label" for="aibg-free-on-saved">{m.ai_background_prompt_label()}</label>
      <textarea
        id="aibg-free-on-saved"
        class="prompt-area short"
        placeholder={m.ai_background_prompt_placeholder()}
        bind:value={freeText}
        disabled={submitting}
      ></textarea>
      <p class="field-hint">{m.ai_background_prompt_combine_hint()}</p>
    {:else if activeTab === "skill"}
      {#if skills.length === 0}
        <p class="empty">{m.ai_background_skill_empty()}</p>
      {:else}
        <label class="field-label" for="aibg-skill">{m.ai_background_skill_label()}</label>
        <select
          id="aibg-skill"
          class="field-input"
          bind:value={skillName}
          disabled={submitting}
        >
          <option value={null}>—</option>
          {#each skills as skill (skill.path)}
            <option value={skillName_from_path(skill.path)}>
              {skillName_from_path(skill.path)}
            </option>
          {/each}
        </select>
      {/if}
    {/if}
  </div>

  {#if error}
    <p class="error-text">{error}</p>
  {:else if validationError}
    <p class="validation-text">{validationError}</p>
  {/if}

  <div class="dialog-actions">
    <span class="hint">{m.ai_background_hint_cmd_enter()}</span>
    <button class="btn btn-cancel" onclick={onClose} disabled={submitting}>
      {m.ai_background_cancel()}
    </button>
    <button
      class="btn btn-confirm"
      onclick={handleSubmit}
      disabled={submitting || !!validationError}
    >
      {submitting ? "..." : m.ai_background_submit()}
    </button>
  </div>
</div>

<script module lang="ts">
  /** Extract a skill name from a SKILL.md path. Exposed for reuse / tests. */
  export function skillName_from_path(path: string): string {
    // .../skills/<name>/SKILL.md
    const parts = path.split("/");
    const idx = parts.indexOf("skills");
    if (idx >= 0 && idx + 1 < parts.length) {
      return parts[idx + 1];
    }
    return path;
  }
</script>

<style>
  .dialog {
    min-width: 520px;
    max-width: 640px;
    max-height: 84vh;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .dialog-title {
    margin: 0 0 4px;
    font-size: 14px;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .field-hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .field-input {
    width: 100%;
    height: 30px;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
    line-height: 1.2;
    outline: none;
    box-sizing: border-box;
    /* Normalise native chrome so <select> and <input> share the same
       height/alignment — native selects on macOS add extra padding for
       the caret which pushed the provider column out of line with the
       base-branch input. */
    appearance: none;
    -webkit-appearance: none;
  }

  /* Restore a small caret so we don't lose the visual affordance
     after flattening the native control. */
  select.field-input {
    padding-right: 24px;
    background-image: linear-gradient(45deg, transparent 50%, var(--text-secondary) 50%),
                      linear-gradient(135deg, var(--text-secondary) 50%, transparent 50%);
    background-position:
      calc(100% - 14px) 13px,
      calc(100% - 9px) 13px;
    background-size: 5px 5px, 5px 5px;
    background-repeat: no-repeat;
  }

  .field-input:focus {
    border-color: var(--accent-blue);
  }

  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
    margin-top: 4px;
  }

  .tab {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 6px 12px;
    font-size: 11px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    font-weight: 600;
  }

  .tab.active {
    color: var(--accent-blue);
    border-bottom-color: var(--accent-blue);
  }

  .tab-panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 140px;
  }

  .prompt-area {
    min-height: 140px;
    padding: 8px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
    outline: none;
    resize: vertical;
    box-sizing: border-box;
  }

  .prompt-area.short {
    min-height: 80px;
  }

  .prompt-area:focus {
    border-color: var(--accent-blue);
  }

  .empty {
    font-size: 12px;
    color: var(--text-secondary);
    font-style: italic;
    margin: 0;
    padding: 8px;
  }

  .error-text {
    font-size: 12px;
    color: var(--accent-red);
    background: var(--overlay-accent-red);
    padding: 6px 10px;
    border-radius: 6px;
    margin: 0;
    word-break: break-word;
  }

  .validation-text {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    align-items: center;
    margin-top: 4px;
  }

  .hint {
    font-size: 10px;
    color: var(--text-secondary);
    margin-right: auto;
  }
</style>
