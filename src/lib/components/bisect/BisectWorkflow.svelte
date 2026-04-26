<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    bisectState,
    bisectLog,
    bisectLoading,
    refreshBisectState,
    startBisect,
    markGood,
    markBad,
    skipCommit,
    resetBisect,
    runAutoBisect,
    clearBisectState,
  } from "../../stores/bisect";
  import { addToast } from "../../stores/toast";
  import { IconButton } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";
  import AutoBisectDialog from "./AutoBisectDialog.svelte";

  let badCommit = $state("");
  let goodCommit = $state("");
  let lastResult = $state("");
  let showAutoDialog = $state(false);
  let logEl: HTMLPreElement | undefined = $state();

  onMount(() => {
    refreshBisectState().catch(() => {});
  });

  onDestroy(() => {
    clearBisectState();
  });

  async function handleStart() {
    try {
      lastResult = await startBisect(
        badCommit.trim() || undefined,
        goodCommit.trim() || undefined,
      );
      badCommit = "";
      goodCommit = "";
    } catch (e) {
      addToast({ message: String(e), type: "error" });
    }
  }

  async function handleGood() {
    try {
      lastResult = await markGood();
    } catch (e) {
      addToast({ message: String(e), type: "error" });
    }
  }

  async function handleBad() {
    try {
      lastResult = await markBad();
    } catch (e) {
      addToast({ message: String(e), type: "error" });
    }
  }

  async function handleSkip() {
    try {
      lastResult = await skipCommit();
    } catch (e) {
      addToast({ message: String(e), type: "error" });
    }
  }

  async function handleReset() {
    try {
      lastResult = "";
      await resetBisect();
    } catch (e) {
      addToast({ message: String(e), type: "error" });
    }
  }

  async function handleAutoRun(testCommand: string) {
    showAutoDialog = false;
    try {
      lastResult = await runAutoBisect(testCommand);
    } catch (e) {
      addToast({ message: String(e), type: "error" });
    }
  }

  // Auto-scroll log to bottom when updated
  $effect(() => {
    if ($bisectLog && logEl) {
      logEl.scrollTop = logEl.scrollHeight;
    }
  });
</script>

