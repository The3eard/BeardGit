<!--
  ReauthNoticeDialog.svelte — one-time-per-update apology surfaced right
  before the actual download begins on macOS (Gatekeeper) and Windows
  (SmartScreen).

  Until Apple / Microsoft developer-certificate signing lands (MT-2), every
  new BeardGit build is unsigned — which means the OS will re-prompt the
  user to re-authorize the relaunched app, exactly like the very first
  install. Rather than surprise the user mid-update, this dialog explains
  what's about to happen, apologises for the friction, and (optionally)
  lets them suppress future prompts via a "Don't show this again"
  checkbox. Linux never renders this dialog.

  The dialog is fully controlled: the parent owns `open` and passes
  `onConfirm` / `onCancel` callbacks. The persistence of the dismissal
  flag lives in the caller — this component just surfaces the checkbox
  state and hands it back via the `dismissForever` arg of `onConfirm`.

  Styling reuses the shared dialog primitives from
  `src/lib/styles/dialog.css`; MT-5's unified Dialog primitive will
  replace this inline markup once it lands.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";

  interface Props {
    /**
     * Whether the dialog is visible. Parent-controlled and bindable so
     * `$bindable` producers can also close the dialog imperatively.
     */
    open: boolean;
    /** Which OS variant of the apology body to render. */
    os: "macos" | "windows";
    /**
     * Called when the user clicks **Update now**. The `dismissForever`
     * argument reflects the current state of the "Don't show this
     * again" checkbox — persist it via `setReauthDismissed(os, true)`
     * when `true`.
     */
    onConfirm: (dismissForever: boolean) => void;
    /**
     * Called when the user clicks **Cancel** or presses **Escape**.
     * Should reset the update state machine to `idle`.
     */
    onCancel: () => void;
  }

  let { open = $bindable(false), os, onConfirm, onCancel }: Props = $props();

  let dismissForever = $state(false);

  function handleConfirm() {
    const value = dismissForever;
    // Reset the local checkbox each time the dialog closes so re-opening
    // it after a failed update starts from an unticked state.
    dismissForever = false;
    onConfirm(value);
  }

  function handleCancel() {
    dismissForever = false;
    onCancel();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      handleCancel();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="backdrop"
    data-testid="reauth-backdrop"
    onclick={handleCancel}
    onkeydown={handleKeydown}
    role="presentation"
  ></div>
  <div
    class="dialog reauth-dialog"
    role="dialog"
    aria-modal="true"
    aria-label={m.update_reauth_title()}
    data-testid="reauth-dialog"
    data-os={os}
  >
    <h3 class="dialog-title">{m.update_reauth_title()}</h3>

    <p class="reauth-body" data-testid="reauth-body">
      {#if os === "macos"}
        {m.update_reauth_body_macos()}
      {:else}
        {m.update_reauth_body_windows()}
      {/if}
    </p>

    <p class="reauth-sorry" data-testid="reauth-sorry">
      {m.update_reauth_sorry()}
    </p>

    <label class="reauth-dismiss">
      <input
        type="checkbox"
        data-testid="reauth-dismiss-checkbox"
        bind:checked={dismissForever}
      />
      <span>{m.update_reauth_dismiss_label()}</span>
    </label>

    <div class="dialog-actions">
      <Button
        variant="neutral"
        testid="reauth-cancel"
        onclick={handleCancel}
      >
        {m.update_reauth_cancel()}
      </Button>
      <Button
        variant="primary"
        testid="reauth-confirm"
        onclick={handleConfirm}
      >
        {m.update_reauth_confirm()}
      </Button>
    </div>
  </div>
{/if}

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions */

  .reauth-dialog {
    min-width: 420px;
    max-width: 520px;
  }

  .reauth-body {
    margin: 0 0 12px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-primary);
  }

  .reauth-sorry {
    margin: 0 0 16px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-secondary);
    font-style: italic;
  }

  .reauth-dismiss {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 16px;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
    user-select: none;
  }

  .reauth-dismiss input[type="checkbox"] {
    accent-color: var(--accent-primary);
    width: 14px;
    height: 14px;
    cursor: pointer;
  }
</style>
