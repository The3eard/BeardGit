<!--
  AssigneePicker — popover dialog for editing issue assignees.

  User enumeration isn't universally available via gh/glab, so we accept
  comma-separated usernames. On Apply, we compute added/removed sets for
  the caller to feed directly into the store's add/remove assignee calls.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";

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

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="backdrop" onclick={onCancel} onkeydown={(e) => { if (e.key === "Escape") onCancel(); }} role="button" tabindex="-1"></div>
<!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
<div class="dialog" role="dialog" aria-modal="true" tabindex="-1" onkeydown={handleKeydown}>
  <h3 class="dialog-title">{m.issues_assignee_picker_title()}</h3>
  <input
    class="picker-input"
    type="text"
    bind:value={input}
    placeholder={m.issues_assignee_picker_placeholder()}
  />
  <p class="picker-hint">{m.issues_assignee_picker_hint()}</p>
  <div class="dialog-actions">
    <Button variant="neutral" onclick={onCancel}>{m.issues_cancel()}</Button>
    <Button variant="primary" onclick={apply}>{m.issues_label_picker_apply()}</Button>
  </div>
</div>

<style>
  .dialog {
    min-width: 340px;
    display: flex;
    flex-direction: column;
    gap: 8px;
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
  .dialog-actions {
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
</style>
