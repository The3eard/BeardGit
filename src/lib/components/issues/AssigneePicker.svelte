<!--
  AssigneePicker — popover dialog for editing issue assignees.

  User enumeration isn't universally available via gh/glab, so we accept
  comma-separated usernames. On Apply, we compute added/removed sets for
  the caller to feed directly into the store's add/remove assignee calls.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Usernames currently assigned. */
    current: string[];
    /** Fired with the added / removed usernames on Apply. */
    onApply: (added: string[], removed: string[]) => void;
    /** Fired when the user dismisses the dialog. */
    onCancel: () => void;
  }

  let { current, onApply, onCancel }: Props = $props();

  // svelte-ignore state_referenced_locally
  let input = $state(current.join(", "));

  function apply() {
    const parsed = input
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const currentSet = new Set(current);
    const parsedSet = new Set(parsed);
    const added = parsed.filter((a) => !currentSet.has(a));
    const removed = current.filter((a) => !parsedSet.has(a));
    onApply(added, removed);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter") apply();
  }
</script>

<button class="overlay" type="button" aria-label={m.issues_cancel()} onclick={onCancel}></button>
<!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
<div class="picker" role="dialog" tabindex="-1" onkeydown={handleKeydown}>
  <h3 class="picker-title">{m.issues_assignee_picker_title()}</h3>
  <input
    class="picker-input"
    type="text"
    bind:value={input}
    placeholder={m.issues_assignee_picker_placeholder()}
  />
  <p class="picker-hint">{m.issues_assignee_picker_hint()}</p>
  <div class="picker-actions">
    <button type="button" class="btn-secondary" onclick={onCancel}>{m.issues_cancel()}</button>
    <button type="button" class="btn-primary" onclick={apply}>{m.issues_label_picker_apply()}</button>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 99;
    border: none;
    cursor: pointer;
  }
  .picker {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 100;
    min-width: 340px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  }
  .picker-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .picker-input {
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
  }
  .picker-hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
  }
  .picker-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding-top: 6px;
    border-top: 1px solid var(--border);
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
