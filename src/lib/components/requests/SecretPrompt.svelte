<!--
  SecretPrompt.svelte — modal that asks the user for a secret value
  needed by the active environment (e.g. an API token referenced by a
  request but not yet stored in the OS keychain).

  Save invokes `requests_set_secret` and dispatches `saved` so the
  caller can re-run the request that needed the secret. Cancel
  dispatches `cancel`. The dispatcher API is preserved so existing
  call sites don't need to change when this prompt is wired in.

  Wraps the content in the shared `Dialog` primitive so the modal
  inherits the app-wide focus trap, Esc-to-close, backdrop click
  semantics, and consistent header/footer chrome — same pattern used
  by `NavigationGuardDialog` and the rest of the dialogs in the app.
-->
<script lang="ts">
  import { requestsSetSecret } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import { createEventDispatcher } from "svelte";
  import { Button, Dialog } from "$lib/components/ui";

  export let envName: string;
  export let secretName: string;

  const dispatch = createEventDispatcher();

  let open = true;
  let value = "";
  let busy = false;

  async function save() {
    busy = true;
    try {
      await runMutation({
        kind: "requests_set_secret",
        invoke: () => requestsSetSecret(envName, secretName, value),
        successToast: () => `Saved secret "${secretName}"`,
        failureToastPrefix: "Save secret failed",
      });
      dispatch("saved");
      open = false;
    } catch {
      // runMutation already surfaced the failure toast; keep the dialog open.
    } finally {
      busy = false;
    }
  }

  function cancel() {
    dispatch("cancel");
    open = false;
  }
</script>

<Dialog
  bind:open
  title="Add secret"
  size="sm"
  onClose={cancel}
>
  <p class="prompt">
    Env <code>{envName}</code> needs a value for
    <code>{secretName}</code>.
  </p>
  <input
    class="bg-input"
    type="password"
    placeholder="paste value"
    bind:value
  />

  {#snippet footer()}
    <Button variant="neutral" onclick={cancel}>Cancel</Button>
    <Button
      variant="primary"
      loading={busy}
      disabled={!value}
      onclick={save}
    >
      Save
    </Button>
  {/snippet}
</Dialog>

<style>
  .prompt {
    margin: 0 0 12px 0;
    font-size: var(--font-size-md);
    color: var(--text-primary);
    line-height: 1.5;
  }

  .prompt code {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    padding: 1px 4px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .bg-input {
    width: 100%;
    height: 30px;
    line-height: 28px;
    padding: 0 10px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    outline: none;
    box-sizing: border-box;
  }

  .bg-input:focus {
    border-color: var(--accent-primary);
  }
</style>
