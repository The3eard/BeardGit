<!--
  Dialog.svelte — shared modal dialog primitive for the MT-5 settings IA
  overhaul.

  Supersedes the per-dialog backdrop/header/footer boilerplate used by
  ConfirmDialog, CreateMrPrDialog, CreateWorktreeDialog, etc. MT-5 will
  migrate those call sites; this component ships the primitive.

  Behaviour (per the MT-5 plan):
  - `open` is bindable so parents can `bind:open` and flip it via Esc,
    backdrop click, or close-button clicks.
  - Backdrop click closes (fires `onClose` + sets `open = false`).
  - `Esc` closes.
  - Focus trap: once mounted, focus moves into the dialog and `Tab` /
    `Shift+Tab` cycles within focusable descendants. No external deps —
    the trap is implemented inline with `querySelectorAll` + a keydown
    handler.
  - Focus is restored to the previously focused element on close.

  ```svelte
  <Dialog bind:open title="Discard changes?" size="sm" onClose={cancel}>
    <p>Are you sure?</p>
    {#snippet footer()}<Button onclick={cancel}>Cancel</Button>{/snippet}
  </Dialog>
  ```
-->
<script lang="ts">
  import { tick } from "svelte";

  interface Props {
    /** Whether the dialog is visible. Two-way bindable via `bind:open`. */
    open: boolean;
    /** Title rendered in the dialog header + used as `aria-label`. */
    title: string;
    /** Width variant — `sm | md | lg`. Default `'md'`. */
    size?: "sm" | "md" | "lg";
    /**
     * Called when the dialog closes (Esc, backdrop click, or the parent
     * setting `open = false`). Parent is expected to persist side-effects.
     */
    onClose?: () => void;
    /** Body slot. */
    children?: import("svelte").Snippet;
    /** Footer (action buttons) slot. */
    footer?: import("svelte").Snippet;
  }

  let {
    open = $bindable(false),
    title,
    size = "md",
    onClose,
    children,
    footer,
  }: Props = $props();

  let dialogEl = $state<HTMLDivElement | null>(null);
  let previouslyFocused: HTMLElement | null = null;

  /** Query focusable descendants of the dialog for the Tab trap. */
  function focusable(root: HTMLElement): HTMLElement[] {
    const selector = [
      "a[href]",
      "button:not([disabled])",
      "input:not([disabled])",
      "select:not([disabled])",
      "textarea:not([disabled])",
      '[tabindex]:not([tabindex="-1"])',
    ].join(",");
    return Array.from(root.querySelectorAll<HTMLElement>(selector)).filter(
      (el) => !el.hasAttribute("data-bg-focus-skip"),
    );
  }

  function close() {
    open = false;
    onClose?.();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key === "Tab" && dialogEl) {
      const nodes = focusable(dialogEl);
      if (nodes.length === 0) {
        event.preventDefault();
        return;
      }
      const first = nodes[0];
      const last = nodes[nodes.length - 1];
      const active = document.activeElement as HTMLElement | null;
      if (event.shiftKey) {
        if (!active || active === first || !dialogEl.contains(active)) {
          event.preventDefault();
          last.focus();
        }
      } else {
        if (!active || active === last || !dialogEl.contains(active)) {
          event.preventDefault();
          first.focus();
        }
      }
    }
  }

  // Track open transitions to manage focus trap lifecycle.
  $effect(() => {
    if (open) {
      previouslyFocused = document.activeElement as HTMLElement | null;
      // Defer focus until the dialog is in the DOM.
      void tick().then(() => {
        if (!dialogEl) return;
        const nodes = focusable(dialogEl);
        if (nodes.length > 0) {
          nodes[0].focus();
        } else {
          dialogEl.focus();
        }
      });
    } else if (previouslyFocused) {
      previouslyFocused.focus?.();
      previouslyFocused = null;
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="bg-dialog-backdrop"
    role="presentation"
    data-testid="bg-dialog-backdrop"
    onclick={close}
  ></div>
  <div
    bind:this={dialogEl}
    class="bg-dialog bg-dialog--{size}"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    data-testid="bg-dialog"
  >
    <header class="bg-dialog__header">
      <h2 class="bg-dialog__title">{title}</h2>
    </header>
    <div class="bg-dialog__body">
      {#if children}{@render children()}{/if}
    </div>
    {#if footer}
      <footer class="bg-dialog__footer">{@render footer()}</footer>
    {/if}
  </div>
{/if}

<style>
  .bg-dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5); /* beardgit:allow-hex: modal backdrop neutral */
    z-index: 999;
  }

  .bg-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 10px;
    z-index: 1000;
    box-shadow: var(--shadow-modal);
    display: flex;
    flex-direction: column;
    max-height: 85vh;
    min-height: 120px;
    outline: none;
  }

  .bg-dialog--sm {
    width: min(420px, 92vw);
  }
  .bg-dialog--md {
    width: min(560px, 92vw);
  }
  .bg-dialog--lg {
    width: min(800px, 92vw);
  }

  .bg-dialog__header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .bg-dialog__title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .bg-dialog__body {
    padding: 16px 20px;
    overflow-y: auto;
    flex: 1;
    color: var(--text-primary);
    font-size: 13px;
    line-height: 1.5;
  }

  .bg-dialog__footer {
    padding: 12px 20px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-shrink: 0;
  }
</style>
