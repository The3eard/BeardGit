<!--
  ReviewerPicker — simple text-input dialog for adding reviewers by
  username. Submits a comma- or whitespace-separated list; the caller
  computes the diff against the current reviewers (and excludes duplicates).

  User enumeration isn't universally available on gh/glab, so this picker
  accepts free-form usernames rather than showing a searchable list.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";

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

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="backdrop" onclick={onCancel} onkeydown={(e) => { if (e.key === "Escape") onCancel(); }} role="button" tabindex="-1"></div>
<!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
<div class="dialog" role="dialog" aria-modal="true" tabindex="-1" onkeydown={handleKeydown}>
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
  <div class="dialog-actions">
    <Button variant="neutral" onclick={onCancel}>{m.mrpr_cancel()}</Button>
    <Button
      variant="primary"
      onclick={apply}
      disabled={!input.trim()}
    >
      {m.mrpr_reviewer_picker_apply()}
    </Button>
  </div>
</div>

<style>
  .dialog {
    min-width: 340px;
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
</style>
