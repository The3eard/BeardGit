<!--
  ReviewerPicker — simple text-input dialog for adding reviewers by
  username. Submits a comma- or whitespace-separated list; the caller
  computes the diff against the current reviewers (and excludes duplicates).

  User enumeration isn't universally available on gh/glab, so this picker
  accepts free-form usernames rather than showing a searchable list.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Reviewers currently assigned (used to dedupe submitted names). */
    current: string[];
    /** Called with the newly added reviewer names. */
    onApply: (added: string[]) => void;
    /** Called when the user dismisses the picker. */
    onCancel: () => void;
  }

  let { current, onApply, onCancel }: Props = $props();

  let input = $state("");

  function apply() {
    const requested = input
      .split(/[\s,]+/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    const currentSet = new Set(current);
    const added = requested.filter((n) => !currentSet.has(n));
    onApply(added);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      apply();
    }
  }
</script>

<button class="overlay" type="button" aria-label={m.mrpr_cancel()} onclick={onCancel}></button>
<!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
<div class="picker" role="dialog" tabindex="-1" onkeydown={handleKeydown}>
  <label class="picker-label" for="reviewer-input"
    >{m.mrpr_reviewer_picker_add_label()}</label
  >
  <input
    id="reviewer-input"
    class="picker-input"
    type="text"
    bind:value={input}
    placeholder={m.mrpr_reviewer_picker_placeholder()}
  />
  <p class="picker-hint">{m.mrpr_reviewer_picker_hint()}</p>
  <div class="picker-actions">
    <button type="button" class="btn-secondary" onclick={onCancel}
      >{m.mrpr_cancel()}</button
    >
    <button
      type="button"
      class="btn-primary"
      onclick={apply}
      disabled={!input.trim()}
    >
      {m.mrpr_reviewer_picker_apply()}
    </button>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 99;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .picker {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 100;
    width: 360px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  }
  .picker-label {
    display: block;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }
  .picker-input {
    width: 100%;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
    box-sizing: border-box;
  }
  .picker-hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 8px 0 12px;
  }
  .picker-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn-primary {
    padding: 5px 12px;
    background: var(--accent-blue);
    color: #fff;
    border: none;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn-secondary {
    padding: 5px 12px;
    background: none;
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
</style>
