<!--
  NetworkSlot — surfaces offline detection via `navigator.onLine`.

  The slot is intentionally quiet: while the browser reports the device
  is online, *nothing* renders. The user's statusbar stays clean. On
  `offline` we surface a small amber/red pill so the user knows network
  operations (fetch / clone / AI network calls) are about to fail.

  `navigator.onLine` is famously optimistic — it only reflects a live
  network interface, not actual reachability to the server. That's OK:
  the drawer surfaces the real per-task error when something fails, and
  the slot's job is just "is there any possibility of internet at all".

  The slot is non-interactive per spec. Clicks do nothing.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import * as m from "$lib/paraglide/messages";

  // Default to online under SSR / test-time when `navigator` isn't the
  // real browser object — tests dispatch synthetic events to flip state.
  let online = $state(
    typeof navigator !== "undefined" && typeof navigator.onLine === "boolean"
      ? navigator.onLine
      : true,
  );

  function handleOnline() {
    online = true;
  }

  function handleOffline() {
    online = false;
  }

  onMount(() => {
    if (typeof window === "undefined") return;
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
  });

  onDestroy(() => {
    if (typeof window === "undefined") return;
    window.removeEventListener("online", handleOnline);
    window.removeEventListener("offline", handleOffline);
  });
</script>

{#if !online}
  <div class="network-slot offline" data-testid="statusbar-network-slot">
    <span class="dot" aria-hidden="true"></span>
    <span class="label">{m.statusbar_offline()}</span>
  </div>
{/if}

<style>
  .network-slot {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 100%;
    padding: 0 8px;
    font-size: 11px;
    user-select: none;
  }

  .network-slot.offline {
    color: var(--accent-orange);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    flex-shrink: 0;
  }

  .label {
    line-height: 1;
  }
</style>
