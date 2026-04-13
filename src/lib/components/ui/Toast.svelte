<script lang="ts">
  import { onMount } from "svelte";
  import { removeToast, type Toast } from "../../stores/toast";

  let { toast }: { toast: Toast } = $props();
  let visible = $state(false);

  onMount(() => {
    // Trigger slide-in on next frame
    requestAnimationFrame(() => { visible = true; });
    if (toast.duration !== null) {
      const timer = setTimeout(() => dismiss(), toast.duration!);
      return () => clearTimeout(timer);
    }
  });

  function dismiss() {
    visible = false;
    setTimeout(() => removeToast(toast.id), 200);
  }
</script>

<div class="toast toast--{toast.type}" class:toast--visible={visible}>
  <div class="toast__accent"></div>
  <span class="toast__message">{toast.message}</span>
  <div class="toast__actions">
    {#if toast.actions}
      {#each toast.actions as action}
        <button class="toast__btn" onclick={action.onclick}>{action.label}</button>
      {/each}
    {/if}
    {#if toast.dismissible}
      <button class="toast__close" onclick={dismiss} aria-label="Dismiss">&times;</button>
    {/if}
  </div>
</div>

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-primary);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    transform: translateX(120%);
    opacity: 0;
    transition: transform 0.2s ease, opacity 0.2s ease;
    max-width: 380px;
    min-width: 240px;
    overflow: hidden;
  }
  .toast--visible {
    transform: translateX(0);
    opacity: 1;
  }
  .toast__accent {
    width: 3px;
    align-self: stretch;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .toast--info .toast__accent { background: var(--accent-blue); }
  .toast--success .toast__accent { background: var(--accent-green, #3fb950); }
  .toast--warning .toast__accent { background: var(--accent-orange, #d29922); }
  .toast--error .toast__accent { background: var(--accent-red, #f85149); }
  .toast__message {
    flex: 1;
    line-height: 1.4;
  }
  .toast__actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .toast__btn {
    padding: 3px 10px;
    border-radius: 4px;
    background: var(--accent-blue);
    color: white;
    border: none;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
  }
  .toast__btn:hover { opacity: 0.85; }
  .toast__close {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 16px;
    cursor: pointer;
    padding: 0 2px;
    line-height: 1;
  }
  .toast__close:hover { color: var(--text-primary); }
</style>