<div class="bisect-view" data-testid="bisect-view">
  <div class="list-header">
    <h2 class="view-title">{m.sidebar_bisect()}</h2>
    {#if $bisectState.active}
      <IconButton icon={"\uF00D"} description={m.bisect_reset()} testid="bisect-reset-btn" onclick={handleReset} />
    {/if}
  </div>

  <div class="bisect-content">
    {#if !$bisectState.active}
      <!-- Inactive: show start form -->
      <div class="inactive-panel">
        <div class="inactive-icon">{"\uF002"}</div>
        <p class="inactive-text">{m.bisect_inactive()}</p>
        <p class="inactive-hint">{m.bisect_inactive_hint()}</p>

        <div class="start-form">
          <div class="form-field">
            <label class="form-label" for="bisect-bad">{m.bisect_bad()}</label>
            <input
              id="bisect-bad"
              class="form-input"
              type="text"
              placeholder="HEAD"
              bind:value={badCommit}
              data-testid="bisect-bad-input"
            />
          </div>
          <div class="form-field">
            <label class="form-label" for="bisect-good">{m.bisect_good()}</label>
            <input
              id="bisect-good"
              class="form-input"
              type="text"
              placeholder="SHA / ref"
              bind:value={goodCommit}
              data-testid="bisect-good-input"
            />
          </div>
          <button class="btn btn-primary start-btn" data-testid="bisect-start-btn" onclick={handleStart}>
            {m.bisect_start()}
          </button>
        </div>
      </div>
    {:else}
      <!-- Active: show controls + state -->
      <div class="active-panel">
        <div class="current-commit-card">
          <span class="card-label">{m.bisect_current()}</span>
          <code class="card-oid">{$bisectState.current_commit ?? "..."}</code>
        </div>

        <div class="action-buttons">
          <button
            class="btn btn-good"
            data-testid="bisect-good-btn"
            onclick={handleGood}
            disabled={$bisectLoading}
          >
            <span class="label-icon">{"\uF00C"}</span>
            {m.bisect_good()}
          </button>
          <button
            class="btn btn-bad"
            data-testid="bisect-bad-btn"
            onclick={handleBad}
            disabled={$bisectLoading}
          >
            <span class="label-icon">{"\uF00D"}</span>
            {m.bisect_bad()}
          </button>
          <button
            class="btn btn-skip"
            onclick={handleSkip}
            disabled={$bisectLoading}
          >
            <span class="label-icon">{"\uF04E"}</span>
            {m.bisect_skip()}
          </button>
        </div>

        <div class="marks-summary">
          {#if $bisectState.good_commits.length > 0}
            <span class="mark-badge good">{m.bisect_good()}: {$bisectState.good_commits.length}</span>
          {/if}
          {#if $bisectState.bad_commits.length > 0}
            <span class="mark-badge bad">{m.bisect_bad()}: {$bisectState.bad_commits.length}</span>
          {/if}
        </div>

        <div class="auto-section">
          <button
            class="btn btn-auto"
            onclick={() => showAutoDialog = true}
            disabled={$bisectLoading}
          >
            <span class="label-icon">{"\uF04B"}</span>
            {m.bisect_auto()}
          </button>
          {#if $bisectLoading}
            <span class="loading-indicator">
              <span class="spinner"></span>
            </span>
          {/if}
        </div>

        {#if lastResult}
          <div class="result-card">
            <pre class="result-text">{lastResult}</pre>
          </div>
        {/if}
      </div>

      <!-- Log section -->
      {#if $bisectLog}
        <div class="log-section">
          <div class="log-header">{m.bisect_log()}</div>
          <pre class="log-content" bind:this={logEl}>{$bisectLog}</pre>
        </div>
      {/if}
    {/if}
  </div>
</div>

{#if showAutoDialog}
  <AutoBisectDialog
    onRun={handleAutoRun}
    onCancel={() => showAutoDialog = false}
  />
{/if}

<style>
  /* list.css provides: .list-header (global via app.css) */

  .bisect-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .view-title {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
    color: var(--text-primary);
  }

  .bisect-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  /* ── Inactive state ── */

  .inactive-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding-top: 32px;
  }

  .inactive-icon {
    font-family: var(--font-icons);
    font-size: 32px;
    color: var(--text-secondary);
    opacity: 0.4;
    margin-bottom: 12px;
  }

  .inactive-text {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .inactive-hint {
    margin: 4px 0 24px;
    font-size: 12px;
    color: var(--text-secondary);
    opacity: 0.7;
  }

  .start-form {
    width: 100%;
    max-width: 360px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .form-input {
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono, "Fira Code", monospace);
    outline: none;
  }

  .form-input:focus {
    border-color: var(--accent-blue);
  }

  .start-btn {
    align-self: center;
    margin-top: 4px;
  }

  /* ── Active state ── */

  .active-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .current-commit-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: var(--overlay-accent-blue);
    border: 1px solid color-mix(in srgb, var(--accent-blue) 20%, transparent);
    border-radius: 8px;
  }

  .card-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent-blue);
    flex-shrink: 0;
  }

  .card-oid {
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 13px;
    color: var(--text-primary);
  }

  .action-buttons {
    display: flex;
    gap: 8px;
  }

  .btn-good {
    background: color-mix(in srgb, var(--accent-green) 12%, transparent);
    color: var(--accent-green);
    border: 1px solid color-mix(in srgb, var(--accent-green) 30%, transparent);
  }

  .btn-good:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent-green) 20%, transparent);
  }

  .btn-bad {
    background: color-mix(in srgb, var(--accent-red) 12%, transparent);
    color: var(--accent-red);
    border: 1px solid color-mix(in srgb, var(--accent-red) 30%, transparent);
  }

  .btn-bad:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent-red) 20%, transparent);
  }

  .btn-skip {
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .btn-skip:hover:not(:disabled) {
    background: color-mix(in srgb, var(--text-primary) 10%, transparent);
  }

  .label-icon {
    font-family: var(--font-icons);
    font-size: 12px;
  }

  .marks-summary {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .mark-badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 10px;
  }

  .mark-badge.good {
    background: color-mix(in srgb, var(--accent-green) 12%, transparent);
    color: var(--accent-green);
  }

  .mark-badge.bad {
    background: color-mix(in srgb, var(--accent-red) 12%, transparent);
    color: var(--accent-red);
  }

  .auto-section {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-auto {
    background: color-mix(in srgb, var(--accent-purple) 12%, transparent);
    color: var(--accent-purple);
    border: 1px solid color-mix(in srgb, var(--accent-purple) 30%, transparent);
  }

  .btn-auto:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent-purple) 20%, transparent);
  }

  .loading-indicator {
    display: flex;
    align-items: center;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border);
    border-top-color: var(--accent-blue);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .result-card {
    padding: 10px 14px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow-x: auto;
  }

  .result-text {
    margin: 0;
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 12px;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* ── Log section ── */

  .log-section {
    margin-top: 16px;
    border-top: 1px solid var(--border);
    padding-top: 12px;
  }

  .log-header {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .log-content {
    margin: 0;
    padding: 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 11px;
    color: var(--text-secondary);
    max-height: 240px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
