<!--
  ForgeDetailShell — centralises the four states every forge detail
  pane cycles through: loading, error (with retry), empty, and the
  loaded content snippet.

  Used by MrPrDetail and ReleaseDetail so we have one place to fix
  UX regressions like "infinite spinner" or "blank pane on empty
  payload". Callers pass their loaded detail markup via the default
  `content` snippet; this component owns the container, the loading
  skeleton, the error banner with a retry button, and the empty
  message. This guarantees consistent visual treatment across
  forges and prevents one-off spinners / error toasts from drifting
  out of sync.

  Props:
    loading      — true while the detail fetch is in flight.
    error        — non-null error message (string) flips into error
                   mode; retry shown iff `onRetry` is also set.
    isEmpty      — true renders `emptyMessage` (only when not
                   loading and no error).
    emptyMessage — user-facing empty-state copy (localised).
    emptyIcon    — optional Nerd Font glyph for the empty state.
    onRetry      — optional click handler for the error state's
                   Retry button.
    content      — default snippet rendered when not loading, no
                   error, and not empty.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";
  import EmptyState from "./EmptyState.svelte";

  interface Props {
    loading: boolean;
    error: string | null;
    isEmpty: boolean;
    emptyMessage: string;
    emptyIcon?: string;
    onRetry?: () => void;
    content?: import("svelte").Snippet;
  }

  let {
    loading,
    error,
    isEmpty,
    emptyMessage,
    emptyIcon,
    onRetry,
    content,
  }: Props = $props();

  /**
   * Truncate error text for the banner — full text still goes to
   * the toast that the caller fires in parallel, so we keep the
   * inline reason to a single line.
   */
  function trim(reason: string): string {
    return reason.length > 80 ? `${reason.slice(0, 77)}…` : reason;
  }
</script>

{#if loading}
  <div class="shell shell-loading" data-testid="forge-detail-loading">
    <div class="spinner" aria-hidden="true"></div>
    <span class="sr-only">{m.forge_detail_loading()}</span>
  </div>
{:else if error}
  <div class="shell shell-error" role="alert" data-testid="forge-detail-error">
    <p class="error-title">{m.forge_detail_error_title()}</p>
    <p class="error-reason">{trim(error)}</p>
    {#if onRetry}
      <Button
        variant="neutral"
        testid="forge-detail-retry"
        onclick={onRetry}>{m.forge_detail_retry()}</Button>
    {/if}
  </div>
{:else if isEmpty}
  <div class="shell shell-empty" data-testid="forge-detail-empty">
    <EmptyState title={emptyMessage} icon={emptyIcon} />
  </div>
{:else}
  {@render content?.()}
{/if}

<style>
  .shell {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 24px;
    gap: 8px;
  }
  .shell-empty,
  .shell-loading {
    color: var(--text-secondary);
    font-size: var(--font-size-md);
  }
  .shell-error {
    color: var(--accent-red);
    font-size: var(--font-size-md);
    max-width: 360px;
    text-align: center;
    margin: 0 auto;
  }
  .error-title {
    font-weight: 600;
    margin: 0;
  }
  .error-reason {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }
  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
