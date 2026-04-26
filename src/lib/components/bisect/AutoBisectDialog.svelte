<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";

  interface Props {
    onRun: (testCommand: string) => void;
    onCancel: () => void;
  }

  let { onRun, onCancel }: Props = $props();
  let testCommand = $state("");

  function handleSubmit() {
    const cmd = testCommand.trim();
    if (cmd) {
      onRun(cmd);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && testCommand.trim()) {
      handleSubmit();
    } else if (e.key === "Escape") {
      onCancel();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onCancel} onkeydown={handleKeyDown}></div>
<div class="dialog" role="dialog" aria-labelledby="auto-bisect-title">
  <h3 id="auto-bisect-title" class="dialog-title">{m.bisect_auto()}</h3>

  <div class="field">
    <label class="field-label" for="test-cmd">{m.bisect_test_command()}</label>
    <input
      id="test-cmd"
      class="field-input"
      type="text"
      placeholder="make test"
      bind:value={testCommand}
      onkeydown={handleKeyDown}
    />
    <p class="field-hint">{m.bisect_test_command_hint()}</p>
  </div>

  <div class="dialog-actions">
    <Button variant="neutral" onclick={onCancel}>
      {m.confirm_cancel()}
    </Button>
    <Button
      variant="primary"
      onclick={handleSubmit}
      disabled={!testCommand.trim()}
    >
      {m.bisect_run()}
    </Button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions, .btn, .btn-cancel, .btn-primary (global via app.css) */

  .dialog {
    min-width: 380px;
    max-width: 480px;
  }

  .field {
    margin-bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .field-input {
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 13px;
    outline: none;
  }

  .field-input:focus {
    border-color: var(--accent-blue);
  }

  .field-hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
    opacity: 0.7;
  }
</style>
